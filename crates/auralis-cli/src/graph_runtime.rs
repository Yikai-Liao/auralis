use std::{
    collections::{BTreeMap, BTreeSet},
    path::{Path, PathBuf},
};

use auralis::EffectRegistry;

use crate::{CliError, PathRole, effect_tokens, ensure_wav_extension, executor, spec};

pub(super) fn run_graph_spec(spec: &Path, locked: bool) -> Result<(), CliError> {
    if locked {
        spec::verify_graph_lock(spec)?;
    }
    let checked = spec::check_graph_spec(spec)?;
    let spec_dir = spec.parent().unwrap_or_else(|| Path::new(""));
    let mut grouped_sinks = BTreeMap::<String, Vec<&spec::CheckedSink>>::new();
    for sink in &checked.sinks {
        grouped_sinks
            .entry(sink.input.clone())
            .or_default()
            .push(sink);
    }
    let mut render_cache = BTreeMap::<String, auralis::AudioBuffer>::new();

    for (input_port, sinks) in grouped_sinks {
        let rendered = render_graph_port_audio(&checked, spec_dir, &input_port, &mut render_cache)?;
        for sink in sinks {
            let output = resolve_spec_path(spec_dir, &sink.path);
            ensure_wav_extension(&output, PathRole::Output)?;
            auralis::AudioFile::from_audio_buffer(rendered.clone())
                .into_pipeline()
                .write_wav(&output)?;
            println!("wrote {} <- {}", sink.path.display(), sink.input);
        }
    }

    Ok(())
}

fn render_graph_port_audio(
    checked: &spec::CheckedGraphSpec,
    spec_dir: &Path,
    input: &str,
    render_cache: &mut BTreeMap<String, auralis::AudioBuffer>,
) -> Result<auralis::AudioBuffer, CliError> {
    if let Some(rendered) = render_cache.get(input) {
        return Ok(rendered.clone());
    }

    let mut visited = BTreeSet::new();
    let rendered = render_graph_input_audio(checked, spec_dir, input, render_cache, &mut visited)?;
    render_cache.insert(input.to_owned(), rendered.clone());
    Ok(rendered)
}

fn render_graph_input_audio(
    checked: &spec::CheckedGraphSpec,
    spec_dir: &Path,
    input: &str,
    render_cache: &mut BTreeMap<String, auralis::AudioBuffer>,
    visited: &mut BTreeSet<String>,
) -> Result<auralis::AudioBuffer, CliError> {
    let Some(input_id) = input.strip_suffix(".audio") else {
        return Err(CliError::UnsupportedGraphSink {
            sink: input.to_owned(),
            input: input.to_owned(),
        });
    };
    if !visited.insert(input_id.to_owned()) {
        return Err(CliError::UnsupportedGraphRunShape);
    }

    if let Some(source) = checked.sources.iter().find(|source| source.id == input_id) {
        let input = resolve_spec_path(spec_dir, &source.path);
        ensure_wav_extension(&input, PathRole::Input)?;
        return auralis::AudioFile::open_wav(&input)?
            .into_pipeline()
            .into_audio_buffer()
            .map_err(CliError::from);
    }

    if let Some(node) = checked.nodes.iter().find(|node| node.id == input_id) {
        let rendered = render_graph_node_audio(checked, spec_dir, node, render_cache)?;
        render_cache.insert(input.to_owned(), rendered.clone());
        return Ok(rendered);
    }

    let chain = checked
        .chains
        .iter()
        .find(|chain| chain.id == input_id)
        .ok_or_else(|| CliError::UnsupportedGraphSink {
            sink: input.to_owned(),
            input: input.to_owned(),
        })?;
    let upstream = render_graph_port_audio(checked, spec_dir, &chain.input, render_cache)?;
    executor::apply_effect_tokens_to_buffer(upstream, &chain.effect_tokens)
}

fn render_graph_node_audio(
    checked: &spec::CheckedGraphSpec,
    spec_dir: &Path,
    node: &spec::CheckedNode,
    render_cache: &mut BTreeMap<String, auralis::AudioBuffer>,
) -> Result<auralis::AudioBuffer, CliError> {
    match node.op.as_deref() {
        None if node.inputs.len() == 1 => {
            render_graph_port_audio(checked, spec_dir, &node.inputs[0], render_cache)
        }
        Some("mix.sum") => render_graph_mix_sum_node(checked, spec_dir, node, render_cache),
        Some(op) if node.inputs.len() == 1 => {
            let upstream =
                render_graph_port_audio(checked, spec_dir, &node.inputs[0], render_cache)?;
            let effect_tokens = node_effect_tokens(op, &node.params)?;
            executor::apply_effect_tokens_to_buffer(upstream, &effect_tokens)
        }
        Some(_) | None => Err(CliError::UnsupportedGraphNodeInputs {
            node: node.id.clone(),
            inputs: node.inputs.clone(),
        }),
    }
}

fn render_graph_mix_sum_node(
    checked: &spec::CheckedGraphSpec,
    spec_dir: &Path,
    node: &spec::CheckedNode,
    render_cache: &mut BTreeMap<String, auralis::AudioBuffer>,
) -> Result<auralis::AudioBuffer, CliError> {
    let mut inputs = Vec::with_capacity(node.inputs.len());
    for (index, input) in node.inputs.iter().enumerate() {
        let rendered = render_graph_port_audio(checked, spec_dir, input, render_cache)?;
        let rendered = if let Some(Some(gain)) = node.input_gains.get(index) {
            let token_refs = ["gain", gain.as_str()];
            executor::apply_effect_token_refs_to_buffer(rendered, &token_refs)?
        } else {
            rendered
        };
        inputs.push(rendered);
    }

    auralis::AudioFile::from_audio_buffers_mixed(&inputs)?
        .into_pipeline()
        .into_audio_buffer()
        .map_err(CliError::from)
}

pub(super) fn node_effect_tokens(
    op: &str,
    params: &BTreeMap<String, toml::Value>,
) -> Result<Vec<String>, CliError> {
    let tokens = effect_tokens::lower_graph_effect_tokens(op, params).map_err(|error| {
        CliError::UnsupportedGraphNodeOp {
            op: op.to_owned(),
            reason: format!("parameter `{}` must be a string or number", error.param()),
        }
    })?;

    if tokens == [op] {
        let descriptor =
            EffectRegistry::resolve(op).map_err(|_| CliError::UnsupportedGraphNodeOp {
                op: op.to_owned(),
                reason: "node op is not implemented by the current graph runner".to_owned(),
            })?;
        if !params.is_empty() {
            return Err(CliError::UnsupportedGraphNodeOp {
                op: op.to_owned(),
                reason: "named parameters are not implemented for this graph node op".to_owned(),
            });
        }
        return Ok(vec![descriptor.canonical_name().to_owned()]);
    }

    Ok(tokens)
}

fn resolve_spec_path(spec_dir: &Path, path: &Path) -> PathBuf {
    if path.is_absolute() {
        path.to_path_buf()
    } else {
        spec_dir.join(path)
    }
}

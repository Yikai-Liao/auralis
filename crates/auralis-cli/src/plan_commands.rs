use std::{collections::BTreeMap, path::PathBuf};

use crate::{
    CliError,
    command_args::{ConvertArgs, PipeArgs, RenderArgs},
    command_support::{effect_input_to_chain_tokens, plan_graph_spec},
    executor::{ConvertOptions, OutputGuard, validate_convert_options},
    graph_plan,
    plan_args::{PlanArgs, PlanCommand},
    plan_combine::plan_combine_surface_command,
    plan_recipes::plan_recipe_surface_command,
    spec,
};

pub(super) fn run_plan_command(args: PlanArgs) -> Result<(), CliError> {
    let PlanArgs {
        spec,
        command,
        target,
        json,
        locked,
    } = args;

    if let Some(command) = command {
        return plan_modern_command(command, spec.as_ref(), target.as_ref(), locked, json);
    }

    let spec = spec.ok_or(CliError::MissingPlanInput)?;
    plan_graph_spec(&spec, target.as_deref(), json, locked)
}

fn plan_modern_command(
    command: PlanCommand,
    spec: Option<&PathBuf>,
    target: Option<&String>,
    locked: bool,
    json: bool,
) -> Result<(), CliError> {
    match command {
        PlanCommand::Convert(convert) => {
            reject_graph_plan_options(spec, target, locked)?;
            plan_convert_command(convert, json)
        }
        PlanCommand::Render(render) => {
            reject_graph_plan_options(spec, target, locked)?;
            plan_render_command(render, json)
        }
        PlanCommand::Pipe(pipe) => {
            reject_graph_plan_options(spec, target, locked)?;
            plan_pipe_command(pipe, json)
        }
        combine @ (PlanCommand::Mix(_)
        | PlanCommand::Concat(_)
        | PlanCommand::MixPower(_)
        | PlanCommand::Merge(_)
        | PlanCommand::Multiply(_)) => {
            reject_graph_plan_options(spec, target, locked)?;
            plan_combine_surface_command(combine, json)
        }
        recipe => {
            reject_graph_plan_options(spec, target, locked)?;
            plan_recipe_surface_command(recipe, json)
        }
    }
}

fn plan_convert_command(convert: ConvertArgs, json: bool) -> Result<(), CliError> {
    let checked = checked_convert_spec(convert)?;
    graph_plan::print_checked_plan("command:convert", "convert", &checked, None, json)
}

fn plan_render_command(render: RenderArgs, json: bool) -> Result<(), CliError> {
    let checked = checked_render_spec(render)?;
    graph_plan::print_checked_plan("command:render", "render", &checked, None, json)
}

fn plan_pipe_command(pipe: PipeArgs, json: bool) -> Result<(), CliError> {
    let checked = checked_pipe_spec(pipe)?;
    graph_plan::print_checked_plan("command:pipe", "pipe", &checked, None, json)
}

fn reject_graph_plan_options(
    spec: Option<&PathBuf>,
    target: Option<&String>,
    locked: bool,
) -> Result<(), CliError> {
    if spec.is_some() || target.is_some() || locked {
        return Err(CliError::PlanCommandRejectsGraphOptions);
    }
    Ok(())
}

fn checked_convert_spec(convert: ConvertArgs) -> Result<spec::CheckedGraphSpec, CliError> {
    let ConvertArgs {
        input,
        output,
        backend,
        output_channels,
        no_auto_channels,
        output_sample_rate,
        no_auto_rate,
        guard,
        norm,
        container,
        sample,
    } = convert;
    validate_convert_options(
        &output,
        ConvertOptions {
            backend,
            output_channels,
            no_auto_channels,
            output_sample_rate,
            no_auto_rate,
            guard: OutputGuard::from(guard),
            norm,
            container,
            sample,
        },
    )?;

    let mut step_labels = Vec::new();
    append_render_policy_steps(
        &mut step_labels,
        output_channels,
        no_auto_channels,
        output_sample_rate,
        no_auto_rate,
        guard,
        norm,
        false,
        None,
        container,
        sample,
    );

    let (chains, sink_input, expanded_step_ids) = if step_labels.is_empty() {
        (Vec::new(), "input.audio".to_owned(), Vec::new())
    } else {
        let step_ids = step_labels
            .iter()
            .enumerate()
            .map(|(index, label)| format!("convert/{:02}-{}", index + 1, step_slug(label)))
            .collect::<Vec<_>>();
        let chain = spec::CheckedChain {
            id: "convert".to_owned(),
            input: "input.audio".to_owned(),
            step_ids: step_ids.clone(),
            step_labels,
            effect_tokens: Vec::new(),
        };
        (vec![chain], "convert.audio".to_owned(), step_ids)
    };

    Ok(spec::CheckedGraphSpec {
        name: Some("convert".to_owned()),
        source_count: 1,
        chain_count: chains.len(),
        node_count: 0,
        sink_count: 1,
        sources: vec![spec::CheckedSource {
            id: "input".to_owned(),
            path: input,
        }],
        chains,
        nodes: Vec::new(),
        sinks: vec![spec::CheckedSink {
            id: "output".to_owned(),
            input: sink_input,
            path: output,
        }],
        expanded_step_ids,
    })
}

fn checked_render_spec(render: RenderArgs) -> Result<spec::CheckedGraphSpec, CliError> {
    let RenderArgs {
        input,
        output,
        backend: _,
        combine,
        additional_inputs,
        output_channels,
        no_auto_channels,
        output_sample_rate,
        no_auto_rate,
        guard,
        norm,
        dither,
        dither_seed,
        container,
        sample,
        effects_file,
        fx,
        chain,
    } = render;
    let mut sources = vec![spec::CheckedSource {
        id: "input".to_owned(),
        path: input,
    }];
    for (index, path) in additional_inputs.into_iter().enumerate() {
        sources.push(spec::CheckedSource {
            id: format!("input{}", index + 2),
            path,
        });
    }

    let mut nodes = Vec::new();
    let upstream = if sources.len() > 1 {
        nodes.push(spec::CheckedNode {
            id: "combine".to_owned(),
            op: Some(format!("combine.{}", combine.as_name())),
            inputs: sources
                .iter()
                .map(|source| format!("{}.audio", source.id))
                .collect(),
            input_gains: vec![None; sources.len()],
            params: BTreeMap::new(),
        });
        "combine.audio".to_owned()
    } else {
        "input.audio".to_owned()
    };

    let step_labels = render_step_labels(effects_file.as_ref(), &fx, chain.as_deref())?;
    let mut step_labels = step_labels;
    append_render_policy_steps(
        &mut step_labels,
        output_channels,
        no_auto_channels,
        output_sample_rate,
        no_auto_rate,
        guard,
        norm,
        dither,
        dither_seed,
        container,
        sample,
    );

    let (chains, sink_input, expanded_step_ids) = if step_labels.is_empty() {
        (Vec::new(), upstream, Vec::new())
    } else {
        let step_ids = step_labels
            .iter()
            .enumerate()
            .map(|(index, label)| format!("render/{:02}-{}", index + 1, step_slug(label)))
            .collect::<Vec<_>>();
        let chain = spec::CheckedChain {
            id: "render".to_owned(),
            input: upstream,
            step_ids: step_ids.clone(),
            step_labels,
            effect_tokens: Vec::new(),
        };
        (vec![chain], "render.audio".to_owned(), step_ids)
    };

    Ok(spec::CheckedGraphSpec {
        name: Some("render".to_owned()),
        source_count: sources.len(),
        chain_count: chains.len(),
        node_count: nodes.len(),
        sink_count: 1,
        sources,
        chains,
        nodes,
        sinks: vec![spec::CheckedSink {
            id: "output".to_owned(),
            input: sink_input,
            path: output,
        }],
        expanded_step_ids,
    })
}

fn checked_pipe_spec(pipe: PipeArgs) -> Result<spec::CheckedGraphSpec, CliError> {
    let PipeArgs {
        input,
        expression,
        output,
        backend: _,
    } = pipe;
    let effect_chain = parse_effect_input(&[], Some(expression.as_str()))?;
    let step_labels = effect_chain_step_labels(&effect_chain);
    let step_ids = step_labels
        .iter()
        .enumerate()
        .map(|(index, label)| format!("pipe/{:02}-{}", index + 1, step_slug(label)))
        .collect::<Vec<_>>();

    Ok(spec::CheckedGraphSpec {
        name: Some("pipe".to_owned()),
        source_count: 1,
        chain_count: 1,
        node_count: 0,
        sink_count: 1,
        sources: vec![spec::CheckedSource {
            id: "input".to_owned(),
            path: input,
        }],
        chains: vec![spec::CheckedChain {
            id: "pipe".to_owned(),
            input: "input.audio".to_owned(),
            step_ids: step_ids.clone(),
            step_labels,
            effect_tokens: Vec::new(),
        }],
        nodes: Vec::new(),
        sinks: vec![spec::CheckedSink {
            id: "output".to_owned(),
            input: "pipe.audio".to_owned(),
            path: output,
        }],
        expanded_step_ids: step_ids,
    })
}

fn render_step_labels(
    effects_file: Option<&PathBuf>,
    fx: &[String],
    chain: Option<&str>,
) -> Result<Vec<String>, CliError> {
    let effect_chain = match (effects_file, !fx.is_empty(), chain) {
        (Some(path), false, None) => auralis::parse_effects_file(path).map_err(CliError::from),
        (None, true, None) => parse_effect_input(fx, None),
        (None, false, Some(chain)) => parse_effect_input(&[], Some(chain)),
        (None, false, None) => Ok(auralis::EffectChain::empty()),
        _ => Err(CliError::MixedEffectInputs),
    }?;
    Ok(effect_chain_step_labels(&effect_chain))
}

fn parse_effect_input(
    fx: &[String],
    chain: Option<&str>,
) -> Result<auralis::EffectChain, CliError> {
    let tokens = effect_input_to_chain_tokens(fx, chain)?;
    let token_refs = tokens.iter().map(String::as_str).collect::<Vec<_>>();
    auralis::parse_effect_chain(&token_refs).map_err(CliError::from)
}

fn effect_chain_step_labels(chain: &auralis::EffectChain) -> Vec<String> {
    chain
        .commands()
        .iter()
        .map(|command| command.render_tokens().join(" "))
        .collect()
}

#[allow(clippy::fn_params_excessive_bools, clippy::too_many_arguments)]
fn append_render_policy_steps(
    step_labels: &mut Vec<String>,
    output_channels: Option<auralis::ChannelCount>,
    no_auto_channels: bool,
    output_sample_rate: Option<auralis::SampleRate>,
    no_auto_rate: bool,
    guard: bool,
    norm: Option<f64>,
    dither: bool,
    dither_seed: Option<u32>,
    container: Option<crate::executor::OutputContainer>,
    sample: Option<auralis::WavSampleFormat>,
) {
    if let Some(channels) = output_channels {
        let suffix = if no_auto_channels { " no-auto" } else { "" };
        step_labels.push(format!("channels {}{}", channels.as_u16(), suffix));
    }
    if let Some(rate) = output_sample_rate {
        let suffix = if no_auto_rate { " no-auto" } else { "" };
        step_labels.push(format!("rate {}{}", rate.as_u32(), suffix));
    }
    if guard {
        step_labels.push("guard".to_owned());
    }
    if let Some(norm) = norm {
        step_labels.push(format!("norm {norm}"));
    }
    if dither {
        step_labels.push(match dither_seed {
            Some(seed) => format!("dither seed={seed}"),
            None => "dither".to_owned(),
        });
    }
    if let Some(container) = container {
        step_labels.push(format!("container {container:?}").to_lowercase());
    }
    if let Some(sample) = sample {
        step_labels.push(format!("sample {sample:?}").to_lowercase());
    }
}

pub(super) fn step_slug(label: &str) -> String {
    label
        .chars()
        .filter_map(|value| {
            if value.is_ascii_alphanumeric() {
                Some(value.to_ascii_lowercase())
            } else if value.is_ascii_whitespace() || value == '-' || value == '_' {
                Some('-')
            } else {
                None
            }
        })
        .take(32)
        .collect()
}

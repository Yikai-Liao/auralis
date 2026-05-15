use std::{ffi::OsStr, path::Path};

use auralis::{EffectRegistry, SUPPORTED_EFFECTS};
use auralis_wav::decode_pcm16_path;
use clap::ValueEnum;
use serde::Serialize;

use crate::{CliError, graph_plan, graph_runtime, spec};

#[derive(Debug, Clone, Copy, ValueEnum)]
pub(super) enum OpsSchemaFormat {
    Json,
}

#[derive(Debug, Clone, Copy)]
pub(super) enum PathRole {
    Input,
    Output,
}

impl std::fmt::Display for PathRole {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Input => formatter.write_str("input"),
            Self::Output => formatter.write_str("output"),
        }
    }
}

#[derive(Debug, Serialize)]
struct JsonInspectOutput {
    format: &'static str,
    sample_rate: u32,
    channels: u16,
    sample_format: &'static str,
    duration_frames: u64,
    duration_seconds: String,
}

pub(super) fn inspect(input: &Path, json: bool) -> Result<(), CliError> {
    ensure_wav_extension(input, PathRole::Input)?;
    let audio = decode_pcm16_path(input)?;
    let sample_rate = audio.spec().sample_rate().as_u32();
    let frames = audio.frames().as_u64();
    let duration_seconds = format_duration_seconds(frames, sample_rate);

    if json {
        let output = JsonInspectOutput {
            format: "wav",
            sample_rate,
            channels: audio.channels().as_u16(),
            sample_format: "pcm16",
            duration_frames: frames,
            duration_seconds,
        };
        println!("{}", serde_json::to_string_pretty(&output)?);
        return Ok(());
    }

    println!("format: wav");
    println!("sample_rate: {sample_rate}");
    println!("channels: {}", audio.channels().as_u16());
    println!("sample_format: pcm16");
    println!("duration_frames: {frames}");
    println!("duration_seconds: {duration_seconds}");

    Ok(())
}

pub(super) fn check_command(
    spec: Option<&Path>,
    locked: bool,
    effects_file: Option<&Path>,
    fx: &[String],
    chain: Option<&str>,
) -> Result<(), CliError> {
    if let Some(spec) = spec {
        if effects_file.is_some() || !fx.is_empty() || chain.is_some() {
            return Err(CliError::MixedCheckInputs);
        }
        return check_graph_spec(spec, locked);
    }

    if locked {
        return Err(CliError::LockedRequiresSpec);
    }
    check_effects(effects_file, fx, chain)
}

pub(super) fn plan_graph_spec(spec: &Path, json: bool, locked: bool) -> Result<(), CliError> {
    graph_plan::plan_graph_spec(spec, json, locked)
}

pub(super) fn run_graph_spec(spec: &Path, locked: bool) -> Result<(), CliError> {
    graph_runtime::run_graph_spec(spec, locked)
}

pub(super) fn print_ops(
    effect: Option<&str>,
    schema: Option<OpsSchemaFormat>,
) -> Result<(), CliError> {
    if matches!(schema, Some(OpsSchemaFormat::Json)) {
        return print_ops_json(effect);
    }

    if let Some(name) = effect {
        return print_one_op(name);
    }

    for descriptor in SUPPORTED_EFFECTS {
        println!(
            "{:<12} {}",
            descriptor.canonical_name(),
            descriptor.summary()
        );
    }

    Ok(())
}

pub(super) fn effect_input_to_chain_tokens(
    fx: &[String],
    chain: Option<&str>,
) -> Result<Vec<String>, CliError> {
    match (!fx.is_empty(), chain) {
        (true, Some(_)) => Err(CliError::MixedEffectInputs),
        (true, None) => effect_specs_to_chain_tokens(fx),
        (false, Some(chain)) => chain_to_effect_specs(chain),
        (false, None) => Ok(Vec::new()),
    }
}

pub(super) fn ensure_wav_extension(path: &Path, role: PathRole) -> Result<(), CliError> {
    if path.extension().and_then(OsStr::to_str) == Some("wav") {
        Ok(())
    } else {
        Err(CliError::UnsupportedFormat {
            path: path.to_path_buf(),
            role,
        })
    }
}

fn check_graph_spec(spec: &Path, locked: bool) -> Result<(), CliError> {
    if locked {
        spec::verify_graph_lock(spec)?;
    } else {
        spec::sync_graph_lock(spec)?;
    }
    let checked = spec::check_graph_spec(spec)?;

    println!("status: ok");
    println!("sources: {}", checked.source_count);
    println!("chains: {}", checked.chain_count);
    println!("nodes: {}", checked.node_count);
    println!("sinks: {}", checked.sink_count);
    println!("expanded_steps: {}", checked.expanded_step_ids.len());

    Ok(())
}

fn check_effects(
    effects_file: Option<&Path>,
    fx: &[String],
    chain: Option<&str>,
) -> Result<(), CliError> {
    let effect_chain = parse_effect_spec(effects_file, fx, chain)?;

    println!("status: ok");
    println!("commands: {}", effect_chain.len());
    println!("boundaries: {}", effect_chain.boundaries().len());

    Ok(())
}

#[derive(Debug, Serialize)]
struct JsonOpDescriptor {
    name: String,
    kind: String,
    summary: String,
    typed_api: String,
    sox_ng_syntax: String,
    aliases: Vec<String>,
}

fn print_ops_json(effect: Option<&str>) -> Result<(), CliError> {
    if let Some(name) = effect {
        let descriptor = EffectRegistry::resolve(name).map_err(CliError::from)?;
        println!(
            "{}",
            serde_json::to_string_pretty(&json_op_descriptor(*descriptor))?
        );
        return Ok(());
    }

    let descriptors = SUPPORTED_EFFECTS
        .iter()
        .copied()
        .map(json_op_descriptor)
        .collect::<Vec<_>>();
    println!("{}", serde_json::to_string_pretty(&descriptors)?);
    Ok(())
}

fn json_op_descriptor(descriptor: auralis::EffectDescriptor) -> JsonOpDescriptor {
    JsonOpDescriptor {
        name: descriptor.canonical_name().to_owned(),
        kind: format!("{:?}", descriptor.kind()),
        summary: descriptor.summary().to_owned(),
        typed_api: descriptor.typed_api().to_owned(),
        sox_ng_syntax: descriptor.sox_ng_syntax().to_owned(),
        aliases: descriptor
            .aliases()
            .iter()
            .map(|alias| (*alias).to_owned())
            .collect(),
    }
}

fn print_one_op(name: &str) -> Result<(), CliError> {
    let descriptor = EffectRegistry::resolve(name).map_err(CliError::from)?;

    println!("name: {}", descriptor.canonical_name());
    println!("kind: {:?}", descriptor.kind());
    println!("summary: {}", descriptor.summary());
    println!("typed_api: {}", descriptor.typed_api());
    println!("sox_ng_syntax: {}", descriptor.sox_ng_syntax());
    if descriptor.aliases().is_empty() {
        println!("aliases: none");
    } else {
        println!("aliases: {}", descriptor.aliases().join(", "));
    }

    Ok(())
}

fn parse_effect_spec(
    effects_file: Option<&Path>,
    fx: &[String],
    chain: Option<&str>,
) -> Result<auralis::EffectChain, CliError> {
    let has_effects_file = effects_file.is_some();
    let has_fx = !fx.is_empty();
    let has_chain = chain.is_some();

    match (has_effects_file, has_fx, has_chain) {
        (true, false, false) => {
            auralis::parse_effects_file(effects_file.unwrap()).map_err(CliError::from)
        }
        (false, true, false) | (false, false, true) => {
            let tokens = effect_input_to_chain_tokens(fx, chain)?;
            let token_refs: Vec<&str> = tokens.iter().map(String::as_str).collect();
            auralis::parse_effect_chain(&token_refs).map_err(CliError::from)
        }
        (false, false, false) => Err(CliError::MissingEffectSpec),
        _ => Err(CliError::MixedEffectInputs),
    }
}

fn chain_to_effect_specs(chain: &str) -> Result<Vec<String>, CliError> {
    let mut specs = Vec::new();

    for spec in chain.split('|').map(str::trim) {
        if spec.is_empty() {
            return Err(CliError::EmptyEffectSpec);
        }
        specs.push(spec.to_owned());
    }

    effect_specs_to_chain_tokens(&specs)
}

fn effect_specs_to_chain_tokens(specs: &[String]) -> Result<Vec<String>, CliError> {
    let mut tokens = Vec::new();

    for spec in specs {
        let parsed =
            shlex::split(spec).ok_or_else(|| CliError::InvalidEffectSpec { spec: spec.clone() })?;
        if parsed.is_empty() {
            return Err(CliError::EmptyEffectSpec);
        }
        tokens.extend(parsed);
    }

    Ok(tokens)
}

fn format_duration_seconds(frames: u64, sample_rate: u32) -> String {
    let sample_rate = u64::from(sample_rate);
    let whole = frames / sample_rate;
    let remainder = frames % sample_rate;
    let fractional = remainder * 1_000_000_000 / sample_rate;

    format!("{whole}.{fractional:09}")
}

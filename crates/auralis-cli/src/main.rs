//! Auralis command-line entrypoint.

mod spec;

use std::{
    ffi::OsStr,
    fs,
    path::{Path, PathBuf},
    process::ExitCode,
};

use auralis::{EffectRegistry, SUPPORTED_EFFECTS};
use auralis_wav::{WavError, decode_pcm16_path};
use clap::{Parser, Subcommand, ValueEnum};
use serde::Serialize;

#[derive(Debug, Parser)]
#[command(version, about = "Deterministic audio DSP tools.")]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Debug, Subcommand)]
#[allow(clippy::large_enum_variant)]
enum Command {
    /// Print metadata for a supported audio file.
    Inspect {
        /// PCM16 WAV input file to inspect.
        input: PathBuf,

        /// Emit machine-readable JSON output.
        #[arg(long)]
        json: bool,
    },

    /// Convert one supported audio file into another container format.
    Convert {
        /// Input audio file to read.
        input: PathBuf,

        /// Output audio file to create.
        #[arg(short = 'o', long = "output", value_name = "FILE")]
        output: PathBuf,

        /// Sample-processing backend to request.
        #[arg(long, value_name = "BACKEND", default_value = "scalar", value_parser = parse_backend)]
        backend: auralis::BackendKind,

        /// Output channel count; inserts SoX-ng-style channel conversion if needed.
        #[arg(short = 'c', long = "channels", value_name = "CHANNELS", value_parser = parse_channel_count)]
        output_channels: Option<auralis::ChannelCount>,

        /// Fail instead of automatically converting channels for --channels.
        #[arg(long)]
        no_auto_channels: bool,

        /// Output sample rate; inserts deterministic rate conversion if needed.
        #[arg(short = 'r', long = "rate", value_name = "RATE", value_parser = parse_sample_rate)]
        output_sample_rate: Option<auralis::SampleRate>,

        /// Fail instead of automatically converting sample rate for --rate.
        #[arg(long)]
        no_auto_rate: bool,

        /// Attenuate final output only if it would clip.
        #[arg(short = 'G', long)]
        guard: bool,

        /// Normalize final output to a peak level in dBFS, defaulting to 0 dBFS.
        #[arg(long, value_name = "DB", num_args = 0..=1, default_missing_value = "0", allow_hyphen_values = true)]
        norm: Option<f64>,

        /// Select WAV sample encoding when the output container is WAV.
        #[arg(long, value_name = "FORMAT", value_parser = parse_wav_sample_format)]
        sample: Option<auralis::WavSampleFormat>,
    },

    /// Keep one range from an audio file.
    Trim {
        /// PCM16 WAV input file to read.
        input: PathBuf,

        /// Frame range to keep, for example `10..30` or `10..`.
        range: String,

        /// Output WAV file to create.
        #[arg(short = 'o', long = "output", value_name = "FILE")]
        output: PathBuf,

        /// Sample-processing backend to request.
        #[arg(long, value_name = "BACKEND", default_value = "scalar", value_parser = parse_backend)]
        backend: auralis::BackendKind,
    },

    /// Normalize one audio file to a peak level.
    Normalize {
        /// Input audio file to read.
        input: PathBuf,

        /// Output audio file to create.
        #[arg(short = 'o', long = "output", value_name = "FILE")]
        output: PathBuf,

        /// Peak target in dBFS, defaulting to 0 dBFS.
        #[arg(long, value_name = "DBFS", default_value = "0", allow_hyphen_values = true, value_parser = parse_dbfs)]
        peak: f64,

        /// Sample-processing backend to request.
        #[arg(long, value_name = "BACKEND", default_value = "scalar", value_parser = parse_backend)]
        backend: auralis::BackendKind,
    },

    /// Reverse one audio file.
    Reverse {
        /// PCM16 WAV input file to read.
        input: PathBuf,

        /// Output WAV file to create.
        #[arg(short = 'o', long = "output", value_name = "FILE")]
        output: PathBuf,

        /// Sample-processing backend to request.
        #[arg(long, value_name = "BACKEND", default_value = "scalar", value_parser = parse_backend)]
        backend: auralis::BackendKind,
    },

    /// Render one ordered stream with typed effect syntax.
    Render {
        /// PCM16 WAV input file to read.
        input: PathBuf,

        /// Output WAV file to create.
        #[arg(short = 'o', long = "output", value_name = "FILE")]
        output: PathBuf,

        /// Sample-processing backend to request.
        #[arg(long, value_name = "BACKEND", default_value = "scalar", value_parser = parse_backend)]
        backend: auralis::BackendKind,

        /// Input-combiner method to apply before effects.
        #[arg(long, value_name = "METHOD", default_value = "concatenate", value_parser = parse_combine_method)]
        combine: auralis::CombineMethod,

        /// Additional PCM16 WAV input files to combine after the first input.
        #[arg(long = "input", value_name = "FILE")]
        additional_inputs: Vec<PathBuf>,

        /// Output channel count; inserts SoX-ng-style channel conversion if needed.
        #[arg(short = 'c', long = "channels", value_name = "CHANNELS", value_parser = parse_channel_count)]
        output_channels: Option<auralis::ChannelCount>,

        /// Fail instead of automatically converting channels for --channels.
        #[arg(long)]
        no_auto_channels: bool,

        /// Output sample rate; inserts deterministic rate conversion if needed.
        #[arg(short = 'r', long = "rate", value_name = "RATE", value_parser = parse_sample_rate)]
        output_sample_rate: Option<auralis::SampleRate>,

        /// Fail instead of automatically converting sample rate for --rate.
        #[arg(long)]
        no_auto_rate: bool,

        /// Attenuate final output only if it would clip.
        #[arg(short = 'G', long)]
        guard: bool,

        /// Normalize final output to a peak level in dBFS, defaulting to 0 dBFS.
        #[arg(long, value_name = "DB", num_args = 0..=1, default_missing_value = "0", allow_hyphen_values = true)]
        norm: Option<f64>,

        /// Apply deterministic TPDF dither before PCM16 encoding.
        #[arg(long)]
        dither: bool,

        /// Deterministic seed used when --dither is enabled.
        #[arg(long, value_name = "SEED")]
        dither_seed: Option<u32>,

        /// Read the effect chain from a SoX-ng-style effects file.
        #[arg(long, value_name = "FILE")]
        effects_file: Option<PathBuf>,

        /// One typed effect command per flag, for example `--fx 'gain -3'`.
        #[arg(long = "fx", value_name = "EFFECT")]
        fx: Vec<String>,

        /// Compact ordered effect chain, for example `--chain 'gain -3 | reverse'`.
        #[arg(long = "chain", value_name = "CHAIN")]
        chain: Option<String>,
    },

    /// Validate typed effect syntax without running audio processing.
    Check {
        /// Auralis graph spec to validate.
        spec: Option<PathBuf>,

        /// Require an up-to-date Auralis.lock instead of refreshing it.
        #[arg(long)]
        locked: bool,

        /// Read the effect chain from a SoX-ng-style effects file.
        #[arg(long, value_name = "FILE")]
        effects_file: Option<PathBuf>,

        /// One typed effect command per flag, for example `--fx 'gain -3'`.
        #[arg(long = "fx", value_name = "EFFECT")]
        fx: Vec<String>,

        /// Compact ordered effect chain, for example `--chain 'gain -3 | reverse'`.
        #[arg(long = "chain", value_name = "CHAIN")]
        chain: Option<String>,
    },

    /// Preview the execution shape for an Auralis graph spec.
    Plan {
        /// Auralis graph spec to plan.
        spec: PathBuf,

        /// Emit machine-readable JSON output.
        #[arg(long)]
        json: bool,

        /// Require an up-to-date Auralis.lock before planning.
        #[arg(long)]
        locked: bool,
    },

    /// Emit an Auralis graph spec as a graph description.
    Graph {
        /// Auralis graph spec to render.
        spec: PathBuf,

        /// Output graph format.
        #[arg(long, value_name = "FORMAT", default_value = "mermaid")]
        format: GraphFormat,
    },

    /// Format an Auralis graph spec.
    Fmt {
        /// Auralis graph spec to format.
        spec: PathBuf,

        /// Check whether formatting changes would be required.
        #[arg(long)]
        check: bool,
    },

    /// Generate shell completion scripts.
    Completions {
        /// Shell to generate completions for.
        shell: CompletionShell,
    },

    /// Print built-in manual pages for Auralis commands.
    Man {
        /// Optional command topic, for example `render` or `plan`.
        topic: Option<String>,
    },

    /// Explain how one graph node or target participates in execution.
    Explain {
        /// Auralis graph spec to inspect.
        spec: PathBuf,

        /// Chain, node, sink, or source id to explain.
        target: String,
    },

    /// Run an Auralis graph spec.
    Run {
        /// Auralis graph spec to execute.
        spec: PathBuf,

        /// Require an up-to-date Auralis.lock before running.
        #[arg(long)]
        locked: bool,
    },

    /// List implemented typed effects or inspect one effect descriptor.
    Ops {
        /// Optional canonical effect name or alias to inspect.
        effect: Option<String>,

        /// Emit machine-readable schema output.
        #[arg(long, value_name = "FORMAT")]
        schema: Option<OpsSchemaFormat>,
    },
}

#[derive(Debug, Clone, Copy, ValueEnum)]
enum GraphFormat {
    Mermaid,
    Dot,
    Svg,
    Json,
}

#[derive(Debug, Clone, Copy, ValueEnum)]
enum OpsSchemaFormat {
    Json,
}

#[derive(Debug, Clone, Copy, ValueEnum)]
enum CompletionShell {
    Bash,
    Zsh,
    Fish,
}

fn main() -> ExitCode {
    match run(Cli::parse()) {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("error: {error}");
            ExitCode::FAILURE
        }
    }
}

#[allow(clippy::too_many_lines)]
fn run(cli: Cli) -> Result<(), CliError> {
    match cli.command {
        Command::Inspect { input, json } => inspect(&input, json),
        Command::Convert {
            input,
            output,
            backend,
            output_channels,
            no_auto_channels,
            output_sample_rate,
            no_auto_rate,
            guard,
            norm,
            sample,
        } => convert_audio(
            &input,
            &output,
            ConvertOptions {
                backend,
                output_channels,
                no_auto_channels,
                output_sample_rate,
                no_auto_rate,
                guard: OutputGuard::from(guard),
                norm,
                sample,
            },
        ),
        Command::Trim {
            input,
            range,
            output,
            backend,
        } => run_trim_recipe(&input, &range, &output, backend),
        Command::Normalize {
            input,
            output,
            peak,
            backend,
        } => normalize_audio(&input, &output, peak, backend),
        Command::Reverse {
            input,
            output,
            backend,
        } => run_effect_recipe(&input, &output, backend, ["reverse"]),
        Command::Render {
            input,
            output,
            backend,
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
            effects_file,
            fx,
            chain,
        } => {
            if effects_file.is_some() && (!fx.is_empty() || chain.is_some()) {
                return Err(CliError::MixedEffectInputs);
            }
            let options = RenderOptions {
                backend,
                combine,
                additional_inputs,
                output_channels,
                no_auto_channels,
                output_sample_rate,
                no_auto_rate,
                guard: OutputGuard::from(guard),
                norm,
                dither: OutputDither::from(dither),
                dither_seed,
                effects_file,
                effect_chain: effect_input_to_chain_tokens(&fx, chain.as_deref())?,
            };

            run_pipeline(&input, &output, &options)
        }
        Command::Check {
            spec,
            locked,
            effects_file,
            fx,
            chain,
        } => check_command(
            spec.as_deref(),
            locked,
            effects_file.as_deref(),
            &fx,
            chain.as_deref(),
        ),
        Command::Plan { spec, json, locked } => plan_graph_spec(&spec, json, locked),
        Command::Graph { spec, format } => graph_spec(&spec, format),
        Command::Fmt { spec, check } => format_graph_spec(&spec, check),
        Command::Completions { shell } => print_completions(shell),
        Command::Man { topic } => print_man_page(topic.as_deref()),
        Command::Explain { spec, target } => explain_graph_target(&spec, &target),
        Command::Run { spec, locked } => run_graph_spec(&spec, locked),
        Command::Ops { effect, schema } => print_ops(effect.as_deref(), schema),
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

fn inspect(input: &Path, json: bool) -> Result<(), CliError> {
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

fn convert_audio(input: &Path, output: &Path, options: ConvertOptions) -> Result<(), CliError> {
    let audio = open_audio_file(input, options.backend)?;
    let format = output_format_from_path(output, options.sample)?;

    audio
        .into_pipeline()
        .with_backend(options.backend)
        .with_sample_rate_conversion_policy(options.sample_rate_conversion_policy()?)
        .with_channel_conversion_policy(options.channel_conversion_policy()?)
        .with_output_level_policy(options.output_level_policy()?)
        .write(output, format)?;

    Ok(())
}

fn run_trim_recipe(
    input: &Path,
    range: &str,
    output: &Path,
    backend: auralis::BackendKind,
) -> Result<(), CliError> {
    run_effect_recipe(input, output, backend, ["trim", range])
}

fn run_effect_recipe<'a>(
    input: &Path,
    output: &Path,
    backend: auralis::BackendKind,
    effect_chain: impl IntoIterator<Item = &'a str>,
) -> Result<(), CliError> {
    let options = RenderOptions {
        backend,
        combine: auralis::CombineMethod::Concatenate,
        additional_inputs: Vec::new(),
        output_channels: None,
        no_auto_channels: false,
        output_sample_rate: None,
        no_auto_rate: false,
        guard: OutputGuard::Disabled,
        norm: None,
        dither: OutputDither::Disabled,
        dither_seed: None,
        effects_file: None,
        effect_chain: effect_chain.into_iter().map(ToOwned::to_owned).collect(),
    };

    run_pipeline(input, output, &options)
}

fn normalize_audio(
    input: &Path,
    output: &Path,
    peak: f64,
    backend: auralis::BackendKind,
) -> Result<(), CliError> {
    convert_audio(
        input,
        output,
        ConvertOptions {
            backend,
            output_channels: None,
            no_auto_channels: false,
            output_sample_rate: None,
            no_auto_rate: false,
            guard: OutputGuard::Disabled,
            norm: Some(peak),
            sample: None,
        },
    )
}

fn run_pipeline(input: &Path, output: &Path, options: &RenderOptions) -> Result<(), CliError> {
    ensure_wav_extension(input, PathRole::Input)?;
    ensure_wav_extension(output, PathRole::Output)?;
    for input in &options.additional_inputs {
        ensure_wav_extension(input, PathRole::Input)?;
    }
    let backend = options.backend;
    let channel_conversion_policy = options.channel_conversion_policy()?;
    let sample_rate_conversion_policy = options.sample_rate_conversion_policy()?;
    let output_level_policy = options.output_level_policy()?;
    let output_dither_policy = options.output_dither_policy()?;
    let effect_chain = options.effect_chain()?;
    let pipeline = open_pipeline(input, options, effect_chain.as_ref())?
        .with_backend(backend)
        .with_sample_rate_conversion_policy(sample_rate_conversion_policy)
        .with_channel_conversion_policy(channel_conversion_policy)
        .with_output_level_policy(output_level_policy)
        .with_output_dither_policy(output_dither_policy);

    match effect_chain {
        Some(effect_chain) => pipeline
            .apply_effect_chain(&effect_chain)
            .write_wav(output)?,
        None => pipeline.write_wav(output)?,
    }

    Ok(())
}

fn check_command(
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

fn plan_graph_spec(spec: &Path, json: bool, locked: bool) -> Result<(), CliError> {
    if locked {
        spec::verify_graph_lock(spec)?;
    }
    let checked = spec::check_graph_spec(spec)?;
    let plan = build_plan(&checked);
    let pipeline_name = checked
        .name
        .as_deref()
        .or_else(|| spec.file_stem().and_then(OsStr::to_str))
        .unwrap_or("Auralis.toml");
    if json {
        return print_json_plan(spec, pipeline_name, &checked, &plan);
    }

    println!("Pipeline: {pipeline_name}");
    println!("Spec: {}", spec.display());
    println!();
    println!("Inputs:");
    for source in &checked.sources {
        println!("  {}  {}", source.id, source.path.display());
    }
    println!();
    println!("Outputs:");
    for sink in &checked.sinks {
        println!("  {}  {}", sink.id, sink.path.display());
    }
    println!();
    println!("Graph:");
    println!("  sources: {}", checked.source_count);
    println!("  chains: {}", checked.chain_count);
    println!("  nodes: {}", checked.node_count);
    println!("  sinks: {}", checked.sink_count);
    println!("  expanded steps: {}", checked.expanded_step_ids.len());
    println!("  streaming segments: {}", plan.streaming_segments);
    println!("  whole-buffer barriers: {}", plan.whole_buffer_barriers);
    println!("  fanout points: {}", plan.fanout_points.len());
    println!();
    println!("Execution:");
    for source in &checked.sources {
        println!("  read {}", source.id);
    }
    for chain in &checked.chains {
        println!("  chain {} <- {}", chain.id, chain.input);
        for step_id in &chain.step_ids {
            println!("    step {step_id}");
        }
    }
    for node in &checked.nodes {
        println!("  node {} ({})", node.id, node_display_label(node));
    }
    for sink in &checked.sinks {
        println!("  write {} <- {}", sink.id, sink.input);
    }
    if !plan.segments.is_empty() {
        println!();
        println!("Segments:");
        for segment in &plan.segments {
            println!("  {}  {}", segment.id, segment.summary);
            println!("      mode: {}", segment.mode);
            if let Some(reason) = &segment.reason {
                println!("      reason: {reason}");
            }
        }
    }
    if !plan.fanout_points.is_empty() {
        println!();
        println!("Fanout:");
        for fanout in &plan.fanout_points {
            println!("  {} -> {}", fanout.port, fanout.consumers.join(", "));
        }
    }

    Ok(())
}

#[derive(Debug, Serialize)]
struct JsonPlan {
    pipeline: String,
    spec: String,
    inputs: Vec<JsonPlanIo>,
    outputs: Vec<JsonPlanIo>,
    graph: JsonPlanGraph,
    execution: Vec<JsonPlanStep>,
    segments: Vec<JsonPlanSegment>,
    fanout_points: Vec<JsonPlanFanout>,
}

#[derive(Debug, Serialize)]
struct JsonPlanIo {
    id: String,
    path: String,
}

#[derive(Debug, Serialize)]
struct JsonPlanGraph {
    sources: usize,
    chains: usize,
    nodes: usize,
    sinks: usize,
    expanded_steps: usize,
    streaming_segments: usize,
    whole_buffer_barriers: usize,
    fanout_points: usize,
}

#[derive(Debug, Serialize)]
#[serde(tag = "action")]
enum JsonPlanStep {
    #[serde(rename = "read")]
    Read { id: String },
    #[serde(rename = "chain")]
    Chain {
        id: String,
        input: String,
        steps: Vec<String>,
    },
    #[serde(rename = "node")]
    Node {
        id: String,
        inputs: Vec<String>,
        label: String,
    },
    #[serde(rename = "write")]
    Write { id: String, input: String },
}

#[derive(Debug, Serialize)]
struct JsonPlanSegment {
    id: String,
    mode: String,
    summary: String,
    reason: Option<String>,
}

#[derive(Debug, Serialize)]
struct JsonPlanFanout {
    port: String,
    consumers: Vec<String>,
}

#[derive(Debug, Clone, Copy)]
enum PlanStepMode {
    Streaming,
    WholeBufferBarrier,
    AnalysisPass,
}

impl PlanStepMode {
    const fn label(self) -> &'static str {
        match self {
            Self::Streaming => "streaming",
            Self::WholeBufferBarrier => "whole-buffer barrier",
            Self::AnalysisPass => "analysis pass",
        }
    }
}

#[derive(Debug)]
struct PlanSegment {
    id: String,
    mode: &'static str,
    summary: String,
    reason: Option<String>,
}

#[derive(Debug)]
struct FanoutPoint {
    port: String,
    consumers: Vec<String>,
}

#[derive(Debug)]
struct GraphPlan {
    streaming_segments: usize,
    whole_buffer_barriers: usize,
    segments: Vec<PlanSegment>,
    fanout_points: Vec<FanoutPoint>,
}

fn print_json_plan(
    spec: &Path,
    pipeline_name: &str,
    checked: &spec::CheckedGraphSpec,
    plan: &GraphPlan,
) -> Result<(), CliError> {
    let mut execution = Vec::new();
    for source in &checked.sources {
        execution.push(JsonPlanStep::Read {
            id: source.id.clone(),
        });
    }
    for chain in &checked.chains {
        execution.push(JsonPlanStep::Chain {
            id: chain.id.clone(),
            input: chain.input.clone(),
            steps: chain.step_ids.clone(),
        });
    }
    for node in &checked.nodes {
        execution.push(JsonPlanStep::Node {
            id: node.id.clone(),
            inputs: node.inputs.clone(),
            label: node_display_label(node),
        });
    }
    for sink in &checked.sinks {
        execution.push(JsonPlanStep::Write {
            id: sink.id.clone(),
            input: sink.input.clone(),
        });
    }

    let plan = JsonPlan {
        pipeline: pipeline_name.to_owned(),
        spec: spec.display().to_string(),
        inputs: checked
            .sources
            .iter()
            .map(|source| JsonPlanIo {
                id: source.id.clone(),
                path: source.path.display().to_string(),
            })
            .collect(),
        outputs: checked
            .sinks
            .iter()
            .map(|sink| JsonPlanIo {
                id: sink.id.clone(),
                path: sink.path.display().to_string(),
            })
            .collect(),
        graph: JsonPlanGraph {
            sources: checked.source_count,
            chains: checked.chain_count,
            nodes: checked.node_count,
            sinks: checked.sink_count,
            expanded_steps: checked.expanded_step_ids.len(),
            streaming_segments: plan.streaming_segments,
            whole_buffer_barriers: plan.whole_buffer_barriers,
            fanout_points: plan.fanout_points.len(),
        },
        execution,
        segments: plan
            .segments
            .iter()
            .map(|segment| JsonPlanSegment {
                id: segment.id.clone(),
                mode: segment.mode.to_owned(),
                summary: segment.summary.clone(),
                reason: segment.reason.clone(),
            })
            .collect(),
        fanout_points: plan
            .fanout_points
            .iter()
            .map(|fanout| JsonPlanFanout {
                port: fanout.port.clone(),
                consumers: fanout.consumers.clone(),
            })
            .collect(),
    };

    println!("{}", serde_json::to_string_pretty(&plan)?);
    Ok(())
}

fn build_plan(checked: &spec::CheckedGraphSpec) -> GraphPlan {
    let mut segments = Vec::new();
    let mut streaming_segments = 0;
    let mut whole_buffer_barriers = 0;

    for chain in &checked.chains {
        let upstream = chain
            .input
            .strip_suffix(".audio")
            .unwrap_or(chain.input.as_str());
        let mut stream_steps = vec![format!("{upstream}.read")];
        for (step_id, label) in chain.step_ids.iter().zip(chain.step_labels.iter()) {
            match classify_plan_step(label) {
                (PlanStepMode::Streaming, _) => stream_steps.push(step_id.clone()),
                (mode, reason) => {
                    if stream_steps.len() > 1 {
                        streaming_segments += 1;
                        segments.push(PlanSegment {
                            id: format!("S{streaming_segments}"),
                            mode: PlanStepMode::Streaming.label(),
                            summary: stream_steps.join(" -> "),
                            reason: None,
                        });
                        stream_steps = Vec::new();
                    }
                    whole_buffer_barriers += 1;
                    segments.push(PlanSegment {
                        id: format!("B{whole_buffer_barriers}"),
                        mode: mode.label(),
                        summary: step_id.clone(),
                        reason: Some(reason.to_owned()),
                    });
                }
            }
        }
        if stream_steps.len() > 1 {
            streaming_segments += 1;
            segments.push(PlanSegment {
                id: format!("S{streaming_segments}"),
                mode: PlanStepMode::Streaming.label(),
                summary: stream_steps.join(" -> "),
                reason: None,
            });
        }
    }

    GraphPlan {
        streaming_segments,
        whole_buffer_barriers,
        segments,
        fanout_points: fanout_points(checked),
    }
}

fn classify_plan_step(label: &str) -> (PlanStepMode, &'static str) {
    let op = label.split_whitespace().next().unwrap_or(label);
    match op {
        "reverse" => (
            PlanStepMode::WholeBufferBarrier,
            "reverse requires a full-buffer materialization",
        ),
        "norm" => (
            PlanStepMode::AnalysisPass,
            "norm.peak scans the whole stream before applying gain",
        ),
        _ => (PlanStepMode::Streaming, ""),
    }
}

fn fanout_points(checked: &spec::CheckedGraphSpec) -> Vec<FanoutPoint> {
    let mut consumers = std::collections::BTreeMap::<String, Vec<String>>::new();
    for chain in &checked.chains {
        consumers
            .entry(chain.input.clone())
            .or_default()
            .push(format!("chain {}", chain.id));
    }
    for node in &checked.nodes {
        for input in &node.inputs {
            consumers
                .entry(input.clone())
                .or_default()
                .push(format!("node {}", node.id));
        }
    }
    for sink in &checked.sinks {
        consumers
            .entry(sink.input.clone())
            .or_default()
            .push(format!("sink {}", sink.id));
    }

    consumers
        .into_iter()
        .filter_map(|(port, consumers)| {
            (consumers.len() > 1).then_some(FanoutPoint { port, consumers })
        })
        .collect()
}

fn graph_spec(spec: &Path, format: GraphFormat) -> Result<(), CliError> {
    let checked = spec::check_graph_spec(spec)?;
    match format {
        GraphFormat::Mermaid => print_mermaid_graph(&checked),
        GraphFormat::Dot => print_dot_graph(&checked),
        GraphFormat::Svg => print_svg_graph(&checked),
        GraphFormat::Json => print_json_graph(&checked)?,
    }

    Ok(())
}

struct CompletionSpec {
    name: &'static str,
    options: &'static [&'static str],
}

struct ManPage {
    name: &'static str,
    summary: &'static str,
    synopsis: &'static str,
    description: &'static str,
    options: &'static [(&'static str, &'static str)],
}

const COMPLETION_SPECS: &[CompletionSpec] = &[
    CompletionSpec {
        name: "inspect",
        options: &["--json"],
    },
    CompletionSpec {
        name: "convert",
        options: &[
            "-o",
            "--output",
            "--backend",
            "-c",
            "--channels",
            "--no-auto-channels",
            "-r",
            "--rate",
            "--no-auto-rate",
            "-G",
            "--guard",
            "--norm",
            "--sample",
        ],
    },
    CompletionSpec {
        name: "trim",
        options: &["-o", "--output", "--backend"],
    },
    CompletionSpec {
        name: "normalize",
        options: &["-o", "--output", "--peak", "--backend"],
    },
    CompletionSpec {
        name: "reverse",
        options: &["-o", "--output", "--backend"],
    },
    CompletionSpec {
        name: "render",
        options: &[
            "-o",
            "--output",
            "--backend",
            "--combine",
            "--input",
            "-c",
            "--channels",
            "--no-auto-channels",
            "-r",
            "--rate",
            "--no-auto-rate",
            "-G",
            "--guard",
            "--norm",
            "--dither",
            "--dither-seed",
            "--effects-file",
            "--fx",
            "--chain",
        ],
    },
    CompletionSpec {
        name: "check",
        options: &["--locked", "--effects-file", "--fx", "--chain"],
    },
    CompletionSpec {
        name: "plan",
        options: &["--json", "--locked"],
    },
    CompletionSpec {
        name: "graph",
        options: &["--format"],
    },
    CompletionSpec {
        name: "fmt",
        options: &["--check"],
    },
    CompletionSpec {
        name: "completions",
        options: &[],
    },
    CompletionSpec {
        name: "man",
        options: &[],
    },
    CompletionSpec {
        name: "explain",
        options: &[],
    },
    CompletionSpec {
        name: "run",
        options: &["--locked"],
    },
    CompletionSpec {
        name: "ops",
        options: &["--schema"],
    },
];

const MAN_PAGES: &[ManPage] = &[
    ManPage {
        name: "auralis",
        summary: "modern deterministic audio processing CLI",
        synopsis: "auralis <command> [options]",
        description: "Auralis exposes conversion, ordered render pipelines, graph planning, graph execution, inspection, and developer tooling from one typed command surface.",
        options: &[
            ("inspect", "Print PCM16 WAV metadata, optionally as JSON."),
            (
                "convert",
                "Convert one supported audio file into another container format.",
            ),
            ("trim", "Keep one range from an audio file."),
            ("normalize", "Normalize one audio file to a peak level."),
            ("reverse", "Reverse one audio file."),
            (
                "render",
                "Run one ordered DSP pipeline over one combined input stream.",
            ),
            ("check", "Validate graph specs or typed effect syntax."),
            ("plan", "Preview execution shape for an Auralis graph spec."),
            ("graph", "Emit an Auralis graph as mermaid, dot, or json."),
            ("fmt", "Format an Auralis graph spec."),
            ("completions", "Generate shell completion scripts."),
            ("man", "Print built-in manual pages."),
            (
                "explain",
                "Explain why a node or target participates in execution.",
            ),
            ("run", "Run an Auralis graph spec."),
            ("ops", "Inspect the typed operation registry."),
        ],
    },
    ManPage {
        name: "trim",
        summary: "keep one audio range",
        synopsis: "auralis trim INPUT.wav RANGE -o OUTPUT.wav [--backend BACKEND]",
        description: "Trim is a recipe alias for keeping one contiguous range. It lowers to the same typed effect pipeline as `render --fx 'trim ...'`.",
        options: &[
            (
                "RANGE",
                "Frame range to keep, for example `10..30` or `10..`.",
            ),
            ("-o, --output FILE", "Output WAV file to create."),
            ("--backend BACKEND", "Request scalar or simd processing."),
        ],
    },
    ManPage {
        name: "normalize",
        summary: "normalize to a peak level",
        synopsis: "auralis normalize INPUT.wav -o OUTPUT.wav [--peak DBFS] [--backend BACKEND]",
        description: "Normalize is a recipe alias for output-boundary peak normalization. It lowers to the same output policy as `render --norm`.",
        options: &[
            ("-o, --output FILE", "Output audio file to create."),
            (
                "--peak DBFS",
                "Peak target in dBFS, accepting values like `-1` or `-1dBFS`.",
            ),
            ("--backend BACKEND", "Request scalar or simd processing."),
        ],
    },
    ManPage {
        name: "reverse",
        summary: "reverse one audio file",
        synopsis: "auralis reverse INPUT.wav -o OUTPUT.wav [--backend BACKEND]",
        description: "Reverse is a recipe alias for reversing all frames in one input. It lowers to the same typed effect pipeline as `render --fx reverse`.",
        options: &[
            ("-o, --output FILE", "Output WAV file to create."),
            ("--backend BACKEND", "Request scalar or simd processing."),
        ],
    },
    ManPage {
        name: "render",
        summary: "run one ordered DSP pipeline",
        synopsis: "auralis render INPUT.wav -o OUTPUT.wav [--fx EFFECT]... [--chain CHAIN] [options]",
        description: "Render is the primary linear-chain entry point. It combines optional additional inputs, parses typed effect syntax, applies output-boundary policies, and writes one output artifact.",
        options: &[
            ("-o, --output FILE", "Output WAV file to create."),
            ("--fx EFFECT", "Append one ordered typed effect string."),
            (
                "--chain CHAIN",
                "Use a compact pipe-delimited effect chain.",
            ),
            (
                "--combine METHOD",
                "Combine multiple inputs before effects.",
            ),
            ("--backend BACKEND", "Request scalar or simd processing."),
            ("--channels CHANNELS", "Set the output channel count."),
            ("--rate RATE", "Set the output sample rate."),
            ("--guard", "Apply clip guard at the output boundary."),
            (
                "--norm [DB]",
                "Normalize the output boundary to a peak target.",
            ),
            (
                "--dither",
                "Apply deterministic TPDF dither before PCM16 encoding.",
            ),
        ],
    },
    ManPage {
        name: "plan",
        summary: "preview graph execution",
        synopsis: "auralis plan SPEC [--json] [--locked]",
        description: "Plan validates an Auralis graph spec, exposes streaming segments, whole-buffer barriers, fanout points, and output targets, and can emit a machine-readable JSON form for tooling.",
        options: &[
            ("--json", "Emit machine-readable JSON output."),
            (
                "--locked",
                "Require a matching Auralis.lock before planning.",
            ),
        ],
    },
    ManPage {
        name: "run",
        summary: "execute an Auralis graph spec",
        synopsis: "auralis run SPEC [--locked]",
        description: "Run executes the currently supported source-to-chain-to-sink subset of graph specs. Unsupported graph nodes should be inspected with `plan` first.",
        options: &[(
            "--locked",
            "Require a matching Auralis.lock before running.",
        )],
    },
    ManPage {
        name: "check",
        summary: "validate effect syntax or graph specs",
        synopsis: "auralis check [SPEC] [--locked] [--fx EFFECT]... [--chain CHAIN] [--effects-file FILE]",
        description: "Check validates either one graph spec or one effect-input mode. For graph specs, the default mode refreshes Auralis.lock while `--locked` requires an up-to-date lock.",
        options: &[
            (
                "--locked",
                "Require a matching Auralis.lock instead of refreshing it.",
            ),
            ("--fx EFFECT", "Validate one typed effect string."),
            ("--chain CHAIN", "Validate a compact pipe-delimited chain."),
            (
                "--effects-file FILE",
                "Validate a SoX-ng-style effects file.",
            ),
        ],
    },
    ManPage {
        name: "ops",
        summary: "inspect the typed operation registry",
        synopsis: "auralis ops [EFFECT] [--schema json]",
        description: "Ops lists implemented typed operations, resolves aliases, and can emit machine-readable registry metadata for tooling.",
        options: &[("--schema json", "Emit machine-readable JSON output.")],
    },
];

fn print_completions(shell: CompletionShell) -> Result<(), CliError> {
    match shell {
        CompletionShell::Bash => print_bash_completions(),
        CompletionShell::Zsh => print_zsh_completions(),
        CompletionShell::Fish => print_fish_completions(),
    }
    Ok(())
}

fn print_man_page(topic: Option<&str>) -> Result<(), CliError> {
    let page = topic
        .map(|topic| {
            MAN_PAGES
                .iter()
                .find(|page| page.name == topic)
                .ok_or_else(|| CliError::UnknownManTopic {
                    topic: topic.to_owned(),
                })
        })
        .transpose()?
        .unwrap_or(&MAN_PAGES[0]);

    println!("NAME");
    println!("  {} - {}", page.name, page.summary);
    println!();
    println!("SYNOPSIS");
    println!("  {}", page.synopsis);
    println!();
    println!("DESCRIPTION");
    println!("  {}", page.description);
    if !page.options.is_empty() {
        println!();
        println!("OPTIONS");
        for (name, description) in page.options {
            println!("  {}", name);
            println!("    {}", description);
        }
    }

    Ok(())
}

fn print_bash_completions() {
    let commands = COMPLETION_SPECS
        .iter()
        .map(|spec| spec.name)
        .collect::<Vec<_>>()
        .join(" ");
    println!("_auralis_completions() {{");
    println!("  local cur prev words cword");
    println!("  _init_completion || return");
    println!();
    println!("  case \"$prev\" in");
    println!("    auralis)");
    println!("      COMPREPLY=( $(compgen -W \"{commands}\" -- \"$cur\") )");
    println!("      return");
    println!("      ;;");
    for spec in COMPLETION_SPECS {
        if spec.options.is_empty() {
            continue;
        }
        println!("    {})", spec.name);
        println!(
            "      COMPREPLY=( $(compgen -W \"{}\" -- \"$cur\") )",
            spec.options.join(" ")
        );
        println!("      return");
        println!("      ;;");
    }
    println!("  esac");
    println!();
    println!("  if [[ $cword -eq 1 ]]; then");
    println!("    COMPREPLY=( $(compgen -W \"{commands}\" -- \"$cur\") )");
    println!("    return");
    println!("  fi");
    println!("}}");
    println!("complete -F _auralis_completions auralis");
}

fn print_zsh_completions() {
    println!("#compdef auralis");
    println!("local -a commands");
    println!("commands=(");
    for spec in COMPLETION_SPECS {
        println!("  '{}:{}'", spec.name, spec.name);
    }
    println!(")");
    println!("if (( CURRENT == 2 )); then");
    println!("  _describe 'command' commands");
    println!("  return");
    println!("fi");
    println!("case $words[2] in");
    for spec in COMPLETION_SPECS {
        println!("  {})", spec.name);
        if spec.options.is_empty() {
            println!("    _message 'no additional option completions'");
        } else {
            println!("    _values 'option' \\");
            for option in spec.options {
                println!("      '{}[{} option]' \\", option, spec.name);
            }
            println!("      ;");
        }
        println!("    ;;");
    }
    println!("esac");
}

fn print_fish_completions() {
    for spec in COMPLETION_SPECS {
        println!(
            "complete -c auralis -n '__fish_use_subcommand' -a '{}' -d '{}'",
            spec.name, spec.name
        );
    }
    for spec in COMPLETION_SPECS {
        for option in spec.options {
            let (flag_kind, flag_name) = if let Some(long) = option.strip_prefix("--") {
                ("-l", long)
            } else if let Some(short) = option.strip_prefix('-') {
                ("-s", short)
            } else {
                continue;
            };
            println!(
                "complete -c auralis -n '__fish_seen_subcommand_from {}' {} {}",
                spec.name, flag_kind, flag_name
            );
        }
    }
}

fn format_graph_spec(spec: &Path, check: bool) -> Result<(), CliError> {
    let formatted = spec::format_graph_spec(spec)?;
    let current = fs::read_to_string(spec).map_err(|error| spec::GraphSpecError::Read {
        path: spec.to_path_buf(),
        error,
    })?;
    if check {
        if current == formatted {
            return Ok(());
        }
        return Err(CliError::GraphSpecNeedsFormatting {
            path: spec.to_path_buf(),
        });
    }

    if current != formatted {
        fs::write(spec, formatted)?;
    }
    Ok(())
}

fn explain_graph_target(spec: &Path, target: &str) -> Result<(), CliError> {
    let checked = spec::check_graph_spec(spec)?;

    if let Some(source) = checked.sources.iter().find(|source| source.id == target) {
        println!("Node: {}", source.id);
        println!("Kind: source");
        println!("Path:");
        println!("  {}", source.path.display());
        println!("Output port:");
        println!("  {}.audio", source.id);
        print_downstream(&checked, &format!("{}.audio", source.id));
        return Ok(());
    }

    if let Some(chain) = checked.chains.iter().find(|chain| chain.id == target) {
        println!("Node: {}", chain.id);
        println!("Kind: chain");
        println!("Input:");
        println!("  {}", chain.input);
        println!("Expanded steps:");
        for step_id in &chain.step_ids {
            println!("  {step_id}");
        }
        println!("Execution mode:");
        for (step_id, label) in chain.step_ids.iter().zip(chain.step_labels.iter()) {
            let (mode, reason) = classify_plan_step(label);
            if reason.is_empty() {
                println!("  {step_id:<24} {}", mode.label());
            } else {
                println!("  {step_id:<24} {} ({reason})", mode.label());
            }
        }
        print_downstream(&checked, &format!("{}.audio", chain.id));
        return Ok(());
    }

    if let Some(node) = checked.nodes.iter().find(|node| node.id == target) {
        println!("Node: {}", node.id);
        println!("Kind: node");
        println!("Op:");
        println!("  {}", node_display_label(node));
        println!("Inputs:");
        for input in &node.inputs {
            println!("  {input}");
        }
        println!("Execution mode:");
        let (mode, reason) = classify_node_plan_mode(node);
        if reason.is_empty() {
            println!("  {}", mode.label());
        } else {
            println!("  {} ({reason})", mode.label());
        }
        print_downstream(&checked, &format!("{}.audio", node.id));
        return Ok(());
    }

    if let Some(sink) = checked.sinks.iter().find(|sink| sink.id == target) {
        println!("Node: {}", sink.id);
        println!("Kind: sink");
        println!("Input:");
        println!("  {}", sink.input);
        println!("Path:");
        println!("  {}", sink.path.display());
        println!("Downstream:");
        println!("  none");
        return Ok(());
    }

    Err(CliError::UnknownExplainTarget {
        target: target.to_owned(),
    })
}

fn print_downstream(checked: &spec::CheckedGraphSpec, port: &str) {
    println!("Downstream:");
    let downstream = downstream_consumers(checked, port);
    if downstream.is_empty() {
        println!("  none");
        return;
    }
    for consumer in downstream {
        println!("  {consumer}");
    }
}

fn print_mermaid_graph(checked: &spec::CheckedGraphSpec) {
    println!("flowchart LR");
    for source in &checked.sources {
        println!(
            "  {}[\"source: {}\"]",
            mermaid_id(&source.id),
            source.path.display()
        );
    }
    for chain in &checked.chains {
        let mut previous = chain.input.strip_suffix(".audio").unwrap_or(&chain.input);
        for (step_id, label) in chain.step_ids.iter().zip(chain.step_labels.iter()) {
            let node_id = mermaid_id(step_id);
            println!("  {node_id}[\"{label}\"]");
            println!("  {} --> {node_id}", mermaid_id(previous));
            previous = step_id;
        }
    }
    for node in &checked.nodes {
        println!(
            "  {}[\"{}\"]",
            mermaid_id(&node.id),
            node_display_label(node)
        );
        for input in &node.inputs {
            let upstream = sink_upstream(checked, strip_audio_suffix(input));
            println!("  {} --> {}", mermaid_id(upstream), mermaid_id(&node.id));
        }
    }
    for sink in &checked.sinks {
        println!(
            "  {}[\"sink: {}\"]",
            mermaid_id(&sink.id),
            sink.path.display()
        );
        let input = sink.input.strip_suffix(".audio").unwrap_or(&sink.input);
        let upstream = sink_upstream(checked, input);
        println!("  {} --> {}", mermaid_id(upstream), mermaid_id(&sink.id));
    }
}

fn print_dot_graph(checked: &spec::CheckedGraphSpec) {
    println!("digraph Auralis {{");
    println!("  rankdir=LR;");
    for source in &checked.sources {
        println!(
            "  {} [label={}];",
            dot_id(&source.id),
            dot_label(&format!("source: {}", source.path.display()))
        );
    }
    for chain in &checked.chains {
        let mut previous = chain.input.strip_suffix(".audio").unwrap_or(&chain.input);
        for (step_id, label) in chain.step_ids.iter().zip(chain.step_labels.iter()) {
            println!("  {} [label={}];", dot_id(step_id), dot_label(label));
            println!("  {} -> {};", dot_id(previous), dot_id(step_id));
            previous = step_id;
        }
    }
    for node in &checked.nodes {
        println!(
            "  {} [label={}];",
            dot_id(&node.id),
            dot_label(&node_display_label(node))
        );
        for input in &node.inputs {
            println!(
                "  {} -> {};",
                dot_id(sink_upstream(checked, strip_audio_suffix(input))),
                dot_id(&node.id)
            );
        }
    }
    for sink in &checked.sinks {
        println!(
            "  {} [label={}];",
            dot_id(&sink.id),
            dot_label(&format!("sink: {}", sink.path.display()))
        );
        let input = sink.input.strip_suffix(".audio").unwrap_or(&sink.input);
        println!(
            "  {} -> {};",
            dot_id(sink_upstream(checked, input)),
            dot_id(&sink.id)
        );
    }
    println!("}}");
}

#[derive(Debug, Serialize)]
struct JsonGraph {
    nodes: Vec<JsonGraphNode>,
    edges: Vec<JsonGraphEdge>,
}

#[derive(Debug, Serialize)]
struct JsonGraphNode {
    id: String,
    kind: &'static str,
    label: String,
}

#[derive(Debug, Serialize)]
struct JsonGraphEdge {
    from: String,
    to: String,
}

fn print_json_graph(checked: &spec::CheckedGraphSpec) -> Result<(), CliError> {
    let graph = build_json_graph(checked);

    println!("{}", serde_json::to_string_pretty(&graph)?);
    Ok(())
}

fn build_json_graph(checked: &spec::CheckedGraphSpec) -> JsonGraph {
    let mut graph = JsonGraph {
        nodes: Vec::new(),
        edges: Vec::new(),
    };

    for source in &checked.sources {
        graph.nodes.push(JsonGraphNode {
            id: source.id.clone(),
            kind: "source",
            label: source.path.display().to_string(),
        });
    }

    for chain in &checked.chains {
        let mut previous = chain.input.strip_suffix(".audio").unwrap_or(&chain.input);
        for (step_id, label) in chain.step_ids.iter().zip(chain.step_labels.iter()) {
            graph.nodes.push(JsonGraphNode {
                id: step_id.clone(),
                kind: "step",
                label: label.clone(),
            });
            graph.edges.push(JsonGraphEdge {
                from: previous.to_string(),
                to: step_id.clone(),
            });
            previous = step_id;
        }
    }

    for node in &checked.nodes {
        graph.nodes.push(JsonGraphNode {
            id: node.id.clone(),
            kind: "node",
            label: node_display_label(node),
        });
        for input in &node.inputs {
            graph.edges.push(JsonGraphEdge {
                from: sink_upstream(checked, strip_audio_suffix(input)).to_string(),
                to: node.id.clone(),
            });
        }
    }

    for sink in &checked.sinks {
        graph.nodes.push(JsonGraphNode {
            id: sink.id.clone(),
            kind: "sink",
            label: sink.path.display().to_string(),
        });
        let input = sink.input.strip_suffix(".audio").unwrap_or(&sink.input);
        graph.edges.push(JsonGraphEdge {
            from: sink_upstream(checked, input).to_string(),
            to: sink.id.clone(),
        });
    }

    graph
}

fn print_svg_graph(checked: &spec::CheckedGraphSpec) {
    let graph = build_json_graph(checked);
    let card_width = 200_i32;
    let card_height = 44_i32;
    let gap = 28_i32;
    let margin = 24_i32;
    let width = margin * 2 + card_width;
    let height = margin * 2 + (graph.nodes.len() as i32 * (card_height + gap)).saturating_sub(gap);

    println!(
        "<svg xmlns=\"http://www.w3.org/2000/svg\" width=\"{width}\" height=\"{height}\" viewBox=\"0 0 {width} {height}\">"
    );
    println!("  <style>");
    println!("    text {{ font-family: monospace; font-size: 12px; fill: #111827; }}");
    println!("    .kind {{ font-size: 10px; fill: #6b7280; }}");
    println!("    .source {{ fill: #dbeafe; stroke: #2563eb; }}");
    println!("    .step {{ fill: #dcfce7; stroke: #16a34a; }}");
    println!("    .node {{ fill: #f3e8ff; stroke: #7c3aed; }}");
    println!("    .sink {{ fill: #fee2e2; stroke: #dc2626; }}");
    println!("    .edge {{ stroke: #94a3b8; stroke-width: 2; fill: none; }}");
    println!("  </style>");
    println!("  <defs>");
    println!(
        "    <marker id=\"arrow\" markerWidth=\"10\" markerHeight=\"10\" refX=\"8\" refY=\"3\" orient=\"auto\">"
    );
    println!("      <path d=\"M0,0 L0,6 L9,3 z\" fill=\"#94a3b8\" />");
    println!("    </marker>");
    println!("  </defs>");

    let mut positions = std::collections::BTreeMap::new();
    for (index, node) in graph.nodes.iter().enumerate() {
        let x = margin;
        let y = margin + index as i32 * (card_height + gap);
        positions.insert(node.id.clone(), (x, y));
    }

    for edge in &graph.edges {
        let Some(&(from_x, from_y)) = positions.get(&edge.from) else {
            continue;
        };
        let Some(&(to_x, to_y)) = positions.get(&edge.to) else {
            continue;
        };
        let x1 = from_x + card_width / 2;
        let y1 = from_y + card_height;
        let x2 = to_x + card_width / 2;
        let y2 = to_y;
        println!(
            "  <path class=\"edge\" marker-end=\"url(#arrow)\" d=\"M{x1} {y1} L{x2} {y2}\" />"
        );
    }

    for node in &graph.nodes {
        let (x, y) = positions[&node.id];
        let label = xml_escape(&node.label);
        let kind = xml_escape(node.kind);
        println!(
            "  <rect class=\"{}\" x=\"{x}\" y=\"{y}\" width=\"{card_width}\" height=\"{card_height}\" rx=\"10\" />",
            node.kind
        );
        println!(
            "  <text class=\"kind\" x=\"{}\" y=\"{}\">{kind}</text>",
            x + 12,
            y + 16
        );
        println!("  <text x=\"{}\" y=\"{}\">{label}</text>", x + 12, y + 31);
    }

    println!("</svg>");
}

fn mermaid_id(id: &str) -> String {
    id.chars()
        .map(|character| match character {
            'a'..='z' | 'A'..='Z' | '0'..='9' | '_' => character,
            _ => '_',
        })
        .collect()
}

fn strip_audio_suffix(port: &str) -> &str {
    port.strip_suffix(".audio").unwrap_or(port)
}

fn sink_upstream<'a>(checked: &'a spec::CheckedGraphSpec, input: &'a str) -> &'a str {
    checked
        .chains
        .iter()
        .find(|chain| chain.id == input)
        .and_then(|chain| chain.step_ids.last())
        .map_or(input, String::as_str)
}

fn downstream_consumers(checked: &spec::CheckedGraphSpec, port: &str) -> Vec<String> {
    let mut consumers = Vec::new();
    for chain in &checked.chains {
        if chain.input == port {
            consumers.push(chain.id.clone());
        }
    }
    for node in &checked.nodes {
        if node.inputs.iter().any(|input| input == port) {
            consumers.push(node.id.clone());
        }
    }
    for sink in &checked.sinks {
        if sink.input == port {
            consumers.push(sink.id.clone());
        }
    }
    consumers
}

fn dot_id(id: &str) -> String {
    dot_label(id)
}

fn dot_label(value: &str) -> String {
    format!("\"{}\"", value.replace('\\', "\\\\").replace('"', "\\\""))
}

fn xml_escape(value: &str) -> String {
    value
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&apos;")
}

fn node_display_label(node: &spec::CheckedNode) -> String {
    match node.op.as_deref() {
        None => "passthrough".to_owned(),
        Some("mix.sum") => "mix.sum".to_owned(),
        Some(op) => node_effect_tokens(op, &node.params).map_or_else(
            |_| op.to_owned(),
            |tokens| {
                if tokens.is_empty() {
                    op.to_owned()
                } else {
                    tokens.join(" ")
                }
            },
        ),
    }
}

fn classify_node_plan_mode(node: &spec::CheckedNode) -> (PlanStepMode, &'static str) {
    match node.op.as_deref() {
        None => (PlanStepMode::Streaming, "passthrough node"),
        Some("mix.sum") => (PlanStepMode::Streaming, "multi-input streaming mix"),
        Some("norm.peak") => (
            PlanStepMode::AnalysisPass,
            "norm.peak scans the whole stream before applying gain",
        ),
        Some("reverse") => (
            PlanStepMode::WholeBufferBarrier,
            "reverse requires a full-buffer materialization",
        ),
        Some(_) => (PlanStepMode::Streaming, ""),
    }
}

fn param_as_string(value: &toml::Value) -> Option<String> {
    match value {
        toml::Value::String(value) => Some(value.clone()),
        toml::Value::Integer(value) => Some(value.to_string()),
        toml::Value::Float(value) => Some(value.to_string()),
        _ => None,
    }
}

fn strip_db_suffix(value: &str) -> String {
    value
        .strip_suffix("dBFS")
        .or_else(|| value.strip_suffix("dbfs"))
        .or_else(|| value.strip_suffix("dB"))
        .or_else(|| value.strip_suffix("db"))
        .unwrap_or(value)
        .to_owned()
}

fn param_as_frequency_hz(value: &toml::Value) -> Option<String> {
    let value = param_as_string(value)?;
    Some(
        value
            .strip_suffix("Hz")
            .or_else(|| value.strip_suffix("hz"))
            .unwrap_or(&value)
            .to_owned(),
    )
}

fn fade_curve_token(value: String) -> String {
    match value.as_str() {
        "linear" => "t".to_owned(),
        "logarithmic" => "l".to_owned(),
        "quarter-sine" => "q".to_owned(),
        "half-sine" => "h".to_owned(),
        "inverted-parabola" => "p".to_owned(),
        _ => value,
    }
}

fn run_graph_spec(spec: &Path, locked: bool) -> Result<(), CliError> {
    if locked {
        spec::verify_graph_lock(spec)?;
    }
    let checked = spec::check_graph_spec(spec)?;
    let spec_dir = spec.parent().unwrap_or_else(|| Path::new(""));
    let mut grouped_sinks = std::collections::BTreeMap::<String, Vec<&spec::CheckedSink>>::new();
    for sink in &checked.sinks {
        grouped_sinks
            .entry(sink.input.clone())
            .or_default()
            .push(sink);
    }
    let mut render_cache = std::collections::BTreeMap::<String, auralis::AudioBuffer>::new();

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
    render_cache: &mut std::collections::BTreeMap<String, auralis::AudioBuffer>,
) -> Result<auralis::AudioBuffer, CliError> {
    if let Some(rendered) = render_cache.get(input) {
        return Ok(rendered.clone());
    }

    let mut visited = std::collections::BTreeSet::new();
    let rendered = render_graph_input_audio(checked, spec_dir, input, render_cache, &mut visited)?;
    render_cache.insert(input.to_owned(), rendered.clone());
    Ok(rendered)
}

fn render_graph_input_audio(
    checked: &spec::CheckedGraphSpec,
    spec_dir: &Path,
    input: &str,
    render_cache: &mut std::collections::BTreeMap<String, auralis::AudioBuffer>,
    visited: &mut std::collections::BTreeSet<String>,
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
        let rendered = render_graph_node_audio(checked, spec_dir, node, render_cache, visited)?;
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
    let token_refs: Vec<&str> = chain.effect_tokens.iter().map(String::as_str).collect();
    let effect_chain = auralis::parse_effect_chain(&token_refs)?;
    auralis::AudioFile::from_audio_buffer(upstream)
        .into_pipeline()
        .apply_effect_chain(&effect_chain)
        .into_audio_buffer()
        .map_err(CliError::from)
}

fn render_graph_node_audio(
    checked: &spec::CheckedGraphSpec,
    spec_dir: &Path,
    node: &spec::CheckedNode,
    render_cache: &mut std::collections::BTreeMap<String, auralis::AudioBuffer>,
    visited: &mut std::collections::BTreeSet<String>,
) -> Result<auralis::AudioBuffer, CliError> {
    match node.op.as_deref() {
        None if node.inputs.len() == 1 => {
            render_graph_port_audio(checked, spec_dir, &node.inputs[0], render_cache)
        }
        Some("mix.sum") => {
            render_graph_mix_sum_node(checked, spec_dir, node, render_cache, visited)
        }
        Some(op) if node.inputs.len() == 1 => {
            let upstream =
                render_graph_port_audio(checked, spec_dir, &node.inputs[0], render_cache)?;
            let effect_tokens = node_effect_tokens(op, &node.params)?;
            let token_refs: Vec<&str> = effect_tokens.iter().map(String::as_str).collect();
            let effect_chain = auralis::parse_effect_chain(&token_refs)?;
            auralis::AudioFile::from_audio_buffer(upstream)
                .into_pipeline()
                .apply_effect_chain(&effect_chain)
                .into_audio_buffer()
                .map_err(CliError::from)
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
    render_cache: &mut std::collections::BTreeMap<String, auralis::AudioBuffer>,
    _visited: &mut std::collections::BTreeSet<String>,
) -> Result<auralis::AudioBuffer, CliError> {
    let mut inputs = Vec::with_capacity(node.inputs.len());
    for (index, input) in node.inputs.iter().enumerate() {
        let rendered = render_graph_port_audio(checked, spec_dir, input, render_cache)?;
        let rendered = if let Some(Some(gain)) = node.input_gains.get(index) {
            let token_refs = ["gain", gain.as_str()];
            let effect_chain = auralis::parse_effect_chain(&token_refs)?;
            auralis::AudioFile::from_audio_buffer(rendered)
                .into_pipeline()
                .apply_effect_chain(&effect_chain)
                .into_audio_buffer()?
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

fn node_effect_tokens(
    op: &str,
    params: &std::collections::BTreeMap<String, toml::Value>,
) -> Result<Vec<String>, CliError> {
    match op {
        "gain" => {
            let Some(by) = params.get("by") else {
                return Ok(vec!["gain".to_owned()]);
            };
            let Some(by) = param_as_string(by) else {
                return Err(CliError::UnsupportedGraphNodeOp {
                    op: op.to_owned(),
                    reason: "parameter `by` must be a string or number".to_owned(),
                });
            };
            Ok(vec!["gain".to_owned(), strip_db_suffix(&by)])
        }
        "dcshift" => {
            let Some(shift) = params.get("shift") else {
                return Err(CliError::UnsupportedGraphNodeOp {
                    op: op.to_owned(),
                    reason: "parameter `shift` is required".to_owned(),
                });
            };
            let Some(shift) = param_as_string(shift) else {
                return Err(CliError::UnsupportedGraphNodeOp {
                    op: op.to_owned(),
                    reason: "parameter `shift` must be a string or number".to_owned(),
                });
            };
            Ok(vec!["dcshift".to_owned(), shift])
        }
        "trim" => {
            let Some(range) = params.get("range") else {
                return Err(CliError::UnsupportedGraphNodeOp {
                    op: op.to_owned(),
                    reason: "parameter `range` is required".to_owned(),
                });
            };
            let Some(range) = param_as_string(range) else {
                return Err(CliError::UnsupportedGraphNodeOp {
                    op: op.to_owned(),
                    reason: "parameter `range` must be a string or number".to_owned(),
                });
            };
            let Some((start, end)) = range.split_once("..") else {
                return Err(CliError::UnsupportedGraphNodeOp {
                    op: op.to_owned(),
                    reason: "parameter `range` must use start..end syntax".to_owned(),
                });
            };
            Ok(vec!["trim".to_owned(), start.to_owned(), format!("={end}")])
        }
        "fade" => {
            let Some(fade_in) = params.get("fade_in") else {
                return Err(CliError::UnsupportedGraphNodeOp {
                    op: op.to_owned(),
                    reason: "parameter `fade_in` is required".to_owned(),
                });
            };
            let Some(fade_in) = param_as_string(fade_in) else {
                return Err(CliError::UnsupportedGraphNodeOp {
                    op: op.to_owned(),
                    reason: "parameter `fade_in` must be a string or number".to_owned(),
                });
            };
            let curve = params
                .get("curve")
                .and_then(param_as_string)
                .map_or_else(|| "l".to_owned(), fade_curve_token);
            let fade_out = params.get("fade_out").and_then(param_as_string);
            Ok(match fade_out {
                Some(fade_out) => vec!["fade".to_owned(), curve, fade_in, "0".to_owned(), fade_out],
                None => vec!["fade".to_owned(), curve, fade_in],
            })
        }
        "filter.highpass" => {
            let Some(cutoff) = params.get("cutoff") else {
                return Err(CliError::UnsupportedGraphNodeOp {
                    op: op.to_owned(),
                    reason: "parameter `cutoff` is required".to_owned(),
                });
            };
            let Some(cutoff) = param_as_frequency_hz(cutoff) else {
                return Err(CliError::UnsupportedGraphNodeOp {
                    op: op.to_owned(),
                    reason: "parameter `cutoff` must be a string or number".to_owned(),
                });
            };
            let mut tokens = vec!["highpass".to_owned(), cutoff];
            if let Some(q) = params.get("q") {
                let Some(q) = param_as_string(q) else {
                    return Err(CliError::UnsupportedGraphNodeOp {
                        op: op.to_owned(),
                        reason: "parameter `q` must be a string or number".to_owned(),
                    });
                };
                tokens.push(format!("{q}q"));
            }
            Ok(tokens)
        }
        "norm.peak" => {
            let target = params
                .get("target")
                .and_then(param_as_string)
                .map(|value| strip_db_suffix(&value))
                .unwrap_or_else(|| "0".to_owned());
            Ok(vec!["norm".to_owned(), target])
        }
        unsupported => Err(CliError::UnsupportedGraphNodeOp {
            op: unsupported.to_owned(),
            reason: "node op is not implemented by the current graph runner".to_owned(),
        }),
    }
}

fn resolve_spec_path(spec_dir: &Path, path: &Path) -> PathBuf {
    if path.is_absolute() {
        path.to_path_buf()
    } else {
        spec_dir.join(path)
    }
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

fn print_ops(effect: Option<&str>, schema: Option<OpsSchemaFormat>) -> Result<(), CliError> {
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

#[derive(Debug)]
struct RenderOptions {
    backend: auralis::BackendKind,
    combine: auralis::CombineMethod,
    additional_inputs: Vec<PathBuf>,
    output_channels: Option<auralis::ChannelCount>,
    no_auto_channels: bool,
    output_sample_rate: Option<auralis::SampleRate>,
    no_auto_rate: bool,
    guard: OutputGuard,
    norm: Option<f64>,
    dither: OutputDither,
    dither_seed: Option<u32>,
    effects_file: Option<PathBuf>,
    effect_chain: Vec<String>,
}

#[derive(Debug, Clone, Copy)]
struct ConvertOptions {
    backend: auralis::BackendKind,
    output_channels: Option<auralis::ChannelCount>,
    no_auto_channels: bool,
    output_sample_rate: Option<auralis::SampleRate>,
    no_auto_rate: bool,
    guard: OutputGuard,
    norm: Option<f64>,
    sample: Option<auralis::WavSampleFormat>,
}

impl ConvertOptions {
    fn channel_conversion_policy(&self) -> Result<auralis::ChannelConversionPolicy, CliError> {
        match (self.output_channels, self.no_auto_channels) {
            (None, false) => Ok(auralis::ChannelConversionPolicy::Preserve),
            (Some(channels), false) => Ok(auralis::ChannelConversionPolicy::automatic(channels)),
            (Some(channels), true) => Ok(auralis::ChannelConversionPolicy::require(channels)),
            (None, true) => Err(CliError::NoAutoChannelsWithoutOutputChannels),
        }
    }

    fn sample_rate_conversion_policy(
        &self,
    ) -> Result<auralis::SampleRateConversionPolicy, CliError> {
        match (self.output_sample_rate, self.no_auto_rate) {
            (None, false) => Ok(auralis::SampleRateConversionPolicy::Preserve),
            (Some(sample_rate), false) => {
                Ok(auralis::SampleRateConversionPolicy::automatic(sample_rate))
            }
            (Some(sample_rate), true) => {
                Ok(auralis::SampleRateConversionPolicy::require(sample_rate))
            }
            (None, true) => Err(CliError::NoAutoRateWithoutOutputRate),
        }
    }

    fn output_level_policy(&self) -> Result<auralis::OutputLevelPolicy, CliError> {
        match (self.guard, self.norm) {
            (OutputGuard::Disabled, None) => Ok(auralis::OutputLevelPolicy::Preserve),
            (OutputGuard::Enabled, None) => Ok(auralis::OutputLevelPolicy::guard()),
            (OutputGuard::Disabled, Some(target)) => auralis::Decibels::new(target)
                .map(auralis::OutputLevelPolicy::normalize)
                .map_err(auralis::Error::from)
                .map_err(CliError::from),
            (OutputGuard::Enabled, Some(_)) => Err(CliError::MixedGuardAndNorm),
        }
    }
}

impl RenderOptions {
    fn effect_chain(&self) -> Result<Option<auralis::EffectChain>, CliError> {
        if let Some(path) = &self.effects_file {
            return auralis::parse_effects_file(path)
                .map(Some)
                .map_err(CliError::from);
        }

        if self.effect_chain.is_empty() {
            return Ok(None);
        }

        let tokens: Vec<&str> = self.effect_chain.iter().map(String::as_str).collect();
        auralis::parse_effect_chain(&tokens)
            .map(Some)
            .map_err(CliError::from)
    }

    fn channel_conversion_policy(&self) -> Result<auralis::ChannelConversionPolicy, CliError> {
        match (self.output_channels, self.no_auto_channels) {
            (None, false) => Ok(auralis::ChannelConversionPolicy::Preserve),
            (Some(channels), false) => Ok(auralis::ChannelConversionPolicy::automatic(channels)),
            (Some(channels), true) => Ok(auralis::ChannelConversionPolicy::require(channels)),
            (None, true) => Err(CliError::NoAutoChannelsWithoutOutputChannels),
        }
    }

    fn sample_rate_conversion_policy(
        &self,
    ) -> Result<auralis::SampleRateConversionPolicy, CliError> {
        match (self.output_sample_rate, self.no_auto_rate) {
            (None, false) => Ok(auralis::SampleRateConversionPolicy::Preserve),
            (Some(sample_rate), false) => {
                Ok(auralis::SampleRateConversionPolicy::automatic(sample_rate))
            }
            (Some(sample_rate), true) => {
                Ok(auralis::SampleRateConversionPolicy::require(sample_rate))
            }
            (None, true) => Err(CliError::NoAutoRateWithoutOutputRate),
        }
    }

    fn output_level_policy(&self) -> Result<auralis::OutputLevelPolicy, CliError> {
        match (self.guard, self.norm) {
            (OutputGuard::Disabled, None) => Ok(auralis::OutputLevelPolicy::Preserve),
            (OutputGuard::Enabled, None) => Ok(auralis::OutputLevelPolicy::guard()),
            (OutputGuard::Disabled, Some(target)) => auralis::Decibels::new(target)
                .map(auralis::OutputLevelPolicy::normalize)
                .map_err(auralis::Error::from)
                .map_err(CliError::from),
            (OutputGuard::Enabled, Some(_)) => Err(CliError::MixedGuardAndNorm),
        }
    }

    fn output_dither_policy(&self) -> Result<auralis::OutputDitherPolicy, CliError> {
        match (self.dither, self.dither_seed) {
            (OutputDither::Disabled, None) => Ok(auralis::OutputDitherPolicy::disabled()),
            (OutputDither::Enabled, seed) => {
                let config = seed
                    .map(|seed| auralis::OutputDitherConfig::new().with_seed(seed))
                    .unwrap_or_default();
                Ok(auralis::OutputDitherPolicy::automatic_with_config(config))
            }
            (OutputDither::Disabled, Some(_)) => Err(CliError::DitherSeedWithoutDither),
        }
    }
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

fn effect_input_to_chain_tokens(
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

fn open_pipeline(
    input: &Path,
    options: &RenderOptions,
    effect_chain: Option<&auralis::EffectChain>,
) -> Result<auralis::Pipeline, CliError> {
    if options.additional_inputs.is_empty() {
        if let Some(frames) = effect_chain.and_then(synth_prefix_frame_limit) {
            match auralis_wav::decode_pcm16_prefix_path_with_backend(input, frames, options.backend)
            {
                Ok(audio) => {
                    return Ok(auralis::Pipeline::from_audio_buffer_with_backend(
                        audio,
                        options.backend,
                    ));
                }
                Err(WavError::UnsupportedSampleFormat { .. }) => {}
                Err(error) => return Err(error.into()),
            }
        }

        let audio = auralis::AudioFile::open_wav_with_backend(input, options.backend)?;
        return Ok(audio.into_pipeline());
    }

    let audio = match options.combine {
        auralis::CombineMethod::Concatenate => {
            let mut inputs = Vec::with_capacity(options.additional_inputs.len() + 1);
            inputs.push(input);
            inputs.extend(options.additional_inputs.iter().map(PathBuf::as_path));

            auralis::AudioFile::open_wavs_concatenated_with_backend(inputs, options.backend)?
        }
        auralis::CombineMethod::Sequence => {
            let mut inputs = Vec::with_capacity(options.additional_inputs.len() + 1);
            inputs.push(input);
            inputs.extend(options.additional_inputs.iter().map(PathBuf::as_path));

            auralis::AudioFile::open_wavs_sequenced_with_backend(inputs, options.backend)?
        }
        auralis::CombineMethod::Mix => {
            let mut inputs = Vec::with_capacity(options.additional_inputs.len() + 1);
            inputs.push(input);
            inputs.extend(options.additional_inputs.iter().map(PathBuf::as_path));

            auralis::AudioFile::open_wavs_mixed_with_backend(inputs, options.backend)?
        }
        auralis::CombineMethod::MixPower => {
            let mut inputs = Vec::with_capacity(options.additional_inputs.len() + 1);
            inputs.push(input);
            inputs.extend(options.additional_inputs.iter().map(PathBuf::as_path));

            auralis::AudioFile::open_wavs_mix_powered_with_backend(inputs, options.backend)?
        }
        auralis::CombineMethod::Merge => {
            let mut inputs = Vec::with_capacity(options.additional_inputs.len() + 1);
            inputs.push(input);
            inputs.extend(options.additional_inputs.iter().map(PathBuf::as_path));

            auralis::AudioFile::open_wavs_merged_with_backend(inputs, options.backend)?
        }
        auralis::CombineMethod::Multiply => {
            let mut inputs = Vec::with_capacity(options.additional_inputs.len() + 1);
            inputs.push(input);
            inputs.extend(options.additional_inputs.iter().map(PathBuf::as_path));

            auralis::AudioFile::open_wavs_multiplied_with_backend(inputs, options.backend)?
        }
        _ => unreachable!("the CLI parser only accepts implemented combine methods"),
    };

    Ok(audio.into_pipeline())
}

fn open_audio_file(
    input: &Path,
    backend: auralis::BackendKind,
) -> Result<auralis::AudioFile, CliError> {
    match path_extension(input) {
        Some("wav") => {
            auralis::AudioFile::open_wav_with_backend(input, backend).map_err(CliError::from)
        }
        Some("flac") => auralis::AudioFile::open_flac(input).map_err(CliError::from),
        Some("au" | "snd") => auralis::AudioFile::open_au(input).map_err(CliError::from),
        _ => Err(CliError::UnsupportedConvertInputFormat {
            path: input.to_path_buf(),
        }),
    }
}

fn output_format_from_path(
    output: &Path,
    wav_sample: Option<auralis::WavSampleFormat>,
) -> Result<auralis::OutputFormat, CliError> {
    match path_extension(output) {
        Some("wav") => Ok(auralis::OutputFormat::Wav(match wav_sample {
            Some(sample) => auralis::WavEncodeOptions::new(sample),
            None => auralis::WavEncodeOptions::default(),
        })),
        Some("flac") => {
            if wav_sample.is_some() {
                return Err(CliError::WavSampleFormatRequiresWavOutput);
            }
            Ok(auralis::OutputFormat::Flac(auralis::FlacEncodeOptions))
        }
        Some("aiff" | "aif") => {
            if wav_sample.is_some() {
                return Err(CliError::WavSampleFormatRequiresWavOutput);
            }
            Ok(auralis::OutputFormat::Aiff(
                auralis::AiffEncodeOptions::default(),
            ))
        }
        Some("aifc") => {
            if wav_sample.is_some() {
                return Err(CliError::WavSampleFormatRequiresWavOutput);
            }
            Ok(auralis::OutputFormat::Aiff(
                auralis::AiffEncodeOptions::aifc_signed16_le(),
            ))
        }
        Some("au" | "snd") => {
            if wav_sample.is_some() {
                return Err(CliError::WavSampleFormatRequiresWavOutput);
            }
            Ok(auralis::OutputFormat::Au(
                auralis::AuEncodeOptions::default(),
            ))
        }
        _ => Err(CliError::UnsupportedConvertOutputFormat {
            path: output.to_path_buf(),
        }),
    }
}

fn path_extension(path: &Path) -> Option<&str> {
    path.extension().and_then(OsStr::to_str)
}

fn synth_prefix_frame_limit(effect_chain: &auralis::EffectChain) -> Option<auralis::FrameCount> {
    match effect_chain.commands().first()? {
        auralis::EffectCommand::Synth(synth) => synth.input_prefix_frames(),
        _ => None,
    }
}

#[derive(Debug, Clone, Copy)]
enum OutputGuard {
    Disabled,
    Enabled,
}

impl From<bool> for OutputGuard {
    fn from(value: bool) -> Self {
        if value { Self::Enabled } else { Self::Disabled }
    }
}

#[derive(Debug, Clone, Copy)]
enum OutputDither {
    Disabled,
    Enabled,
}

impl From<bool> for OutputDither {
    fn from(value: bool) -> Self {
        if value { Self::Enabled } else { Self::Disabled }
    }
}

fn ensure_wav_extension(path: &Path, role: PathRole) -> Result<(), CliError> {
    if path.extension().and_then(OsStr::to_str) == Some("wav") {
        Ok(())
    } else {
        Err(CliError::UnsupportedFormat {
            path: path.to_path_buf(),
            role,
        })
    }
}

fn format_duration_seconds(frames: u64, sample_rate: u32) -> String {
    let sample_rate = u64::from(sample_rate);
    let whole = frames / sample_rate;
    let remainder = frames % sample_rate;
    let fractional = remainder * 1_000_000_000 / sample_rate;

    format!("{whole}.{fractional:09}")
}

fn parse_backend(value: &str) -> Result<auralis::BackendKind, String> {
    auralis::BackendKind::from_name(value)
        .ok_or_else(|| "backend must be `scalar` or `simd`".to_owned())
}

fn parse_dbfs(value: &str) -> Result<f64, String> {
    let trimmed = value.trim();
    let number = trimmed
        .strip_suffix("dBFS")
        .or_else(|| trimmed.strip_suffix("dbfs"))
        .unwrap_or(trimmed);

    number
        .parse::<f64>()
        .map_err(|error| format!("invalid dBFS value `{value}`: {error}"))
}

fn parse_combine_method(value: &str) -> Result<auralis::CombineMethod, String> {
    auralis::CombineMethod::from_name(value).ok_or_else(|| {
        "combine method must be `concatenate`, `sequence`, `mix`, `mix-power`, `merge`, or `multiply`"
            .to_owned()
    })
}

fn parse_channel_count(value: &str) -> Result<auralis::ChannelCount, String> {
    let channels = value
        .parse::<u16>()
        .map_err(|_| "channels must be a positive integer no larger than 65535".to_owned())?;

    auralis::ChannelCount::new(channels)
        .map_err(|_| "channels must be a positive integer no larger than 65535".to_owned())
}

fn parse_sample_rate(value: &str) -> Result<auralis::SampleRate, String> {
    let sample_rate = value
        .parse::<u32>()
        .map_err(|_| "rate must be a positive integer no larger than 4294967295".to_owned())?;

    auralis::SampleRate::new(sample_rate)
        .map_err(|_| "rate must be a positive integer no larger than 4294967295".to_owned())
}

fn parse_wav_sample_format(value: &str) -> Result<auralis::WavSampleFormat, String> {
    match value {
        "pcm8" => Ok(auralis::WavSampleFormat::Pcm8),
        "pcm16" => Ok(auralis::WavSampleFormat::Pcm16),
        "pcm24" => Ok(auralis::WavSampleFormat::Pcm24),
        "pcm32" => Ok(auralis::WavSampleFormat::Pcm32),
        "float32" => Ok(auralis::WavSampleFormat::Float32),
        "float64" => Ok(auralis::WavSampleFormat::Float64),
        "ulaw" => Ok(auralis::WavSampleFormat::ULaw),
        "alaw" => Ok(auralis::WavSampleFormat::ALaw),
        _ => Err(
            "sample format must be `pcm8`, `pcm16`, `pcm24`, `pcm32`, `float32`, `float64`, `ulaw`, or `alaw`"
                .to_owned(),
        ),
    }
}

#[derive(Debug)]
enum CliError {
    Auralis(auralis::Error),
    ChainParse(auralis::EffectChainParseError),
    EffectName(auralis::EffectNameError),
    EffectsFile(auralis::EffectsFileReadError),
    GraphSpec(spec::GraphSpecError),
    Json(serde_json::Error),
    Io(std::io::Error),
    Wav(WavError),
    EmptyEffectSpec,
    InvalidEffectSpec { spec: String },
    MissingEffectSpec,
    MixedCheckInputs,
    MixedEffectInputs,
    MixedGuardAndNorm,
    DitherSeedWithoutDither,
    NoAutoChannelsWithoutOutputChannels,
    NoAutoRateWithoutOutputRate,
    LockedRequiresSpec,
    GraphSpecNeedsFormatting { path: PathBuf },
    UnknownManTopic { topic: String },
    UnknownExplainTarget { target: String },
    UnsupportedGraphRunShape,
    UnsupportedGraphNodeInputs { node: String, inputs: Vec<String> },
    UnsupportedGraphNodeOp { op: String, reason: String },
    UnsupportedGraphChainInput { chain: String, input: String },
    UnsupportedGraphSink { sink: String, input: String },
    UnsupportedConvertInputFormat { path: PathBuf },
    UnsupportedConvertOutputFormat { path: PathBuf },
    UnsupportedFormat { path: PathBuf, role: PathRole },
    WavSampleFormatRequiresWavOutput,
}

#[derive(Debug, Clone, Copy)]
enum PathRole {
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

impl std::fmt::Display for CliError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Auralis(error) => write!(formatter, "{error}"),
            Self::ChainParse(error) => write!(formatter, "{error}"),
            Self::EffectName(error) => write!(formatter, "{error}"),
            Self::EffectsFile(error) => write!(formatter, "{error}"),
            Self::GraphSpec(error) => write!(formatter, "{error}"),
            Self::Json(error) => write!(formatter, "{error}"),
            Self::Io(error) => write!(formatter, "{error}"),
            Self::Wav(error) => write!(formatter, "{error}"),
            Self::EmptyEffectSpec => {
                formatter.write_str("effect input requires a non-empty effect")
            }
            Self::InvalidEffectSpec { spec } => write!(
                formatter,
                "effect string `{spec}` contains unmatched shell quoting"
            ),
            Self::MissingEffectSpec => {
                formatter.write_str("one of SPEC, --fx, --chain, or --effects-file is required")
            }
            Self::MixedCheckInputs => {
                formatter.write_str("check accepts either SPEC or one effect input mode")
            }
            Self::MixedEffectInputs => {
                formatter.write_str("--fx, --chain, and --effects-file are mutually exclusive")
            }
            Self::MixedGuardAndNorm => {
                formatter.write_str("--guard cannot be combined with --norm")
            }
            Self::DitherSeedWithoutDither => formatter.write_str("--dither-seed requires --dither"),
            Self::NoAutoChannelsWithoutOutputChannels => {
                formatter.write_str("--no-auto-channels requires --channels")
            }
            Self::NoAutoRateWithoutOutputRate => {
                formatter.write_str("--no-auto-rate requires --rate")
            }
            Self::LockedRequiresSpec => {
                formatter.write_str("--locked requires a graph spec input")
            }
            Self::GraphSpecNeedsFormatting { path } => write!(
                formatter,
                "graph spec {} is not formatted; run `auralis fmt {}`",
                path.display(),
                path.display()
            ),
            Self::UnknownManTopic { topic } => write!(
                formatter,
                "no built-in manual page for `{topic}`"
            ),
            Self::UnknownExplainTarget { target } => write!(
                formatter,
                "spec does not define a source, chain, node, or sink named `{target}`"
            ),
            Self::UnsupportedGraphRunShape => formatter.write_str(
                "run currently supports source-to-chain-to-sink graph specs only; use `plan` to inspect unsupported nodes",
            ),
            Self::UnsupportedGraphNodeInputs { node, inputs } => write!(
                formatter,
                "node `{node}` cannot be run with inputs [{}]; only single-input passthrough nodes are supported",
                inputs.join(", ")
            ),
            Self::UnsupportedGraphNodeOp { op, reason } => write!(
                formatter,
                "node op `{op}` is not supported by the current graph runner: {reason}"
            ),
            Self::UnsupportedGraphChainInput { chain, input } => write!(
                formatter,
                "chain `{chain}` cannot be run from unsupported input `{input}`"
            ),
            Self::UnsupportedGraphSink { sink, input } => write!(
                formatter,
                "sink `{sink}` cannot be run from unsupported input `{input}`"
            ),
            Self::UnsupportedConvertInputFormat { path } => write!(
                formatter,
                "unsupported convert input format for {}; supported inputs are wav, flac, au, and snd",
                path.display()
            ),
            Self::UnsupportedConvertOutputFormat { path } => write!(
                formatter,
                "unsupported convert output format for {}; supported outputs are wav, flac, aiff, aif, aifc, au, and snd",
                path.display()
            ),
            Self::UnsupportedFormat { path, role } => {
                write!(
                    formatter,
                    "unsupported {role} format for {}; only PCM16 WAV is supported",
                    path.display()
                )
            }
            Self::WavSampleFormatRequiresWavOutput => {
                formatter.write_str("--sample is supported only for WAV output")
            }
        }
    }
}

impl From<auralis::Error> for CliError {
    fn from(error: auralis::Error) -> Self {
        Self::Auralis(error)
    }
}

impl From<auralis::EffectChainParseError> for CliError {
    fn from(error: auralis::EffectChainParseError) -> Self {
        Self::ChainParse(error)
    }
}

impl From<auralis::EffectNameError> for CliError {
    fn from(error: auralis::EffectNameError) -> Self {
        Self::EffectName(error)
    }
}

impl From<auralis::EffectsFileReadError> for CliError {
    fn from(error: auralis::EffectsFileReadError) -> Self {
        Self::EffectsFile(error)
    }
}

impl From<spec::GraphSpecError> for CliError {
    fn from(error: spec::GraphSpecError) -> Self {
        Self::GraphSpec(error)
    }
}

impl From<serde_json::Error> for CliError {
    fn from(error: serde_json::Error) -> Self {
        Self::Json(error)
    }
}

impl From<std::io::Error> for CliError {
    fn from(error: std::io::Error) -> Self {
        Self::Io(error)
    }
}

impl From<WavError> for CliError {
    fn from(error: WavError) -> Self {
        Self::Wav(error)
    }
}

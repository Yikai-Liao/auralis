//! Auralis command-line entrypoint.

mod spec;

use std::{
    ffi::OsStr,
    path::{Path, PathBuf},
    process::ExitCode,
};

use auralis::{EffectRegistry, SUPPORTED_EFFECTS};
use auralis_wav::{decode_pcm16_path, WavError};
use clap::{Parser, Subcommand, ValueEnum};

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
    },

    /// Emit an Auralis graph spec as a graph description.
    Graph {
        /// Auralis graph spec to render.
        spec: PathBuf,

        /// Output graph format.
        #[arg(long, value_name = "FORMAT", default_value = "mermaid")]
        format: GraphFormat,
    },

    /// Run an Auralis graph spec.
    Run {
        /// Auralis graph spec to execute.
        spec: PathBuf,
    },

    /// List implemented typed effects or inspect one effect descriptor.
    Ops {
        /// Optional canonical effect name or alias to inspect.
        effect: Option<String>,
    },
}

#[derive(Debug, Clone, Copy, ValueEnum)]
enum GraphFormat {
    Mermaid,
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
        Command::Inspect { input } => inspect(&input),
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
            effects_file,
            fx,
            chain,
        } => check_command(
            spec.as_deref(),
            effects_file.as_deref(),
            &fx,
            chain.as_deref(),
        ),
        Command::Plan { spec } => plan_graph_spec(&spec),
        Command::Graph { spec, format } => graph_spec(&spec, format),
        Command::Run { spec } => run_graph_spec(&spec),
        Command::Ops { effect } => print_ops(effect.as_deref()),
    }
}

fn inspect(input: &Path) -> Result<(), CliError> {
    ensure_wav_extension(input, PathRole::Input)?;
    let audio = decode_pcm16_path(input)?;
    let sample_rate = audio.spec().sample_rate().as_u32();
    let frames = audio.frames().as_u64();
    let duration_seconds = format_duration_seconds(frames, sample_rate);

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
    effects_file: Option<&Path>,
    fx: &[String],
    chain: Option<&str>,
) -> Result<(), CliError> {
    if let Some(spec) = spec {
        if effects_file.is_some() || !fx.is_empty() || chain.is_some() {
            return Err(CliError::MixedCheckInputs);
        }
        return check_graph_spec(spec);
    }

    check_effects(effects_file, fx, chain)
}

fn check_graph_spec(spec: &Path) -> Result<(), CliError> {
    let checked = spec::check_graph_spec(spec)?;

    println!("status: ok");
    println!("sources: {}", checked.source_count);
    println!("chains: {}", checked.chain_count);
    println!("nodes: {}", checked.node_count);
    println!("sinks: {}", checked.sink_count);
    println!("expanded_steps: {}", checked.expanded_step_ids.len());

    Ok(())
}

fn plan_graph_spec(spec: &Path) -> Result<(), CliError> {
    let checked = spec::check_graph_spec(spec)?;
    let pipeline_name = checked
        .name
        .as_deref()
        .or_else(|| spec.file_stem().and_then(OsStr::to_str))
        .unwrap_or("Auralis.toml");

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
        println!("  node {node}");
    }
    for sink in &checked.sinks {
        println!("  write {} <- {}", sink.id, sink.input);
    }

    Ok(())
}

fn graph_spec(spec: &Path, format: GraphFormat) -> Result<(), CliError> {
    let checked = spec::check_graph_spec(spec)?;
    match format {
        GraphFormat::Mermaid => print_mermaid_graph(&checked),
    }

    Ok(())
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
        println!("  {}[\"{}\"]", mermaid_id(node), node);
    }
    for sink in &checked.sinks {
        println!(
            "  {}[\"sink: {}\"]",
            mermaid_id(&sink.id),
            sink.path.display()
        );
        let input = sink.input.strip_suffix(".audio").unwrap_or(&sink.input);
        let upstream = checked
            .chains
            .iter()
            .find(|chain| chain.id == input)
            .and_then(|chain| chain.step_ids.last())
            .map_or(input, String::as_str);
        println!("  {} --> {}", mermaid_id(upstream), mermaid_id(&sink.id));
    }
}

fn mermaid_id(id: &str) -> String {
    id.chars()
        .map(|character| match character {
            'a'..='z' | 'A'..='Z' | '0'..='9' | '_' => character,
            _ => '_',
        })
        .collect()
}

fn run_graph_spec(spec: &Path) -> Result<(), CliError> {
    let checked = spec::check_graph_spec(spec)?;
    if !checked.nodes.is_empty() {
        return Err(CliError::UnsupportedGraphRunShape);
    }

    let spec_dir = spec.parent().unwrap_or_else(|| Path::new(""));
    for sink in &checked.sinks {
        let (source, effect_chain) = resolve_graph_sink_pipeline(&checked, sink)?;
        let input = resolve_spec_path(spec_dir, &source.path);
        let output = resolve_spec_path(spec_dir, &sink.path);

        ensure_wav_extension(&input, PathRole::Input)?;
        ensure_wav_extension(&output, PathRole::Output)?;
        let pipeline = auralis::AudioFile::open_wav(&input)?.into_pipeline();
        match effect_chain {
            Some(effect_chain) => pipeline
                .apply_effect_chain(&effect_chain)
                .write_wav(&output)?,
            None => pipeline.write_wav(&output)?,
        }
        println!("wrote {} <- {}", sink.path.display(), sink.input);
    }

    Ok(())
}

fn resolve_graph_sink_pipeline<'a>(
    checked: &'a spec::CheckedGraphSpec,
    sink: &spec::CheckedSink,
) -> Result<(&'a spec::CheckedSource, Option<auralis::EffectChain>), CliError> {
    let Some(input_id) = sink.input.strip_suffix(".audio") else {
        return Err(CliError::UnsupportedGraphSink {
            sink: sink.id.clone(),
            input: sink.input.clone(),
        });
    };

    if let Some(source) = checked.sources.iter().find(|source| source.id == input_id) {
        return Ok((source, None));
    }

    let chain = checked
        .chains
        .iter()
        .find(|chain| chain.id == input_id)
        .ok_or_else(|| CliError::UnsupportedGraphSink {
            sink: sink.id.clone(),
            input: sink.input.clone(),
        })?;
    let Some(source_id) = chain.input.strip_suffix(".audio") else {
        return Err(CliError::UnsupportedGraphChainInput {
            chain: chain.id.clone(),
            input: chain.input.clone(),
        });
    };
    let source = checked
        .sources
        .iter()
        .find(|source| source.id == source_id)
        .ok_or_else(|| CliError::UnsupportedGraphChainInput {
            chain: chain.id.clone(),
            input: chain.input.clone(),
        })?;
    let token_refs: Vec<&str> = chain.effect_tokens.iter().map(String::as_str).collect();
    let effect_chain = auralis::parse_effect_chain(&token_refs)?;

    Ok((source, Some(effect_chain)))
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

fn print_ops(effect: Option<&str>) -> Result<(), CliError> {
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
    UnsupportedGraphRunShape,
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
            Self::UnsupportedGraphRunShape => formatter.write_str(
                "run currently supports source-to-chain-to-sink graph specs only; use `plan` to inspect unsupported nodes",
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

impl From<WavError> for CliError {
    fn from(error: WavError) -> Self {
        Self::Wav(error)
    }
}

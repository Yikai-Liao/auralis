use std::path::PathBuf;

use clap::{Args, ValueEnum};

use crate::{
    command_support::OpsSchemaFormat,
    completions::CompletionShell,
    parsers::{
        parse_backend, parse_channel_count, parse_combine_method, parse_dbfs, parse_sample_rate,
        parse_wav_sample_format,
    },
};

#[derive(Debug, Args)]
pub(crate) struct InspectArgs {
    /// PCM16 WAV input file to inspect.
    pub(crate) input: PathBuf,

    /// Emit machine-readable JSON output.
    #[arg(long)]
    pub(crate) json: bool,
}

#[derive(Debug, Args)]
pub(crate) struct ConvertArgs {
    /// Input audio file to read.
    pub(crate) input: PathBuf,

    /// Output audio file to create.
    #[arg(short = 'o', long = "output", value_name = "FILE")]
    pub(crate) output: PathBuf,

    /// Sample-processing backend to request.
    #[arg(long, value_name = "BACKEND", default_value = "scalar", value_parser = parse_backend)]
    pub(crate) backend: auralis::BackendKind,

    /// Output channel count; inserts SoX-ng-style channel conversion if needed.
    #[arg(short = 'c', long = "channels", value_name = "CHANNELS", value_parser = parse_channel_count)]
    pub(crate) output_channels: Option<auralis::ChannelCount>,

    /// Fail instead of automatically converting channels for --channels.
    #[arg(long)]
    pub(crate) no_auto_channels: bool,

    /// Output sample rate; inserts deterministic rate conversion if needed.
    #[arg(short = 'r', long = "rate", value_name = "RATE", value_parser = parse_sample_rate)]
    pub(crate) output_sample_rate: Option<auralis::SampleRate>,

    /// Fail instead of automatically converting sample rate for --rate.
    #[arg(long)]
    pub(crate) no_auto_rate: bool,

    /// Attenuate final output only if it would clip.
    #[arg(short = 'G', long)]
    pub(crate) guard: bool,

    /// Normalize final output to a peak level in dBFS, defaulting to 0 dBFS.
    #[arg(long, value_name = "DB", num_args = 0..=1, default_missing_value = "0", allow_hyphen_values = true)]
    pub(crate) norm: Option<f64>,

    /// Select WAV sample encoding when the output container is WAV.
    #[arg(long, value_name = "FORMAT", value_parser = parse_wav_sample_format)]
    pub(crate) sample: Option<auralis::WavSampleFormat>,
}

#[derive(Debug, Args)]
pub(crate) struct TrimArgs {
    /// PCM16 WAV input file to read.
    pub(crate) input: PathBuf,

    /// Frame range to keep, for example `10..30` or `10..`.
    pub(crate) range: String,

    /// Output WAV file to create.
    #[arg(short = 'o', long = "output", value_name = "FILE")]
    pub(crate) output: PathBuf,

    /// Sample-processing backend to request.
    #[arg(long, value_name = "BACKEND", default_value = "scalar", value_parser = parse_backend)]
    pub(crate) backend: auralis::BackendKind,
}

#[derive(Debug, Args)]
pub(crate) struct NormalizeArgs {
    /// Input audio file to read.
    pub(crate) input: PathBuf,

    /// Output audio file to create.
    #[arg(short = 'o', long = "output", value_name = "FILE")]
    pub(crate) output: PathBuf,

    /// Peak target in dBFS, defaulting to 0 dBFS.
    #[arg(long, value_name = "DBFS", default_value = "0", allow_hyphen_values = true, value_parser = parse_dbfs)]
    pub(crate) peak: f64,

    /// Sample-processing backend to request.
    #[arg(long, value_name = "BACKEND", default_value = "scalar", value_parser = parse_backend)]
    pub(crate) backend: auralis::BackendKind,
}

#[derive(Debug, Args)]
pub(crate) struct NormArgs {
    /// PCM16 WAV input file to read.
    pub(crate) input: PathBuf,

    /// Peak target in dBFS.
    #[arg(value_name = "DBFS", default_value = "0", allow_hyphen_values = true)]
    pub(crate) level: String,

    /// Output WAV file to create.
    #[arg(short = 'o', long = "output", value_name = "FILE")]
    pub(crate) output: PathBuf,

    /// Sample-processing backend to request.
    #[arg(long, value_name = "BACKEND", default_value = "scalar", value_parser = parse_backend)]
    pub(crate) backend: auralis::BackendKind,
}

#[derive(Debug, Args)]
pub(crate) struct RateArgs {
    /// PCM16 WAV input file to read.
    pub(crate) input: PathBuf,

    /// Target sample rate in Hz.
    #[arg(value_name = "RATE")]
    pub(crate) sample_rate: String,

    /// Output WAV file to create.
    #[arg(short = 'o', long = "output", value_name = "FILE")]
    pub(crate) output: PathBuf,

    /// Sample-processing backend to request.
    #[arg(long, value_name = "BACKEND", default_value = "scalar", value_parser = parse_backend)]
    pub(crate) backend: auralis::BackendKind,
}

#[derive(Debug, Args)]
pub(crate) struct ChannelsArgs {
    /// PCM16 WAV input file to read.
    pub(crate) input: PathBuf,

    /// Target channel count.
    #[arg(value_name = "CHANNELS")]
    pub(crate) count: String,

    /// Output WAV file to create.
    #[arg(short = 'o', long = "output", value_name = "FILE")]
    pub(crate) output: PathBuf,

    /// Sample-processing backend to request.
    #[arg(long, value_name = "BACKEND", default_value = "scalar", value_parser = parse_backend)]
    pub(crate) backend: auralis::BackendKind,
}

#[derive(Debug, Args)]
pub(crate) struct GainArgs {
    /// PCM16 WAV input file to read.
    pub(crate) input: PathBuf,

    /// Gain adjustment in dB, for example `-3` or `-3dB`.
    #[arg(value_name = "DB", allow_hyphen_values = true)]
    pub(crate) db: String,

    /// Output WAV file to create.
    #[arg(short = 'o', long = "output", value_name = "FILE")]
    pub(crate) output: PathBuf,

    /// Sample-processing backend to request.
    #[arg(long, value_name = "BACKEND", default_value = "scalar", value_parser = parse_backend)]
    pub(crate) backend: auralis::BackendKind,
}

#[derive(Debug, Args)]
pub(crate) struct SimpleRecipeArgs {
    /// PCM16 WAV input file to read.
    pub(crate) input: PathBuf,

    /// Output WAV file to create.
    #[arg(short = 'o', long = "output", value_name = "FILE")]
    pub(crate) output: PathBuf,

    /// Sample-processing backend to request.
    #[arg(long, value_name = "BACKEND", default_value = "scalar", value_parser = parse_backend)]
    pub(crate) backend: auralis::BackendKind,
}

#[derive(Debug, Args)]
pub(crate) struct EchoArgs {
    /// PCM16 WAV input file to read.
    pub(crate) input: PathBuf,

    /// Clean input gain.
    #[arg(
        long = "gain-in",
        value_name = "GAIN",
        default_value = "0.8",
        allow_hyphen_values = true
    )]
    pub(crate) gain_in: String,

    /// Output gain.
    #[arg(
        long = "gain-out",
        value_name = "GAIN",
        default_value = "0.9",
        allow_hyphen_values = true
    )]
    pub(crate) gain_out: String,

    /// Echo tap as `delay_ms,decay`; repeat for multiple taps.
    #[arg(
        long = "tap",
        value_name = "DELAY_MS,DECAY",
        required = true,
        allow_hyphen_values = true
    )]
    pub(crate) taps: Vec<String>,

    /// Output WAV file to create.
    #[arg(short = 'o', long = "output", value_name = "FILE")]
    pub(crate) output: PathBuf,

    /// Sample-processing backend to request.
    #[arg(long, value_name = "BACKEND", default_value = "scalar", value_parser = parse_backend)]
    pub(crate) backend: auralis::BackendKind,
}

#[derive(Debug, Args)]
#[allow(clippy::struct_excessive_bools)]
pub(crate) struct RenderArgs {
    /// PCM16 WAV input file to read.
    pub(crate) input: PathBuf,

    /// Output WAV file to create.
    #[arg(short = 'o', long = "output", value_name = "FILE")]
    pub(crate) output: PathBuf,

    /// Sample-processing backend to request.
    #[arg(long, value_name = "BACKEND", default_value = "scalar", value_parser = parse_backend)]
    pub(crate) backend: auralis::BackendKind,

    /// Input-combiner method to apply before effects.
    #[arg(long, value_name = "METHOD", default_value = "concatenate", value_parser = parse_combine_method)]
    pub(crate) combine: auralis::CombineMethod,

    /// Additional PCM16 WAV input files to combine after the first input.
    #[arg(long = "input", value_name = "FILE")]
    pub(crate) additional_inputs: Vec<PathBuf>,

    /// Output channel count; inserts SoX-ng-style channel conversion if needed.
    #[arg(short = 'c', long = "channels", value_name = "CHANNELS", value_parser = parse_channel_count)]
    pub(crate) output_channels: Option<auralis::ChannelCount>,

    /// Fail instead of automatically converting channels for --channels.
    #[arg(long)]
    pub(crate) no_auto_channels: bool,

    /// Output sample rate; inserts deterministic rate conversion if needed.
    #[arg(short = 'r', long = "rate", value_name = "RATE", value_parser = parse_sample_rate)]
    pub(crate) output_sample_rate: Option<auralis::SampleRate>,

    /// Fail instead of automatically converting sample rate for --rate.
    #[arg(long)]
    pub(crate) no_auto_rate: bool,

    /// Attenuate final output only if it would clip.
    #[arg(short = 'G', long)]
    pub(crate) guard: bool,

    /// Normalize final output to a peak level in dBFS, defaulting to 0 dBFS.
    #[arg(long, value_name = "DB", num_args = 0..=1, default_missing_value = "0", allow_hyphen_values = true)]
    pub(crate) norm: Option<f64>,

    /// Apply deterministic TPDF dither before PCM16 encoding.
    #[arg(long)]
    pub(crate) dither: bool,

    /// Deterministic seed used when --dither is enabled.
    #[arg(long, value_name = "SEED")]
    pub(crate) dither_seed: Option<u32>,

    /// Read the effect chain from a SoX-ng-style effects file.
    #[arg(long, value_name = "FILE")]
    pub(crate) effects_file: Option<PathBuf>,

    /// One typed effect command per flag, for example `--fx 'gain -3'`.
    #[arg(long = "fx", value_name = "EFFECT")]
    pub(crate) fx: Vec<String>,

    /// Compact ordered effect chain, for example `--chain 'gain -3 | reverse'`.
    #[arg(long = "chain", value_name = "CHAIN")]
    pub(crate) chain: Option<String>,
}

#[derive(Debug, Args)]
pub(crate) struct CheckArgs {
    /// Auralis graph spec to validate.
    pub(crate) spec: Option<PathBuf>,

    /// Require an up-to-date Auralis.lock instead of refreshing it.
    #[arg(long)]
    pub(crate) locked: bool,

    /// Read the effect chain from a SoX-ng-style effects file.
    #[arg(long, value_name = "FILE")]
    pub(crate) effects_file: Option<PathBuf>,

    /// One typed effect command per flag, for example `--fx 'gain -3'`.
    #[arg(long = "fx", value_name = "EFFECT")]
    pub(crate) fx: Vec<String>,

    /// Compact ordered effect chain, for example `--chain 'gain -3 | reverse'`.
    #[arg(long = "chain", value_name = "CHAIN")]
    pub(crate) chain: Option<String>,
}

#[derive(Debug, Args)]
pub(crate) struct PlanArgs {
    /// Auralis graph spec to plan.
    pub(crate) spec: PathBuf,

    /// Emit machine-readable JSON output.
    #[arg(long)]
    pub(crate) json: bool,

    /// Require an up-to-date Auralis.lock before planning.
    #[arg(long)]
    pub(crate) locked: bool,
}

#[derive(Debug, Args)]
pub(crate) struct GraphArgs {
    /// Auralis graph spec to render.
    pub(crate) spec: PathBuf,

    /// Output graph format.
    #[arg(long, value_name = "FORMAT", default_value = "mermaid")]
    pub(crate) format: GraphFormat,
}

#[derive(Debug, Args)]
pub(crate) struct FmtArgs {
    /// Auralis graph spec to format.
    pub(crate) spec: PathBuf,

    /// Check whether formatting changes would be required.
    #[arg(long)]
    pub(crate) check: bool,
}

#[derive(Debug, Args)]
pub(crate) struct CompletionsArgs {
    /// Shell to generate completions for.
    pub(crate) shell: CompletionShell,
}

#[derive(Debug, Args)]
pub(crate) struct ManArgs {
    /// Optional command topic, for example `render` or `plan`.
    pub(crate) topic: Option<String>,
}

#[derive(Debug, Args)]
pub(crate) struct ExplainArgs {
    /// Auralis graph spec to inspect.
    pub(crate) spec: PathBuf,

    /// Chain, node, sink, or source id to explain.
    pub(crate) target: String,
}

#[derive(Debug, Args)]
pub(crate) struct RunArgs {
    /// Auralis graph spec to execute.
    pub(crate) spec: PathBuf,

    /// Require an up-to-date Auralis.lock before running.
    #[arg(long)]
    pub(crate) locked: bool,
}

#[derive(Debug, Args)]
pub(crate) struct OpsArgs {
    /// Optional canonical effect name or alias to inspect.
    pub(crate) effect: Option<String>,

    /// Emit machine-readable schema output.
    #[arg(long, value_name = "FORMAT")]
    pub(crate) schema: Option<OpsSchemaFormat>,
}

#[derive(Debug, Clone, Copy, ValueEnum)]
pub(crate) enum GraphFormat {
    Mermaid,
    Dot,
    Svg,
    Json,
}

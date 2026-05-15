use std::path::PathBuf;

use clap::{Args, Subcommand, ValueEnum};

use crate::{
    command_support::OpsSchemaFormat,
    completions::CompletionShell,
    executor::OutputContainer,
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

    /// Select output container explicitly instead of inferring it from -o.
    #[arg(long, value_name = "CONTAINER", value_enum)]
    pub(crate) container: Option<OutputContainer>,

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
pub(crate) struct ChorusArgs {
    /// PCM16 WAV input file to read.
    pub(crate) input: PathBuf,

    /// Clean input gain.
    #[arg(
        long = "gain-in",
        value_name = "GAIN",
        default_value = "0.5",
        allow_hyphen_values = true
    )]
    pub(crate) gain_in: String,

    /// Output gain.
    #[arg(
        long = "gain-out",
        value_name = "GAIN",
        default_value = "1",
        allow_hyphen_values = true
    )]
    pub(crate) gain_out: String,

    /// Interpolation mode: none, linear, or quadratic.
    #[arg(long, value_name = "MODE", default_value = "none")]
    pub(crate) interpolation: String,

    /// Default modulation wave: sine or triangle.
    #[arg(long, value_name = "WAVE", default_value = "sine")]
    pub(crate) wave: String,

    /// Chorus stage as `delay_ms,decay,speed_hz,depth_ms[,wave]`; repeat for multiple stages.
    #[arg(long = "stage", value_name = "STAGE", allow_hyphen_values = true)]
    pub(crate) stages: Vec<String>,

    /// Output WAV file to create.
    #[arg(short = 'o', long = "output", value_name = "FILE")]
    pub(crate) output: PathBuf,

    /// Sample-processing backend to request.
    #[arg(long, value_name = "BACKEND", default_value = "scalar", value_parser = parse_backend)]
    pub(crate) backend: auralis::BackendKind,
}

#[derive(Debug, Args)]
pub(crate) struct FlangerArgs {
    /// PCM16 WAV input file to read.
    pub(crate) input: PathBuf,

    /// Base delay in milliseconds.
    #[arg(
        long,
        value_name = "MS",
        default_value = "0",
        allow_hyphen_values = true
    )]
    pub(crate) delay: String,

    /// Sweep depth in milliseconds.
    #[arg(
        long,
        value_name = "MS",
        default_value = "2",
        allow_hyphen_values = true
    )]
    pub(crate) depth: String,

    /// Regeneration percentage.
    #[arg(
        long,
        value_name = "PERCENT",
        default_value = "0",
        allow_hyphen_values = true
    )]
    pub(crate) regen: String,

    /// Wet width percentage.
    #[arg(
        long,
        value_name = "PERCENT",
        default_value = "71",
        allow_hyphen_values = true
    )]
    pub(crate) width: String,

    /// Modulation speed in Hz.
    #[arg(
        long,
        value_name = "HZ",
        default_value = "0.5",
        allow_hyphen_values = true
    )]
    pub(crate) speed: String,

    /// Modulation wave: sine or triangle.
    #[arg(long, value_name = "WAVE", default_value = "sine")]
    pub(crate) wave: String,

    /// Stereo phase percentage.
    #[arg(
        long,
        value_name = "PERCENT",
        default_value = "25",
        allow_hyphen_values = true
    )]
    pub(crate) phase: String,

    /// Interpolation mode: none, linear, or quadratic.
    #[arg(long, value_name = "MODE", default_value = "linear")]
    pub(crate) interpolation: String,

    /// Output WAV file to create.
    #[arg(short = 'o', long = "output", value_name = "FILE")]
    pub(crate) output: PathBuf,

    /// Sample-processing backend to request.
    #[arg(long, value_name = "BACKEND", default_value = "scalar", value_parser = parse_backend)]
    pub(crate) backend: auralis::BackendKind,
}

#[derive(Debug, Args)]
pub(crate) struct PhaserArgs {
    /// PCM16 WAV input file to read.
    pub(crate) input: PathBuf,

    /// Clean input gain.
    #[arg(
        long = "gain-in",
        value_name = "GAIN",
        default_value = "0.4",
        allow_hyphen_values = true
    )]
    pub(crate) gain_in: String,

    /// Output gain.
    #[arg(
        long = "gain-out",
        value_name = "GAIN",
        default_value = "0.74",
        allow_hyphen_values = true
    )]
    pub(crate) gain_out: String,

    /// Delay in milliseconds.
    #[arg(
        long,
        value_name = "MS",
        default_value = "3",
        allow_hyphen_values = true
    )]
    pub(crate) delay: String,

    /// Regeneration amount.
    #[arg(
        long,
        value_name = "AMOUNT",
        default_value = "0.4",
        allow_hyphen_values = true
    )]
    pub(crate) regen: String,

    /// Modulation speed in Hz.
    #[arg(
        long,
        value_name = "HZ",
        default_value = "0.5",
        allow_hyphen_values = true
    )]
    pub(crate) speed: String,

    /// Modulation wave: sine or triangle.
    #[arg(long, value_name = "WAVE", default_value = "sine")]
    pub(crate) wave: String,

    /// Interpolation mode: none, linear, or quadratic.
    #[arg(long, value_name = "MODE", default_value = "none")]
    pub(crate) interpolation: String,

    /// Output WAV file to create.
    #[arg(short = 'o', long = "output", value_name = "FILE")]
    pub(crate) output: PathBuf,

    /// Sample-processing backend to request.
    #[arg(long, value_name = "BACKEND", default_value = "scalar", value_parser = parse_backend)]
    pub(crate) backend: auralis::BackendKind,
}

#[derive(Debug, Args)]
pub(crate) struct ContrastArgs {
    /// PCM16 WAV input file to read.
    pub(crate) input: PathBuf,

    /// Contrast amount from 0 to 100.
    #[arg(long, value_name = "AMOUNT", default_value = "75")]
    pub(crate) amount: String,

    /// Output WAV file to create.
    #[arg(short = 'o', long = "output", value_name = "FILE")]
    pub(crate) output: PathBuf,

    /// Sample-processing backend to request.
    #[arg(long, value_name = "BACKEND", default_value = "scalar", value_parser = parse_backend)]
    pub(crate) backend: auralis::BackendKind,
}

#[derive(Debug, Args)]
pub(crate) struct OverdriveArgs {
    /// PCM16 WAV input file to read.
    pub(crate) input: PathBuf,

    /// Overdrive gain.
    #[arg(
        long,
        value_name = "GAIN",
        default_value = "20",
        allow_hyphen_values = true
    )]
    pub(crate) gain: String,

    /// Overdrive color.
    #[arg(
        long,
        value_name = "COLOR",
        default_value = "20",
        allow_hyphen_values = true
    )]
    pub(crate) color: String,

    /// Output WAV file to create.
    #[arg(short = 'o', long = "output", value_name = "FILE")]
    pub(crate) output: PathBuf,

    /// Sample-processing backend to request.
    #[arg(long, value_name = "BACKEND", default_value = "scalar", value_parser = parse_backend)]
    pub(crate) backend: auralis::BackendKind,
}

#[derive(Debug, Args)]
pub(crate) struct SaturationArgs {
    /// PCM16 WAV input file to read.
    pub(crate) input: PathBuf,

    /// Saturation curve type: tanh, sqrt, or diode.
    #[arg(long = "type", value_name = "TYPE", default_value = "tanh")]
    pub(crate) saturation_type: String,

    /// Wet/dry blend amount.
    #[arg(
        long,
        value_name = "BLEND",
        default_value = "1",
        allow_hyphen_values = true
    )]
    pub(crate) blend: String,

    /// Input offset before saturation.
    #[arg(
        long,
        value_name = "OFFSET",
        default_value = "0",
        allow_hyphen_values = true
    )]
    pub(crate) offset: String,

    /// Curve-specific parameter: drive, color, or threshold.
    #[arg(long, value_name = "VALUE", allow_hyphen_values = true)]
    pub(crate) parameter: Option<String>,

    /// Output WAV file to create.
    #[arg(short = 'o', long = "output", value_name = "FILE")]
    pub(crate) output: PathBuf,

    /// Sample-processing backend to request.
    #[arg(long, value_name = "BACKEND", default_value = "scalar", value_parser = parse_backend)]
    pub(crate) backend: auralis::BackendKind,
}

#[derive(Debug, Args)]
pub(crate) struct DcShiftArgs {
    /// PCM16 WAV input file to read.
    pub(crate) input: PathBuf,

    /// DC shift amount.
    #[arg(value_name = "SHIFT", allow_hyphen_values = true)]
    pub(crate) shift: String,

    /// Optional limiter gain.
    #[arg(long, value_name = "GAIN", allow_hyphen_values = true)]
    pub(crate) limiter_gain: Option<String>,

    /// Output WAV file to create.
    #[arg(short = 'o', long = "output", value_name = "FILE")]
    pub(crate) output: PathBuf,

    /// Sample-processing backend to request.
    #[arg(long, value_name = "BACKEND", default_value = "scalar", value_parser = parse_backend)]
    pub(crate) backend: auralis::BackendKind,
}

#[derive(Debug, Args)]
pub(crate) struct VolArgs {
    /// PCM16 WAV input file to read.
    pub(crate) input: PathBuf,

    /// Volume gain value, for example `0.5` or `-6dB`.
    #[arg(value_name = "GAIN", allow_hyphen_values = true)]
    pub(crate) gain: String,

    /// Gain interpretation: amplitude, power, or dB.
    #[arg(long = "type", value_name = "TYPE")]
    pub(crate) gain_type: Option<String>,

    /// Optional limiter gain.
    #[arg(long, value_name = "GAIN", allow_hyphen_values = true)]
    pub(crate) limiter_gain: Option<String>,

    /// Output WAV file to create.
    #[arg(short = 'o', long = "output", value_name = "FILE")]
    pub(crate) output: PathBuf,

    /// Sample-processing backend to request.
    #[arg(long, value_name = "BACKEND", default_value = "scalar", value_parser = parse_backend)]
    pub(crate) backend: auralis::BackendKind,
}

#[derive(Debug, Args)]
pub(crate) struct SoftVolArgs {
    /// PCM16 WAV input file to read.
    pub(crate) input: PathBuf,

    /// Volume multiplier.
    #[arg(
        long,
        value_name = "VOLUME",
        default_value = "1",
        allow_hyphen_values = true
    )]
    pub(crate) volume: String,

    /// Seconds required for volume doubling.
    #[arg(
        long,
        value_name = "SECONDS",
        default_value = "0",
        allow_hyphen_values = true
    )]
    pub(crate) double_time: String,

    /// Extra headroom in dB.
    #[arg(
        long,
        value_name = "DB",
        default_value = "0",
        allow_hyphen_values = true
    )]
    pub(crate) headroom: String,

    /// Output WAV file to create.
    #[arg(short = 'o', long = "output", value_name = "FILE")]
    pub(crate) output: PathBuf,

    /// Sample-processing backend to request.
    #[arg(long, value_name = "BACKEND", default_value = "scalar", value_parser = parse_backend)]
    pub(crate) backend: auralis::BackendKind,
}

#[derive(Debug, Args)]
pub(crate) struct TremoloArgs {
    /// PCM16 WAV input file to read.
    pub(crate) input: PathBuf,

    /// Modulation speed in Hz.
    #[arg(value_name = "SPEED_HZ", allow_hyphen_values = true)]
    pub(crate) speed: String,

    /// Modulation depth percentage.
    #[arg(
        long,
        value_name = "PERCENT",
        default_value = "40",
        allow_hyphen_values = true
    )]
    pub(crate) depth: String,

    /// Output WAV file to create.
    #[arg(short = 'o', long = "output", value_name = "FILE")]
    pub(crate) output: PathBuf,

    /// Sample-processing backend to request.
    #[arg(long, value_name = "BACKEND", default_value = "scalar", value_parser = parse_backend)]
    pub(crate) backend: auralis::BackendKind,
}

#[derive(Debug, Args)]
pub(crate) struct SpeedArgs {
    /// PCM16 WAV input file to read.
    pub(crate) input: PathBuf,

    /// Speed factor, or cents with a `c` suffix.
    #[arg(value_name = "FACTOR", allow_hyphen_values = true)]
    pub(crate) factor: String,

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

    /// Select output container explicitly instead of inferring it from -o.
    #[arg(long, value_name = "CONTAINER", value_enum)]
    pub(crate) container: Option<OutputContainer>,

    /// Select WAV sample encoding when the output container is WAV.
    #[arg(long, value_name = "FORMAT", value_parser = parse_wav_sample_format)]
    pub(crate) sample: Option<auralis::WavSampleFormat>,

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
pub(crate) struct PipeArgs {
    /// PCM16 WAV input file to read.
    pub(crate) input: PathBuf,

    /// Pipe-delimited ordered effect expression, for example `gain -3 | reverse`.
    #[arg(value_name = "EXPR")]
    pub(crate) expression: String,

    /// Output WAV file to create.
    #[arg(short = 'o', long = "output", value_name = "FILE")]
    pub(crate) output: PathBuf,

    /// Sample-processing backend to request.
    #[arg(long, value_name = "BACKEND", default_value = "scalar", value_parser = parse_backend)]
    pub(crate) backend: auralis::BackendKind,
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

    /// Plan only the named target.
    #[arg(long, value_name = "TARGET")]
    pub(crate) target: Option<String>,

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

    /// Output file to write instead of stdout.
    #[arg(short = 'o', long = "output", value_name = "FILE")]
    pub(crate) output: Option<PathBuf>,

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
pub(crate) struct InitArgs {
    /// Graph spec path to create.
    #[arg(default_value = "Auralis.toml")]
    pub(crate) spec: PathBuf,
}

#[derive(Debug, Args)]
pub(crate) struct CacheArgs {
    #[command(subcommand)]
    pub(crate) command: CacheCommand,
}

#[derive(Debug, Subcommand)]
pub(crate) enum CacheCommand {
    /// Print local persistent-cache status.
    Status(CacheStatusArgs),

    /// Remove files from the local persistent cache.
    Clear(CacheClearArgs),
}

#[derive(Debug, Args)]
pub(crate) struct CacheStatusArgs {
    /// Cache root directory to inspect.
    #[arg(long, value_name = "DIR", default_value = ".auralis/cache")]
    pub(crate) root: PathBuf,

    /// Emit machine-readable JSON output.
    #[arg(long)]
    pub(crate) json: bool,
}

#[derive(Debug, Args)]
pub(crate) struct CacheClearArgs {
    /// Cache root directory to clear.
    #[arg(long, value_name = "DIR", default_value = ".auralis/cache")]
    pub(crate) root: PathBuf,

    /// Confirm destructive cache removal.
    #[arg(long)]
    pub(crate) yes: bool,
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
    #[arg(default_value = "Auralis.toml")]
    pub(crate) spec: PathBuf,

    /// Run only the named target.
    #[arg(long, value_name = "TARGET")]
    pub(crate) target: Option<String>,

    /// Require an up-to-date Auralis.lock before running.
    #[arg(long)]
    pub(crate) locked: bool,

    /// Graph execution cache policy.
    #[arg(long, value_enum, default_value_t = CacheMode::Smart)]
    pub(crate) cache: CacheMode,
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

#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum)]
pub(crate) enum CacheMode {
    Off,
    Smart,
    Full,
}

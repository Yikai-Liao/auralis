use std::path::PathBuf;

use clap::Args;

use crate::parsers::{parse_backend, parse_filter_poles};

#[derive(Debug, Args)]
pub(crate) struct TempoArgs {
    /// PCM16 WAV input file to read.
    pub(crate) input: PathBuf,

    /// Tempo factor.
    #[arg(value_name = "FACTOR", allow_hyphen_values = true)]
    pub(crate) factor: String,

    /// Prefer quicker search.
    #[arg(long)]
    pub(crate) quick: bool,

    /// Tuning profile: music, speech, or linear.
    #[arg(long, value_name = "PROFILE")]
    pub(crate) profile: Option<String>,

    /// Segment length in milliseconds.
    #[arg(long, value_name = "MS", allow_hyphen_values = true)]
    pub(crate) segment: Option<String>,

    /// Search length in milliseconds.
    #[arg(
        long,
        value_name = "MS",
        allow_hyphen_values = true,
        requires = "segment"
    )]
    pub(crate) search: Option<String>,

    /// Overlap length in milliseconds.
    #[arg(
        long,
        value_name = "MS",
        allow_hyphen_values = true,
        requires = "search"
    )]
    pub(crate) overlap: Option<String>,

    /// Output WAV file to create.
    #[arg(short = 'o', long = "output", value_name = "FILE")]
    pub(crate) output: PathBuf,

    /// Sample-processing backend to request.
    #[arg(long, value_name = "BACKEND", default_value = "scalar", value_parser = parse_backend)]
    pub(crate) backend: auralis::BackendKind,
}

#[derive(Debug, Args)]
pub(crate) struct PitchArgs {
    /// PCM16 WAV input file to read.
    pub(crate) input: PathBuf,

    /// Pitch shift in cents.
    #[arg(value_name = "CENTS", allow_hyphen_values = true)]
    pub(crate) cents: String,

    /// Prefer quicker search.
    #[arg(long)]
    pub(crate) quick: bool,

    /// Segment length in milliseconds.
    #[arg(long, value_name = "MS", allow_hyphen_values = true)]
    pub(crate) segment: Option<String>,

    /// Search length in milliseconds.
    #[arg(
        long,
        value_name = "MS",
        allow_hyphen_values = true,
        requires = "segment"
    )]
    pub(crate) search: Option<String>,

    /// Overlap length in milliseconds.
    #[arg(
        long,
        value_name = "MS",
        allow_hyphen_values = true,
        requires = "search"
    )]
    pub(crate) overlap: Option<String>,

    /// Output WAV file to create.
    #[arg(short = 'o', long = "output", value_name = "FILE")]
    pub(crate) output: PathBuf,

    /// Sample-processing backend to request.
    #[arg(long, value_name = "BACKEND", default_value = "scalar", value_parser = parse_backend)]
    pub(crate) backend: auralis::BackendKind,
}

#[derive(Debug, Args)]
pub(crate) struct BassArgs {
    /// PCM16 WAV input file to read.
    pub(crate) input: PathBuf,

    /// Shelf gain in dB.
    #[arg(value_name = "DB", allow_hyphen_values = true)]
    pub(crate) gain: String,

    /// Shelf frequency in Hz.
    #[arg(long, value_name = "HZ", default_value = "100")]
    pub(crate) frequency: String,

    /// Shelf width, for example `0.5s`, `0.707q`, or `1o`.
    #[arg(long, value_name = "WIDTH", default_value = "0.5s")]
    pub(crate) width: String,

    /// Output WAV file to create.
    #[arg(short = 'o', long = "output", value_name = "FILE")]
    pub(crate) output: PathBuf,

    /// Sample-processing backend to request.
    #[arg(long, value_name = "BACKEND", default_value = "scalar", value_parser = parse_backend)]
    pub(crate) backend: auralis::BackendKind,
}

#[derive(Debug, Args)]
pub(crate) struct TrebleArgs {
    /// PCM16 WAV input file to read.
    pub(crate) input: PathBuf,

    /// Shelf gain in dB.
    #[arg(value_name = "DB", allow_hyphen_values = true)]
    pub(crate) gain: String,

    /// Shelf frequency in Hz.
    #[arg(long, value_name = "HZ", default_value = "3000")]
    pub(crate) frequency: String,

    /// Shelf width, for example `0.5s`, `0.707q`, or `1o`.
    #[arg(long, value_name = "WIDTH", default_value = "0.5s")]
    pub(crate) width: String,

    /// Output WAV file to create.
    #[arg(short = 'o', long = "output", value_name = "FILE")]
    pub(crate) output: PathBuf,

    /// Sample-processing backend to request.
    #[arg(long, value_name = "BACKEND", default_value = "scalar", value_parser = parse_backend)]
    pub(crate) backend: auralis::BackendKind,
}

#[derive(Debug, Args)]
pub(crate) struct EqualizerArgs {
    /// PCM16 WAV input file to read.
    pub(crate) input: PathBuf,

    /// Center frequency in Hz.
    #[arg(long, value_name = "HZ")]
    pub(crate) frequency: String,

    /// Band width, for example `500h`, `0.707q`, or `1o`.
    #[arg(long, value_name = "WIDTH")]
    pub(crate) width: String,

    /// Band gain in dB.
    #[arg(long, value_name = "DB", allow_hyphen_values = true)]
    pub(crate) gain: String,

    /// Output WAV file to create.
    #[arg(short = 'o', long = "output", value_name = "FILE")]
    pub(crate) output: PathBuf,

    /// Sample-processing backend to request.
    #[arg(long, value_name = "BACKEND", default_value = "scalar", value_parser = parse_backend)]
    pub(crate) backend: auralis::BackendKind,
}

#[derive(Debug, Args)]
pub(crate) struct PoleFilterArgs {
    /// PCM16 WAV input file to read.
    pub(crate) input: PathBuf,

    /// Filter frequency in Hz.
    #[arg(long, value_name = "HZ")]
    pub(crate) frequency: String,

    /// Optional filter width, for example `500h`, `0.707q`, or `1o`.
    #[arg(long, value_name = "WIDTH")]
    pub(crate) width: Option<String>,

    /// Pole count for simple one-pole or two-pole forms.
    #[arg(long, value_name = "1|2", value_parser = parse_filter_poles)]
    pub(crate) poles: Option<u8>,

    /// Output WAV file to create.
    #[arg(short = 'o', long = "output", value_name = "FILE")]
    pub(crate) output: PathBuf,

    /// Sample-processing backend to request.
    #[arg(long, value_name = "BACKEND", default_value = "scalar", value_parser = parse_backend)]
    pub(crate) backend: auralis::BackendKind,
}

#[derive(Debug, Args)]
pub(crate) struct BandArgs {
    /// PCM16 WAV input file to read.
    pub(crate) input: PathBuf,

    /// Filter frequency in Hz.
    #[arg(long, value_name = "HZ")]
    pub(crate) frequency: String,

    /// Optional filter width, for example `500h`, `0.707q`, or `1o`.
    #[arg(long, value_name = "WIDTH")]
    pub(crate) width: Option<String>,

    /// Use the unpitched noise mode.
    #[arg(long)]
    pub(crate) unpitched: bool,

    /// Output WAV file to create.
    #[arg(short = 'o', long = "output", value_name = "FILE")]
    pub(crate) output: PathBuf,

    /// Sample-processing backend to request.
    #[arg(long, value_name = "BACKEND", default_value = "scalar", value_parser = parse_backend)]
    pub(crate) backend: auralis::BackendKind,
}

#[derive(Debug, Args)]
pub(crate) struct BandPassArgs {
    /// PCM16 WAV input file to read.
    pub(crate) input: PathBuf,

    /// Filter frequency in Hz.
    #[arg(long, value_name = "HZ")]
    pub(crate) frequency: String,

    /// Filter width, for example `500h`, `0.707q`, or `1o`.
    #[arg(long, value_name = "WIDTH")]
    pub(crate) width: String,

    /// Use constant-skirt-gain mode.
    #[arg(long)]
    pub(crate) constant_skirt: bool,

    /// Output WAV file to create.
    #[arg(short = 'o', long = "output", value_name = "FILE")]
    pub(crate) output: PathBuf,

    /// Sample-processing backend to request.
    #[arg(long, value_name = "BACKEND", default_value = "scalar", value_parser = parse_backend)]
    pub(crate) backend: auralis::BackendKind,
}

#[derive(Debug, Args)]
pub(crate) struct BandRejectArgs {
    /// PCM16 WAV input file to read.
    pub(crate) input: PathBuf,

    /// Filter frequency in Hz.
    #[arg(long, value_name = "HZ")]
    pub(crate) frequency: String,

    /// Filter width, for example `500h`, `0.707q`, or `1o`.
    #[arg(long, value_name = "WIDTH")]
    pub(crate) width: String,

    /// Output WAV file to create.
    #[arg(short = 'o', long = "output", value_name = "FILE")]
    pub(crate) output: PathBuf,

    /// Sample-processing backend to request.
    #[arg(long, value_name = "BACKEND", default_value = "scalar", value_parser = parse_backend)]
    pub(crate) backend: auralis::BackendKind,
}

#[derive(Debug, Args)]
pub(crate) struct FadeArgs {
    /// PCM16 WAV input file to read.
    pub(crate) input: PathBuf,

    /// Fade-in length in frames, for example `24000` or `24000f`.
    #[arg(long = "in", value_name = "FRAMES", default_value = "0")]
    pub(crate) fade_in: String,

    /// Fade-out length in frames, for example `24000` or `24000f`.
    #[arg(long = "out", value_name = "FRAMES")]
    pub(crate) fade_out: Option<String>,

    /// Fade curve family: linear, quarter-sine, half-sine, log, or parabola.
    #[arg(long, value_name = "CURVE", default_value = "linear")]
    pub(crate) curve: String,

    /// Output WAV file to create.
    #[arg(short = 'o', long = "output", value_name = "FILE")]
    pub(crate) output: PathBuf,

    /// Sample-processing backend to request.
    #[arg(long, value_name = "BACKEND", default_value = "scalar", value_parser = parse_backend)]
    pub(crate) backend: auralis::BackendKind,
}

#[derive(Debug, Args)]
pub(crate) struct DelayArgs {
    /// PCM16 WAV input file to read.
    pub(crate) input: PathBuf,

    /// Delay position such as `2s`, `0.25`, or `+1s`; repeat per channel.
    #[arg(long = "position", value_name = "POSITION", required = true)]
    pub(crate) positions: Vec<String>,

    /// Output WAV file to create.
    #[arg(short = 'o', long = "output", value_name = "FILE")]
    pub(crate) output: PathBuf,

    /// Sample-processing backend to request.
    #[arg(long, value_name = "BACKEND", default_value = "scalar", value_parser = parse_backend)]
    pub(crate) backend: auralis::BackendKind,
}

#[derive(Debug, Args)]
pub(crate) struct PadArgs {
    /// PCM16 WAV input file to read.
    pub(crate) input: PathBuf,

    /// Silence to prepend, in frames.
    #[arg(long, value_name = "FRAMES", default_value = "0")]
    pub(crate) start: String,

    /// Silence to append, in frames.
    #[arg(long, value_name = "FRAMES", default_value = "0")]
    pub(crate) end: String,

    /// Positioned silence as `FRAMES@POSITION`; repeat for multiple inserts.
    #[arg(long = "at", value_name = "FRAMES@POSITION")]
    pub(crate) positioned: Vec<String>,

    /// Output WAV file to create.
    #[arg(short = 'o', long = "output", value_name = "FILE")]
    pub(crate) output: PathBuf,

    /// Sample-processing backend to request.
    #[arg(long, value_name = "BACKEND", default_value = "scalar", value_parser = parse_backend)]
    pub(crate) backend: auralis::BackendKind,
}

#[derive(Debug, Args)]
pub(crate) struct RepeatArgs {
    /// PCM16 WAV input file to read.
    pub(crate) input: PathBuf,

    /// Number of extra copies to append.
    #[arg(value_name = "COUNT", default_value = "1")]
    pub(crate) count: String,

    /// Output WAV file to create.
    #[arg(short = 'o', long = "output", value_name = "FILE")]
    pub(crate) output: PathBuf,

    /// Sample-processing backend to request.
    #[arg(long, value_name = "BACKEND", default_value = "scalar", value_parser = parse_backend)]
    pub(crate) backend: auralis::BackendKind,
}

#[derive(Debug, Args)]
pub(crate) struct DownsampleArgs {
    /// PCM16 WAV input file to read.
    pub(crate) input: PathBuf,

    /// Integer downsample factor.
    #[arg(value_name = "FACTOR", default_value = "2")]
    pub(crate) factor: String,

    /// Output WAV file to create.
    #[arg(short = 'o', long = "output", value_name = "FILE")]
    pub(crate) output: PathBuf,

    /// Sample-processing backend to request.
    #[arg(long, value_name = "BACKEND", default_value = "scalar", value_parser = parse_backend)]
    pub(crate) backend: auralis::BackendKind,
}

#[derive(Debug, Args)]
pub(crate) struct UpsampleArgs {
    /// PCM16 WAV input file to read.
    pub(crate) input: PathBuf,

    /// Integer upsample factor.
    #[arg(value_name = "FACTOR", default_value = "2")]
    pub(crate) factor: String,

    /// Output WAV file to create.
    #[arg(short = 'o', long = "output", value_name = "FILE")]
    pub(crate) output: PathBuf,

    /// Sample-processing backend to request.
    #[arg(long, value_name = "BACKEND", default_value = "scalar", value_parser = parse_backend)]
    pub(crate) backend: auralis::BackendKind,
}

#[derive(Debug, Args)]
pub(crate) struct HilbertArgs {
    /// PCM16 WAV input file to read.
    pub(crate) input: PathBuf,

    /// Optional odd FIR tap count.
    #[arg(long, value_name = "TAPS")]
    pub(crate) taps: Option<String>,

    /// Output WAV file to create.
    #[arg(short = 'o', long = "output", value_name = "FILE")]
    pub(crate) output: PathBuf,

    /// Sample-processing backend to request.
    #[arg(long, value_name = "BACKEND", default_value = "scalar", value_parser = parse_backend)]
    pub(crate) backend: auralis::BackendKind,
}

#[derive(Debug, Args)]
pub(crate) struct LoudnessArgs {
    /// PCM16 WAV input file to read.
    pub(crate) input: PathBuf,

    /// Gain in dB.
    #[arg(
        long,
        value_name = "DB",
        default_value = "-10",
        allow_hyphen_values = true
    )]
    pub(crate) gain: String,

    /// Reference level in dB.
    #[arg(
        long,
        value_name = "DB",
        default_value = "65",
        allow_hyphen_values = true
    )]
    pub(crate) reference: String,

    /// Number of FIR half-points.
    #[arg(long, value_name = "N", default_value = "1023")]
    pub(crate) half_points: String,

    /// Output WAV file to create.
    #[arg(short = 'o', long = "output", value_name = "FILE")]
    pub(crate) output: PathBuf,

    /// Sample-processing backend to request.
    #[arg(long, value_name = "BACKEND", default_value = "scalar", value_parser = parse_backend)]
    pub(crate) backend: auralis::BackendKind,
}

#[derive(Debug, Args)]
pub(crate) struct DitherArgs {
    /// PCM16 WAV input file to read.
    pub(crate) input: PathBuf,

    /// Use sloped TPDF dither.
    #[arg(long)]
    pub(crate) sloped: bool,

    /// Noise-shaping filter to apply.
    #[arg(long = "noise-shape", value_name = "SHAPE", value_parser = ["shibata"])]
    pub(crate) noise_shape: Option<String>,

    /// Target precision in bits.
    #[arg(long, value_name = "BITS", default_value = "16")]
    pub(crate) precision: String,

    /// Output WAV file to create.
    #[arg(short = 'o', long = "output", value_name = "FILE")]
    pub(crate) output: PathBuf,

    /// Sample-processing backend to request.
    #[arg(long, value_name = "BACKEND", default_value = "scalar", value_parser = parse_backend)]
    pub(crate) backend: auralis::BackendKind,
}

#[derive(Debug, Args)]
pub(crate) struct ReverbArgs {
    /// PCM16 WAV input file to read.
    pub(crate) input: PathBuf,

    /// Output only the wet reverberated signal.
    #[arg(long = "wet-only")]
    pub(crate) wet_only: bool,

    /// Reverberance percentage.
    #[arg(long, value_name = "PERCENT", default_value = "50")]
    pub(crate) reverberance: String,

    /// High-frequency damping percentage.
    #[arg(long = "hf-damping", value_name = "PERCENT", default_value = "50")]
    pub(crate) hf_damping: String,

    /// Room scale percentage.
    #[arg(long = "room-scale", value_name = "PERCENT", default_value = "100")]
    pub(crate) room_scale: String,

    /// Stereo depth percentage.
    #[arg(long = "stereo-depth", value_name = "PERCENT", default_value = "100")]
    pub(crate) stereo_depth: String,

    /// Pre-delay in milliseconds.
    #[arg(long = "pre-delay", value_name = "MS", default_value = "0")]
    pub(crate) pre_delay: String,

    /// Wet gain in dB.
    #[arg(
        long = "wet-gain",
        value_name = "DB",
        default_value = "0",
        allow_hyphen_values = true
    )]
    pub(crate) wet_gain: String,

    /// Output WAV file to create.
    #[arg(short = 'o', long = "output", value_name = "FILE")]
    pub(crate) output: PathBuf,

    /// Sample-processing backend to request.
    #[arg(long, value_name = "BACKEND", default_value = "scalar", value_parser = parse_backend)]
    pub(crate) backend: auralis::BackendKind,
}

#[derive(Debug, Args)]
pub(crate) struct StretchArgs {
    /// PCM16 WAV input file to read.
    pub(crate) input: PathBuf,

    /// Stretch factor.
    #[arg(value_name = "FACTOR", default_value = "1", allow_hyphen_values = true)]
    pub(crate) factor: String,

    /// Analysis window length in milliseconds.
    #[arg(
        long,
        value_name = "MS",
        default_value = "20",
        allow_hyphen_values = true
    )]
    pub(crate) window: String,

    /// Fade shape: linear, sqrt, half, or quarter.
    #[arg(long, value_name = "SHAPE", default_value = "linear")]
    pub(crate) fade: String,

    /// Window shift ratio.
    #[arg(long, value_name = "RATIO", allow_hyphen_values = true)]
    pub(crate) shift: Option<String>,

    /// Cross-fade ratio.
    #[arg(
        long,
        value_name = "RATIO",
        allow_hyphen_values = true,
        requires = "shift"
    )]
    pub(crate) fading: Option<String>,

    /// Output WAV file to create.
    #[arg(short = 'o', long = "output", value_name = "FILE")]
    pub(crate) output: PathBuf,

    /// Sample-processing backend to request.
    #[arg(long, value_name = "BACKEND", default_value = "scalar", value_parser = parse_backend)]
    pub(crate) backend: auralis::BackendKind,
}

macro_rules! combine_recipe_args {
    ($name:ident, $input_doc:literal) => {
        #[derive(Debug, Args)]
        pub(crate) struct $name {
            #[doc = $input_doc]
            #[arg(value_name = "INPUT", num_args = 2..)]
            pub(crate) inputs: Vec<PathBuf>,

            /// Output WAV file to create.
            #[arg(short = 'o', long = "output", value_name = "FILE")]
            pub(crate) output: PathBuf,

            /// Sample-processing backend to request.
            #[arg(long, value_name = "BACKEND", default_value = "scalar", value_parser = parse_backend)]
            pub(crate) backend: auralis::BackendKind,
        }
    };
}

combine_recipe_args!(MixArgs, "PCM16 WAV input files to mix.");
combine_recipe_args!(ConcatArgs, "PCM16 WAV input files to concatenate.");
combine_recipe_args!(MixPowerArgs, "PCM16 WAV input files to equal-power mix.");
combine_recipe_args!(
    MergeArgs,
    "PCM16 WAV input files to merge into one multichannel output."
);
combine_recipe_args!(MultiplyArgs, "PCM16 WAV input files to multiply.");

//! Auralis command-line entrypoint.

mod effect_tokens;
mod executor;
mod graph_plan;
mod graph_runtime;
mod recipes;
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

use executor::{
    ConvertOptions, OutputDither, OutputGuard, RenderOptions, convert_audio, run_pipeline,
};
use recipes::{
    StretchRecipeOptions, TimingArgs, normalize_audio, run_band_recipe, run_bandpass_recipe,
    run_chorus_recipe, run_combine_recipe, run_dc_shift_recipe, run_delay_recipe,
    run_dither_recipe, run_echo_recipe, run_effect_recipe, run_fade_recipe, run_hilbert_recipe,
    run_mix_recipe, run_pad_recipe, run_phaser_recipe, run_pitch_recipe, run_pole_filter_recipe,
    run_reverb_recipe, run_saturation_recipe, run_stretch_recipe, run_tempo_recipe,
    run_trim_recipe, run_vol_recipe,
};

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

    /// Normalize one audio file with the typed norm effect.
    Norm {
        /// PCM16 WAV input file to read.
        input: PathBuf,

        /// Peak target in dBFS.
        #[arg(value_name = "DBFS", default_value = "0", allow_hyphen_values = true)]
        level: String,

        /// Output WAV file to create.
        #[arg(short = 'o', long = "output", value_name = "FILE")]
        output: PathBuf,

        /// Sample-processing backend to request.
        #[arg(long, value_name = "BACKEND", default_value = "scalar", value_parser = parse_backend)]
        backend: auralis::BackendKind,
    },

    /// Resample one audio file with the typed rate effect.
    Rate {
        /// PCM16 WAV input file to read.
        input: PathBuf,

        /// Target sample rate in Hz.
        #[arg(value_name = "RATE")]
        sample_rate: String,

        /// Output WAV file to create.
        #[arg(short = 'o', long = "output", value_name = "FILE")]
        output: PathBuf,

        /// Sample-processing backend to request.
        #[arg(long, value_name = "BACKEND", default_value = "scalar", value_parser = parse_backend)]
        backend: auralis::BackendKind,
    },

    /// Convert one audio file to a target channel count.
    Channels {
        /// PCM16 WAV input file to read.
        input: PathBuf,

        /// Target channel count.
        #[arg(value_name = "CHANNELS")]
        count: String,

        /// Output WAV file to create.
        #[arg(short = 'o', long = "output", value_name = "FILE")]
        output: PathBuf,

        /// Sample-processing backend to request.
        #[arg(long, value_name = "BACKEND", default_value = "scalar", value_parser = parse_backend)]
        backend: auralis::BackendKind,
    },

    /// Adjust one audio file by a gain amount.
    Gain {
        /// PCM16 WAV input file to read.
        input: PathBuf,

        /// Gain adjustment in dB, for example `-3` or `-3dB`.
        #[arg(value_name = "DB", allow_hyphen_values = true)]
        db: String,

        /// Output WAV file to create.
        #[arg(short = 'o', long = "output", value_name = "FILE")]
        output: PathBuf,

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

    /// Apply CD/DAT de-emphasis to one audio file.
    Deemph {
        /// PCM16 WAV input file to read.
        input: PathBuf,

        /// Output WAV file to create.
        #[arg(short = 'o', long = "output", value_name = "FILE")]
        output: PathBuf,

        /// Sample-processing backend to request.
        #[arg(long, value_name = "BACKEND", default_value = "scalar", value_parser = parse_backend)]
        backend: auralis::BackendKind,
    },

    /// Apply the stereo headphone-cue filter to one audio file.
    Earwax {
        /// PCM16 WAV input file to read.
        input: PathBuf,

        /// Output WAV file to create.
        #[arg(short = 'o', long = "output", value_name = "FILE")]
        output: PathBuf,

        /// Sample-processing backend to request.
        #[arg(long, value_name = "BACKEND", default_value = "scalar", value_parser = parse_backend)]
        backend: auralis::BackendKind,
    },

    /// Add one or more parallel delayed echoes.
    Echo {
        /// PCM16 WAV input file to read.
        input: PathBuf,

        /// Clean input gain.
        #[arg(
            long = "gain-in",
            value_name = "GAIN",
            default_value = "0.8",
            allow_hyphen_values = true
        )]
        gain_in: String,

        /// Output gain.
        #[arg(
            long = "gain-out",
            value_name = "GAIN",
            default_value = "0.9",
            allow_hyphen_values = true
        )]
        gain_out: String,

        /// Echo tap as `delay_ms,decay`; repeat for multiple taps.
        #[arg(
            long = "tap",
            value_name = "DELAY_MS,DECAY",
            required = true,
            allow_hyphen_values = true
        )]
        taps: Vec<String>,

        /// Output WAV file to create.
        #[arg(short = 'o', long = "output", value_name = "FILE")]
        output: PathBuf,

        /// Sample-processing backend to request.
        #[arg(long, value_name = "BACKEND", default_value = "scalar", value_parser = parse_backend)]
        backend: auralis::BackendKind,
    },

    /// Add one or more cascaded delayed echoes.
    Echos {
        /// PCM16 WAV input file to read.
        input: PathBuf,

        /// Clean input gain.
        #[arg(
            long = "gain-in",
            value_name = "GAIN",
            default_value = "0.8",
            allow_hyphen_values = true
        )]
        gain_in: String,

        /// Output gain.
        #[arg(
            long = "gain-out",
            value_name = "GAIN",
            default_value = "0.9",
            allow_hyphen_values = true
        )]
        gain_out: String,

        /// Echo tap as `delay_ms,decay`; repeat for multiple taps.
        #[arg(
            long = "tap",
            value_name = "DELAY_MS,DECAY",
            required = true,
            allow_hyphen_values = true
        )]
        taps: Vec<String>,

        /// Output WAV file to create.
        #[arg(short = 'o', long = "output", value_name = "FILE")]
        output: PathBuf,

        /// Sample-processing backend to request.
        #[arg(long, value_name = "BACKEND", default_value = "scalar", value_parser = parse_backend)]
        backend: auralis::BackendKind,
    },

    /// Add chorus modulation to one audio file.
    Chorus {
        /// PCM16 WAV input file to read.
        input: PathBuf,

        /// Clean input gain.
        #[arg(
            long = "gain-in",
            value_name = "GAIN",
            default_value = "0.5",
            allow_hyphen_values = true
        )]
        gain_in: String,

        /// Output gain.
        #[arg(
            long = "gain-out",
            value_name = "GAIN",
            default_value = "1",
            allow_hyphen_values = true
        )]
        gain_out: String,

        /// Interpolation mode: none, linear, or quadratic.
        #[arg(long, value_name = "MODE", default_value = "none")]
        interpolation: String,

        /// Default modulation wave: sine or triangle.
        #[arg(long, value_name = "WAVE", default_value = "sine")]
        wave: String,

        /// Chorus stage as `delay_ms,decay,speed_hz,depth_ms[,wave]`; repeat for multiple stages.
        #[arg(long = "stage", value_name = "STAGE", allow_hyphen_values = true)]
        stages: Vec<String>,

        /// Output WAV file to create.
        #[arg(short = 'o', long = "output", value_name = "FILE")]
        output: PathBuf,

        /// Sample-processing backend to request.
        #[arg(long, value_name = "BACKEND", default_value = "scalar", value_parser = parse_backend)]
        backend: auralis::BackendKind,
    },

    /// Add flanger modulation to one audio file.
    Flanger {
        /// PCM16 WAV input file to read.
        input: PathBuf,

        /// Base delay in milliseconds.
        #[arg(
            long,
            value_name = "MS",
            default_value = "0",
            allow_hyphen_values = true
        )]
        delay: String,

        /// Sweep depth in milliseconds.
        #[arg(
            long,
            value_name = "MS",
            default_value = "2",
            allow_hyphen_values = true
        )]
        depth: String,

        /// Regeneration percentage.
        #[arg(
            long,
            value_name = "PERCENT",
            default_value = "0",
            allow_hyphen_values = true
        )]
        regen: String,

        /// Wet width percentage.
        #[arg(
            long,
            value_name = "PERCENT",
            default_value = "71",
            allow_hyphen_values = true
        )]
        width: String,

        /// Modulation speed in Hz.
        #[arg(
            long,
            value_name = "HZ",
            default_value = "0.5",
            allow_hyphen_values = true
        )]
        speed: String,

        /// Modulation wave: sine or triangle.
        #[arg(long, value_name = "WAVE", default_value = "sine")]
        wave: String,

        /// Stereo phase percentage.
        #[arg(
            long,
            value_name = "PERCENT",
            default_value = "25",
            allow_hyphen_values = true
        )]
        phase: String,

        /// Interpolation mode: none, linear, or quadratic.
        #[arg(long, value_name = "MODE", default_value = "linear")]
        interpolation: String,

        /// Output WAV file to create.
        #[arg(short = 'o', long = "output", value_name = "FILE")]
        output: PathBuf,

        /// Sample-processing backend to request.
        #[arg(long, value_name = "BACKEND", default_value = "scalar", value_parser = parse_backend)]
        backend: auralis::BackendKind,
    },

    /// Add phaser modulation to one audio file.
    Phaser {
        /// PCM16 WAV input file to read.
        input: PathBuf,

        /// Clean input gain.
        #[arg(
            long = "gain-in",
            value_name = "GAIN",
            default_value = "0.4",
            allow_hyphen_values = true
        )]
        gain_in: String,

        /// Output gain.
        #[arg(
            long = "gain-out",
            value_name = "GAIN",
            default_value = "0.74",
            allow_hyphen_values = true
        )]
        gain_out: String,

        /// Delay in milliseconds.
        #[arg(
            long,
            value_name = "MS",
            default_value = "3",
            allow_hyphen_values = true
        )]
        delay: String,

        /// Regeneration amount.
        #[arg(
            long,
            value_name = "AMOUNT",
            default_value = "0.4",
            allow_hyphen_values = true
        )]
        regen: String,

        /// Modulation speed in Hz.
        #[arg(
            long,
            value_name = "HZ",
            default_value = "0.5",
            allow_hyphen_values = true
        )]
        speed: String,

        /// Modulation wave: sine or triangle.
        #[arg(long, value_name = "WAVE", default_value = "sine")]
        wave: String,

        /// Interpolation mode: none, linear, or quadratic.
        #[arg(long, value_name = "MODE", default_value = "none")]
        interpolation: String,

        /// Output WAV file to create.
        #[arg(short = 'o', long = "output", value_name = "FILE")]
        output: PathBuf,

        /// Sample-processing backend to request.
        #[arg(long, value_name = "BACKEND", default_value = "scalar", value_parser = parse_backend)]
        backend: auralis::BackendKind,
    },

    /// Extract out-of-phase stereo content.
    Oops {
        /// PCM16 WAV input file to read.
        input: PathBuf,

        /// Output WAV file to create.
        #[arg(short = 'o', long = "output", value_name = "FILE")]
        output: PathBuf,

        /// Sample-processing backend to request.
        #[arg(long, value_name = "BACKEND", default_value = "scalar", value_parser = parse_backend)]
        backend: auralis::BackendKind,
    },

    /// Apply RIAA vinyl playback equalization.
    Riaa {
        /// PCM16 WAV input file to read.
        input: PathBuf,

        /// Output WAV file to create.
        #[arg(short = 'o', long = "output", value_name = "FILE")]
        output: PathBuf,

        /// Sample-processing backend to request.
        #[arg(long, value_name = "BACKEND", default_value = "scalar", value_parser = parse_backend)]
        backend: auralis::BackendKind,
    },

    /// Swap adjacent channel pairs.
    Swap {
        /// PCM16 WAV input file to read.
        input: PathBuf,

        /// Output WAV file to create.
        #[arg(short = 'o', long = "output", value_name = "FILE")]
        output: PathBuf,

        /// Sample-processing backend to request.
        #[arg(long, value_name = "BACKEND", default_value = "scalar", value_parser = parse_backend)]
        backend: auralis::BackendKind,
    },

    /// Enhance sample contrast.
    Contrast {
        /// PCM16 WAV input file to read.
        input: PathBuf,

        /// Contrast amount from 0 to 100.
        #[arg(long, value_name = "AMOUNT", default_value = "75")]
        amount: String,

        /// Output WAV file to create.
        #[arg(short = 'o', long = "output", value_name = "FILE")]
        output: PathBuf,

        /// Sample-processing backend to request.
        #[arg(long, value_name = "BACKEND", default_value = "scalar", value_parser = parse_backend)]
        backend: auralis::BackendKind,
    },

    /// Apply overdrive distortion.
    Overdrive {
        /// PCM16 WAV input file to read.
        input: PathBuf,

        /// Overdrive gain.
        #[arg(
            long,
            value_name = "GAIN",
            default_value = "20",
            allow_hyphen_values = true
        )]
        gain: String,

        /// Overdrive color.
        #[arg(
            long,
            value_name = "COLOR",
            default_value = "20",
            allow_hyphen_values = true
        )]
        color: String,

        /// Output WAV file to create.
        #[arg(short = 'o', long = "output", value_name = "FILE")]
        output: PathBuf,

        /// Sample-processing backend to request.
        #[arg(long, value_name = "BACKEND", default_value = "scalar", value_parser = parse_backend)]
        backend: auralis::BackendKind,
    },

    /// Apply saturation distortion.
    Saturation {
        /// PCM16 WAV input file to read.
        input: PathBuf,

        /// Saturation curve type: tanh, sqrt, or diode.
        #[arg(long = "type", value_name = "TYPE", default_value = "tanh")]
        saturation_type: String,

        /// Wet/dry blend amount.
        #[arg(
            long,
            value_name = "BLEND",
            default_value = "1",
            allow_hyphen_values = true
        )]
        blend: String,

        /// Input offset before saturation.
        #[arg(
            long,
            value_name = "OFFSET",
            default_value = "0",
            allow_hyphen_values = true
        )]
        offset: String,

        /// Curve-specific parameter: drive, color, or threshold.
        #[arg(long, value_name = "VALUE", allow_hyphen_values = true)]
        parameter: Option<String>,

        /// Output WAV file to create.
        #[arg(short = 'o', long = "output", value_name = "FILE")]
        output: PathBuf,

        /// Sample-processing backend to request.
        #[arg(long, value_name = "BACKEND", default_value = "scalar", value_parser = parse_backend)]
        backend: auralis::BackendKind,
    },

    /// Shift the DC level of one audio file.
    #[command(name = "dcshift")]
    DcShift {
        /// PCM16 WAV input file to read.
        input: PathBuf,

        /// DC shift amount.
        #[arg(value_name = "SHIFT", allow_hyphen_values = true)]
        shift: String,

        /// Optional limiter gain.
        #[arg(long, value_name = "GAIN", allow_hyphen_values = true)]
        limiter_gain: Option<String>,

        /// Output WAV file to create.
        #[arg(short = 'o', long = "output", value_name = "FILE")]
        output: PathBuf,

        /// Sample-processing backend to request.
        #[arg(long, value_name = "BACKEND", default_value = "scalar", value_parser = parse_backend)]
        backend: auralis::BackendKind,
    },

    /// Apply SoX-ng volume scaling to one audio file.
    Vol {
        /// PCM16 WAV input file to read.
        input: PathBuf,

        /// Volume gain value, for example `0.5` or `-6dB`.
        #[arg(value_name = "GAIN", allow_hyphen_values = true)]
        gain: String,

        /// Gain interpretation: amplitude, power, or dB.
        #[arg(long = "type", value_name = "TYPE")]
        gain_type: Option<String>,

        /// Optional limiter gain.
        #[arg(long, value_name = "GAIN", allow_hyphen_values = true)]
        limiter_gain: Option<String>,

        /// Output WAV file to create.
        #[arg(short = 'o', long = "output", value_name = "FILE")]
        output: PathBuf,

        /// Sample-processing backend to request.
        #[arg(long, value_name = "BACKEND", default_value = "scalar", value_parser = parse_backend)]
        backend: auralis::BackendKind,
    },

    /// Apply soft volume changes to one audio file.
    #[command(name = "softvol")]
    SoftVol {
        /// PCM16 WAV input file to read.
        input: PathBuf,

        /// Volume multiplier.
        #[arg(
            long,
            value_name = "VOLUME",
            default_value = "1",
            allow_hyphen_values = true
        )]
        volume: String,

        /// Seconds required for volume doubling.
        #[arg(
            long,
            value_name = "SECONDS",
            default_value = "0",
            allow_hyphen_values = true
        )]
        double_time: String,

        /// Extra headroom in dB.
        #[arg(
            long,
            value_name = "DB",
            default_value = "0",
            allow_hyphen_values = true
        )]
        headroom: String,

        /// Output WAV file to create.
        #[arg(short = 'o', long = "output", value_name = "FILE")]
        output: PathBuf,

        /// Sample-processing backend to request.
        #[arg(long, value_name = "BACKEND", default_value = "scalar", value_parser = parse_backend)]
        backend: auralis::BackendKind,
    },

    /// Apply tremolo modulation to one audio file.
    Tremolo {
        /// PCM16 WAV input file to read.
        input: PathBuf,

        /// Modulation speed in Hz.
        #[arg(value_name = "SPEED_HZ", allow_hyphen_values = true)]
        speed: String,

        /// Modulation depth percentage.
        #[arg(
            long,
            value_name = "PERCENT",
            default_value = "40",
            allow_hyphen_values = true
        )]
        depth: String,

        /// Output WAV file to create.
        #[arg(short = 'o', long = "output", value_name = "FILE")]
        output: PathBuf,

        /// Sample-processing backend to request.
        #[arg(long, value_name = "BACKEND", default_value = "scalar", value_parser = parse_backend)]
        backend: auralis::BackendKind,
    },

    /// Change playback speed and sample rate.
    Speed {
        /// PCM16 WAV input file to read.
        input: PathBuf,

        /// Speed factor, or cents with a `c` suffix.
        #[arg(value_name = "FACTOR", allow_hyphen_values = true)]
        factor: String,

        /// Output WAV file to create.
        #[arg(short = 'o', long = "output", value_name = "FILE")]
        output: PathBuf,

        /// Sample-processing backend to request.
        #[arg(long, value_name = "BACKEND", default_value = "scalar", value_parser = parse_backend)]
        backend: auralis::BackendKind,
    },

    /// Change tempo without changing pitch.
    Tempo {
        /// PCM16 WAV input file to read.
        input: PathBuf,

        /// Tempo factor.
        #[arg(value_name = "FACTOR", allow_hyphen_values = true)]
        factor: String,

        /// Prefer quicker search.
        #[arg(long)]
        quick: bool,

        /// Tuning profile: music, speech, or linear.
        #[arg(long, value_name = "PROFILE")]
        profile: Option<String>,

        /// Segment length in milliseconds.
        #[arg(long, value_name = "MS", allow_hyphen_values = true)]
        segment: Option<String>,

        /// Search length in milliseconds.
        #[arg(
            long,
            value_name = "MS",
            allow_hyphen_values = true,
            requires = "segment"
        )]
        search: Option<String>,

        /// Overlap length in milliseconds.
        #[arg(
            long,
            value_name = "MS",
            allow_hyphen_values = true,
            requires = "search"
        )]
        overlap: Option<String>,

        /// Output WAV file to create.
        #[arg(short = 'o', long = "output", value_name = "FILE")]
        output: PathBuf,

        /// Sample-processing backend to request.
        #[arg(long, value_name = "BACKEND", default_value = "scalar", value_parser = parse_backend)]
        backend: auralis::BackendKind,
    },

    /// Shift pitch without changing tempo.
    Pitch {
        /// PCM16 WAV input file to read.
        input: PathBuf,

        /// Pitch shift in cents.
        #[arg(value_name = "CENTS", allow_hyphen_values = true)]
        cents: String,

        /// Prefer quicker search.
        #[arg(long)]
        quick: bool,

        /// Segment length in milliseconds.
        #[arg(long, value_name = "MS", allow_hyphen_values = true)]
        segment: Option<String>,

        /// Search length in milliseconds.
        #[arg(
            long,
            value_name = "MS",
            allow_hyphen_values = true,
            requires = "segment"
        )]
        search: Option<String>,

        /// Overlap length in milliseconds.
        #[arg(
            long,
            value_name = "MS",
            allow_hyphen_values = true,
            requires = "search"
        )]
        overlap: Option<String>,

        /// Output WAV file to create.
        #[arg(short = 'o', long = "output", value_name = "FILE")]
        output: PathBuf,

        /// Sample-processing backend to request.
        #[arg(long, value_name = "BACKEND", default_value = "scalar", value_parser = parse_backend)]
        backend: auralis::BackendKind,
    },

    /// Boost or cut bass frequencies in one audio file.
    Bass {
        /// PCM16 WAV input file to read.
        input: PathBuf,

        /// Shelf gain in dB.
        #[arg(value_name = "DB", allow_hyphen_values = true)]
        gain: String,

        /// Shelf frequency in Hz.
        #[arg(long, value_name = "HZ", default_value = "100")]
        frequency: String,

        /// Shelf width, for example `0.5s`, `0.707q`, or `1o`.
        #[arg(long, value_name = "WIDTH", default_value = "0.5s")]
        width: String,

        /// Output WAV file to create.
        #[arg(short = 'o', long = "output", value_name = "FILE")]
        output: PathBuf,

        /// Sample-processing backend to request.
        #[arg(long, value_name = "BACKEND", default_value = "scalar", value_parser = parse_backend)]
        backend: auralis::BackendKind,
    },

    /// Boost or cut treble frequencies in one audio file.
    Treble {
        /// PCM16 WAV input file to read.
        input: PathBuf,

        /// Shelf gain in dB.
        #[arg(value_name = "DB", allow_hyphen_values = true)]
        gain: String,

        /// Shelf frequency in Hz.
        #[arg(long, value_name = "HZ", default_value = "3000")]
        frequency: String,

        /// Shelf width, for example `0.5s`, `0.707q`, or `1o`.
        #[arg(long, value_name = "WIDTH", default_value = "0.5s")]
        width: String,

        /// Output WAV file to create.
        #[arg(short = 'o', long = "output", value_name = "FILE")]
        output: PathBuf,

        /// Sample-processing backend to request.
        #[arg(long, value_name = "BACKEND", default_value = "scalar", value_parser = parse_backend)]
        backend: auralis::BackendKind,
    },

    /// Apply one peaking equalizer band to one audio file.
    Equalizer {
        /// PCM16 WAV input file to read.
        input: PathBuf,

        /// Center frequency in Hz.
        #[arg(long, value_name = "HZ")]
        frequency: String,

        /// Band width, for example `500h`, `0.707q`, or `1o`.
        #[arg(long, value_name = "WIDTH")]
        width: String,

        /// Band gain in dB.
        #[arg(long, value_name = "DB", allow_hyphen_values = true)]
        gain: String,

        /// Output WAV file to create.
        #[arg(short = 'o', long = "output", value_name = "FILE")]
        output: PathBuf,

        /// Sample-processing backend to request.
        #[arg(long, value_name = "BACKEND", default_value = "scalar", value_parser = parse_backend)]
        backend: auralis::BackendKind,
    },

    /// Apply an all-pass filter to one audio file.
    #[command(name = "allpass")]
    AllPass {
        /// PCM16 WAV input file to read.
        input: PathBuf,

        /// Filter frequency in Hz.
        #[arg(long, value_name = "HZ")]
        frequency: String,

        /// Filter width, for example `500h`, `0.707q`, or `1o`.
        #[arg(long, value_name = "WIDTH")]
        width: Option<String>,

        /// Pole count for simple one-pole or two-pole forms.
        #[arg(long, value_name = "1|2", value_parser = parse_filter_poles)]
        poles: Option<u8>,

        /// Output WAV file to create.
        #[arg(short = 'o', long = "output", value_name = "FILE")]
        output: PathBuf,

        /// Sample-processing backend to request.
        #[arg(long, value_name = "BACKEND", default_value = "scalar", value_parser = parse_backend)]
        backend: auralis::BackendKind,
    },

    /// Apply a resonator band-pass filter to one audio file.
    Band {
        /// PCM16 WAV input file to read.
        input: PathBuf,

        /// Filter frequency in Hz.
        #[arg(long, value_name = "HZ")]
        frequency: String,

        /// Optional filter width, for example `500h`, `0.707q`, or `1o`.
        #[arg(long, value_name = "WIDTH")]
        width: Option<String>,

        /// Use the unpitched noise mode.
        #[arg(long)]
        unpitched: bool,

        /// Output WAV file to create.
        #[arg(short = 'o', long = "output", value_name = "FILE")]
        output: PathBuf,

        /// Sample-processing backend to request.
        #[arg(long, value_name = "BACKEND", default_value = "scalar", value_parser = parse_backend)]
        backend: auralis::BackendKind,
    },

    /// Apply an RBJ band-pass filter to one audio file.
    #[command(name = "bandpass")]
    BandPass {
        /// PCM16 WAV input file to read.
        input: PathBuf,

        /// Filter frequency in Hz.
        #[arg(long, value_name = "HZ")]
        frequency: String,

        /// Filter width, for example `500h`, `0.707q`, or `1o`.
        #[arg(long, value_name = "WIDTH")]
        width: String,

        /// Use constant-skirt-gain mode.
        #[arg(long)]
        constant_skirt: bool,

        /// Output WAV file to create.
        #[arg(short = 'o', long = "output", value_name = "FILE")]
        output: PathBuf,

        /// Sample-processing backend to request.
        #[arg(long, value_name = "BACKEND", default_value = "scalar", value_parser = parse_backend)]
        backend: auralis::BackendKind,
    },

    /// Apply an RBJ band-reject filter to one audio file.
    #[command(name = "bandreject")]
    BandReject {
        /// PCM16 WAV input file to read.
        input: PathBuf,

        /// Filter frequency in Hz.
        #[arg(long, value_name = "HZ")]
        frequency: String,

        /// Filter width, for example `500h`, `0.707q`, or `1o`.
        #[arg(long, value_name = "WIDTH")]
        width: String,

        /// Output WAV file to create.
        #[arg(short = 'o', long = "output", value_name = "FILE")]
        output: PathBuf,

        /// Sample-processing backend to request.
        #[arg(long, value_name = "BACKEND", default_value = "scalar", value_parser = parse_backend)]
        backend: auralis::BackendKind,
    },

    /// Apply a high-pass filter to one audio file.
    #[command(name = "highpass")]
    HighPass {
        /// PCM16 WAV input file to read.
        input: PathBuf,

        /// Filter cutoff frequency in Hz.
        #[arg(long, value_name = "HZ")]
        frequency: String,

        /// Optional filter width, for example `500h`, `0.707q`, or `1o`.
        #[arg(long, value_name = "WIDTH")]
        width: Option<String>,

        /// Pole count for simple one-pole or two-pole forms.
        #[arg(long, value_name = "1|2", value_parser = parse_filter_poles)]
        poles: Option<u8>,

        /// Output WAV file to create.
        #[arg(short = 'o', long = "output", value_name = "FILE")]
        output: PathBuf,

        /// Sample-processing backend to request.
        #[arg(long, value_name = "BACKEND", default_value = "scalar", value_parser = parse_backend)]
        backend: auralis::BackendKind,
    },

    /// Apply a low-pass filter to one audio file.
    #[command(name = "lowpass")]
    LowPass {
        /// PCM16 WAV input file to read.
        input: PathBuf,

        /// Filter cutoff frequency in Hz.
        #[arg(long, value_name = "HZ")]
        frequency: String,

        /// Optional filter width, for example `500h`, `0.707q`, or `1o`.
        #[arg(long, value_name = "WIDTH")]
        width: Option<String>,

        /// Pole count for simple one-pole or two-pole forms.
        #[arg(long, value_name = "1|2", value_parser = parse_filter_poles)]
        poles: Option<u8>,

        /// Output WAV file to create.
        #[arg(short = 'o', long = "output", value_name = "FILE")]
        output: PathBuf,

        /// Sample-processing backend to request.
        #[arg(long, value_name = "BACKEND", default_value = "scalar", value_parser = parse_backend)]
        backend: auralis::BackendKind,
    },

    /// Fade one audio file in or out.
    Fade {
        /// PCM16 WAV input file to read.
        input: PathBuf,

        /// Fade-in length in frames, for example `24000` or `24000f`.
        #[arg(long = "in", value_name = "FRAMES", default_value = "0")]
        fade_in: String,

        /// Fade-out length in frames, for example `24000` or `24000f`.
        #[arg(long = "out", value_name = "FRAMES")]
        fade_out: Option<String>,

        /// Fade curve family: linear, quarter-sine, half-sine, log, or parabola.
        #[arg(long, value_name = "CURVE", default_value = "linear")]
        curve: String,

        /// Output WAV file to create.
        #[arg(short = 'o', long = "output", value_name = "FILE")]
        output: PathBuf,

        /// Sample-processing backend to request.
        #[arg(long, value_name = "BACKEND", default_value = "scalar", value_parser = parse_backend)]
        backend: auralis::BackendKind,
    },

    /// Delay one audio file by per-channel positions.
    Delay {
        /// PCM16 WAV input file to read.
        input: PathBuf,

        /// Delay position such as `2s`, `0.25`, or `+1s`; repeat per channel.
        #[arg(long = "position", value_name = "POSITION", required = true)]
        positions: Vec<String>,

        /// Output WAV file to create.
        #[arg(short = 'o', long = "output", value_name = "FILE")]
        output: PathBuf,

        /// Sample-processing backend to request.
        #[arg(long, value_name = "BACKEND", default_value = "scalar", value_parser = parse_backend)]
        backend: auralis::BackendKind,
    },

    /// Add silence before, after, or inside one audio file.
    Pad {
        /// PCM16 WAV input file to read.
        input: PathBuf,

        /// Silence to prepend, in frames.
        #[arg(long, value_name = "FRAMES", default_value = "0")]
        start: String,

        /// Silence to append, in frames.
        #[arg(long, value_name = "FRAMES", default_value = "0")]
        end: String,

        /// Positioned silence as `FRAMES@POSITION`; repeat for multiple inserts.
        #[arg(long = "at", value_name = "FRAMES@POSITION")]
        positioned: Vec<String>,

        /// Output WAV file to create.
        #[arg(short = 'o', long = "output", value_name = "FILE")]
        output: PathBuf,

        /// Sample-processing backend to request.
        #[arg(long, value_name = "BACKEND", default_value = "scalar", value_parser = parse_backend)]
        backend: auralis::BackendKind,
    },

    /// Append finite copies of one audio file.
    Repeat {
        /// PCM16 WAV input file to read.
        input: PathBuf,

        /// Number of extra copies to append.
        #[arg(value_name = "COUNT", default_value = "1")]
        count: String,

        /// Output WAV file to create.
        #[arg(short = 'o', long = "output", value_name = "FILE")]
        output: PathBuf,

        /// Sample-processing backend to request.
        #[arg(long, value_name = "BACKEND", default_value = "scalar", value_parser = parse_backend)]
        backend: auralis::BackendKind,
    },

    /// Keep every Nth sample from one audio file.
    Downsample {
        /// PCM16 WAV input file to read.
        input: PathBuf,

        /// Integer downsample factor.
        #[arg(value_name = "FACTOR", default_value = "2")]
        factor: String,

        /// Output WAV file to create.
        #[arg(short = 'o', long = "output", value_name = "FILE")]
        output: PathBuf,

        /// Sample-processing backend to request.
        #[arg(long, value_name = "BACKEND", default_value = "scalar", value_parser = parse_backend)]
        backend: auralis::BackendKind,
    },

    /// Insert zero samples between input samples.
    Upsample {
        /// PCM16 WAV input file to read.
        input: PathBuf,

        /// Integer upsample factor.
        #[arg(value_name = "FACTOR", default_value = "2")]
        factor: String,

        /// Output WAV file to create.
        #[arg(short = 'o', long = "output", value_name = "FILE")]
        output: PathBuf,

        /// Sample-processing backend to request.
        #[arg(long, value_name = "BACKEND", default_value = "scalar", value_parser = parse_backend)]
        backend: auralis::BackendKind,
    },

    /// Apply Hilbert transform phase shifting.
    Hilbert {
        /// PCM16 WAV input file to read.
        input: PathBuf,

        /// Optional odd FIR tap count.
        #[arg(long, value_name = "TAPS")]
        taps: Option<String>,

        /// Output WAV file to create.
        #[arg(short = 'o', long = "output", value_name = "FILE")]
        output: PathBuf,

        /// Sample-processing backend to request.
        #[arg(long, value_name = "BACKEND", default_value = "scalar", value_parser = parse_backend)]
        backend: auralis::BackendKind,
    },

    /// Apply loudness compensation filtering.
    Loudness {
        /// PCM16 WAV input file to read.
        input: PathBuf,

        /// Gain in dB.
        #[arg(
            long,
            value_name = "DB",
            default_value = "-10",
            allow_hyphen_values = true
        )]
        gain: String,

        /// Reference level in dB.
        #[arg(
            long,
            value_name = "DB",
            default_value = "65",
            allow_hyphen_values = true
        )]
        reference: String,

        /// Number of FIR half-points.
        #[arg(long, value_name = "N", default_value = "1023")]
        half_points: String,

        /// Output WAV file to create.
        #[arg(short = 'o', long = "output", value_name = "FILE")]
        output: PathBuf,

        /// Sample-processing backend to request.
        #[arg(long, value_name = "BACKEND", default_value = "scalar", value_parser = parse_backend)]
        backend: auralis::BackendKind,
    },

    /// Apply deterministic dithering.
    Dither {
        /// PCM16 WAV input file to read.
        input: PathBuf,

        /// Use sloped TPDF dither.
        #[arg(long)]
        sloped: bool,

        /// Noise-shaping filter to apply.
        #[arg(long = "noise-shape", value_name = "SHAPE", value_parser = ["shibata"])]
        noise_shape: Option<String>,

        /// Target precision in bits.
        #[arg(long, value_name = "BITS", default_value = "16")]
        precision: String,

        /// Output WAV file to create.
        #[arg(short = 'o', long = "output", value_name = "FILE")]
        output: PathBuf,

        /// Sample-processing backend to request.
        #[arg(long, value_name = "BACKEND", default_value = "scalar", value_parser = parse_backend)]
        backend: auralis::BackendKind,
    },

    /// Apply stereo reverberation to one audio file.
    Reverb {
        /// PCM16 WAV input file to read.
        input: PathBuf,

        /// Output only the wet reverberated signal.
        #[arg(long = "wet-only")]
        wet_only: bool,

        /// Reverberance percentage.
        #[arg(long, value_name = "PERCENT", default_value = "50")]
        reverberance: String,

        /// High-frequency damping percentage.
        #[arg(long = "hf-damping", value_name = "PERCENT", default_value = "50")]
        hf_damping: String,

        /// Room scale percentage.
        #[arg(long = "room-scale", value_name = "PERCENT", default_value = "100")]
        room_scale: String,

        /// Stereo depth percentage.
        #[arg(long = "stereo-depth", value_name = "PERCENT", default_value = "100")]
        stereo_depth: String,

        /// Pre-delay in milliseconds.
        #[arg(long = "pre-delay", value_name = "MS", default_value = "0")]
        pre_delay: String,

        /// Wet gain in dB.
        #[arg(
            long = "wet-gain",
            value_name = "DB",
            default_value = "0",
            allow_hyphen_values = true
        )]
        wet_gain: String,

        /// Output WAV file to create.
        #[arg(short = 'o', long = "output", value_name = "FILE")]
        output: PathBuf,

        /// Sample-processing backend to request.
        #[arg(long, value_name = "BACKEND", default_value = "scalar", value_parser = parse_backend)]
        backend: auralis::BackendKind,
    },

    /// Change duration with basic windowed stretching.
    Stretch {
        /// PCM16 WAV input file to read.
        input: PathBuf,

        /// Stretch factor.
        #[arg(value_name = "FACTOR", default_value = "1", allow_hyphen_values = true)]
        factor: String,

        /// Analysis window length in milliseconds.
        #[arg(
            long,
            value_name = "MS",
            default_value = "20",
            allow_hyphen_values = true
        )]
        window: String,

        /// Fade shape: linear, sqrt, half, or quarter.
        #[arg(long, value_name = "SHAPE", default_value = "linear")]
        fade: String,

        /// Window shift ratio.
        #[arg(long, value_name = "RATIO", allow_hyphen_values = true)]
        shift: Option<String>,

        /// Cross-fade ratio.
        #[arg(
            long,
            value_name = "RATIO",
            allow_hyphen_values = true,
            requires = "shift"
        )]
        fading: Option<String>,

        /// Output WAV file to create.
        #[arg(short = 'o', long = "output", value_name = "FILE")]
        output: PathBuf,

        /// Sample-processing backend to request.
        #[arg(long, value_name = "BACKEND", default_value = "scalar", value_parser = parse_backend)]
        backend: auralis::BackendKind,
    },

    /// Mix two or more audio files into one output.
    Mix {
        /// PCM16 WAV input files to mix.
        #[arg(value_name = "INPUT", num_args = 2..)]
        inputs: Vec<PathBuf>,

        /// Output WAV file to create.
        #[arg(short = 'o', long = "output", value_name = "FILE")]
        output: PathBuf,

        /// Sample-processing backend to request.
        #[arg(long, value_name = "BACKEND", default_value = "scalar", value_parser = parse_backend)]
        backend: auralis::BackendKind,
    },

    /// Concatenate two or more audio files end-to-end.
    Concat {
        /// PCM16 WAV input files to concatenate.
        #[arg(value_name = "INPUT", num_args = 2..)]
        inputs: Vec<PathBuf>,

        /// Output WAV file to create.
        #[arg(short = 'o', long = "output", value_name = "FILE")]
        output: PathBuf,

        /// Sample-processing backend to request.
        #[arg(long, value_name = "BACKEND", default_value = "scalar", value_parser = parse_backend)]
        backend: auralis::BackendKind,
    },

    /// Mix two or more audio files using equal-power scaling.
    MixPower {
        /// PCM16 WAV input files to equal-power mix.
        #[arg(value_name = "INPUT", num_args = 2..)]
        inputs: Vec<PathBuf>,

        /// Output WAV file to create.
        #[arg(short = 'o', long = "output", value_name = "FILE")]
        output: PathBuf,

        /// Sample-processing backend to request.
        #[arg(long, value_name = "BACKEND", default_value = "scalar", value_parser = parse_backend)]
        backend: auralis::BackendKind,
    },

    /// Merge all channels from two or more audio files.
    Merge {
        /// PCM16 WAV input files to merge into one multichannel output.
        #[arg(value_name = "INPUT", num_args = 2..)]
        inputs: Vec<PathBuf>,

        /// Output WAV file to create.
        #[arg(short = 'o', long = "output", value_name = "FILE")]
        output: PathBuf,

        /// Sample-processing backend to request.
        #[arg(long, value_name = "BACKEND", default_value = "scalar", value_parser = parse_backend)]
        backend: auralis::BackendKind,
    },

    /// Multiply corresponding samples from two or more audio files.
    Multiply {
        /// PCM16 WAV input files to multiply.
        #[arg(value_name = "INPUT", num_args = 2..)]
        inputs: Vec<PathBuf>,

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
        Command::Norm {
            input,
            level,
            output,
            backend,
        } => run_effect_recipe(&input, &output, backend, ["norm", level.as_str()]),
        Command::Rate {
            input,
            sample_rate,
            output,
            backend,
        } => run_effect_recipe(&input, &output, backend, ["rate", sample_rate.as_str()]),
        Command::Channels {
            input,
            count,
            output,
            backend,
        } => run_effect_recipe(&input, &output, backend, ["channels", count.as_str()]),
        Command::Gain {
            input,
            db,
            output,
            backend,
        } => run_effect_recipe(&input, &output, backend, ["gain", db.as_str()]),
        Command::Reverse {
            input,
            output,
            backend,
        } => run_effect_recipe(&input, &output, backend, ["reverse"]),
        Command::Deemph {
            input,
            output,
            backend,
        } => run_effect_recipe(&input, &output, backend, ["deemph"]),
        Command::Earwax {
            input,
            output,
            backend,
        } => run_effect_recipe(&input, &output, backend, ["earwax"]),
        Command::Echo {
            input,
            gain_in,
            gain_out,
            taps,
            output,
            backend,
        } => run_echo_recipe("echo", &input, &gain_in, &gain_out, &taps, &output, backend),
        Command::Echos {
            input,
            gain_in,
            gain_out,
            taps,
            output,
            backend,
        } => run_echo_recipe(
            "echos", &input, &gain_in, &gain_out, &taps, &output, backend,
        ),
        Command::Chorus {
            input,
            gain_in,
            gain_out,
            interpolation,
            wave,
            stages,
            output,
            backend,
        } => run_chorus_recipe(
            &input,
            &gain_in,
            &gain_out,
            &interpolation,
            &wave,
            &stages,
            &output,
            backend,
        ),
        Command::Flanger {
            input,
            delay,
            depth,
            regen,
            width,
            speed,
            wave,
            phase,
            interpolation,
            output,
            backend,
        } => run_effect_recipe(
            &input,
            &output,
            backend,
            [
                "flanger",
                delay.as_str(),
                depth.as_str(),
                regen.as_str(),
                width.as_str(),
                speed.as_str(),
                wave.as_str(),
                phase.as_str(),
                interpolation.as_str(),
            ],
        ),
        Command::Phaser {
            input,
            gain_in,
            gain_out,
            delay,
            regen,
            speed,
            wave,
            interpolation,
            output,
            backend,
        } => run_phaser_recipe(
            &input,
            &gain_in,
            &gain_out,
            &delay,
            &regen,
            &speed,
            &wave,
            &interpolation,
            &output,
            backend,
        ),
        Command::Oops {
            input,
            output,
            backend,
        } => run_effect_recipe(&input, &output, backend, ["oops"]),
        Command::Riaa {
            input,
            output,
            backend,
        } => run_effect_recipe(&input, &output, backend, ["riaa"]),
        Command::Swap {
            input,
            output,
            backend,
        } => run_effect_recipe(&input, &output, backend, ["swap"]),
        Command::Contrast {
            input,
            amount,
            output,
            backend,
        } => run_effect_recipe(&input, &output, backend, ["contrast", amount.as_str()]),
        Command::Overdrive {
            input,
            gain,
            color,
            output,
            backend,
        } => run_effect_recipe(
            &input,
            &output,
            backend,
            ["overdrive", gain.as_str(), color.as_str()],
        ),
        Command::Saturation {
            input,
            saturation_type,
            blend,
            offset,
            parameter,
            output,
            backend,
        } => run_saturation_recipe(
            &input,
            &saturation_type,
            &blend,
            &offset,
            parameter.as_deref(),
            &output,
            backend,
        ),
        Command::DcShift {
            input,
            shift,
            limiter_gain,
            output,
            backend,
        } => run_dc_shift_recipe(&input, &shift, limiter_gain.as_deref(), &output, backend),
        Command::Vol {
            input,
            gain,
            gain_type,
            limiter_gain,
            output,
            backend,
        } => run_vol_recipe(
            &input,
            &gain,
            gain_type.as_deref(),
            limiter_gain.as_deref(),
            &output,
            backend,
        ),
        Command::SoftVol {
            input,
            volume,
            double_time,
            headroom,
            output,
            backend,
        } => run_effect_recipe(
            &input,
            &output,
            backend,
            [
                "softvol",
                volume.as_str(),
                double_time.as_str(),
                headroom.as_str(),
            ],
        ),
        Command::Tremolo {
            input,
            speed,
            depth,
            output,
            backend,
        } => run_effect_recipe(
            &input,
            &output,
            backend,
            ["tremolo", speed.as_str(), depth.as_str()],
        ),
        Command::Speed {
            input,
            factor,
            output,
            backend,
        } => run_effect_recipe(&input, &output, backend, ["speed", factor.as_str()]),
        Command::Tempo {
            input,
            factor,
            quick,
            profile,
            segment,
            search,
            overlap,
            output,
            backend,
        } => run_tempo_recipe(
            &input,
            &factor,
            quick,
            profile.as_deref(),
            TimingArgs {
                segment: segment.as_deref(),
                search: search.as_deref(),
                overlap: overlap.as_deref(),
            },
            &output,
            backend,
        ),
        Command::Pitch {
            input,
            cents,
            quick,
            segment,
            search,
            overlap,
            output,
            backend,
        } => run_pitch_recipe(
            &input,
            &cents,
            quick,
            TimingArgs {
                segment: segment.as_deref(),
                search: search.as_deref(),
                overlap: overlap.as_deref(),
            },
            &output,
            backend,
        ),
        Command::Bass {
            input,
            gain,
            frequency,
            width,
            output,
            backend,
        } => run_effect_recipe(
            &input,
            &output,
            backend,
            ["bass", gain.as_str(), frequency.as_str(), width.as_str()],
        ),
        Command::Treble {
            input,
            gain,
            frequency,
            width,
            output,
            backend,
        } => run_effect_recipe(
            &input,
            &output,
            backend,
            ["treble", gain.as_str(), frequency.as_str(), width.as_str()],
        ),
        Command::Equalizer {
            input,
            frequency,
            width,
            gain,
            output,
            backend,
        } => run_effect_recipe(
            &input,
            &output,
            backend,
            [
                "equalizer",
                frequency.as_str(),
                width.as_str(),
                gain.as_str(),
            ],
        ),
        Command::AllPass {
            input,
            frequency,
            width,
            poles,
            output,
            backend,
        } => run_pole_filter_recipe(
            &input,
            "allpass",
            &frequency,
            width.as_deref(),
            poles,
            &output,
            backend,
        ),
        Command::Band {
            input,
            frequency,
            width,
            unpitched,
            output,
            backend,
        } => run_band_recipe(
            &input,
            &frequency,
            width.as_deref(),
            unpitched,
            &output,
            backend,
        ),
        Command::BandPass {
            input,
            frequency,
            width,
            constant_skirt,
            output,
            backend,
        } => run_bandpass_recipe(&input, &frequency, &width, constant_skirt, &output, backend),
        Command::BandReject {
            input,
            frequency,
            width,
            output,
            backend,
        } => run_effect_recipe(
            &input,
            &output,
            backend,
            ["bandreject", frequency.as_str(), width.as_str()],
        ),
        Command::HighPass {
            input,
            frequency,
            width,
            poles,
            output,
            backend,
        } => run_pole_filter_recipe(
            &input,
            "highpass",
            &frequency,
            width.as_deref(),
            poles,
            &output,
            backend,
        ),
        Command::LowPass {
            input,
            frequency,
            width,
            poles,
            output,
            backend,
        } => run_pole_filter_recipe(
            &input,
            "lowpass",
            &frequency,
            width.as_deref(),
            poles,
            &output,
            backend,
        ),
        Command::Fade {
            input,
            fade_in,
            fade_out,
            curve,
            output,
            backend,
        } => run_fade_recipe(
            &input,
            &fade_in,
            fade_out.as_deref(),
            &curve,
            &output,
            backend,
        ),
        Command::Delay {
            input,
            positions,
            output,
            backend,
        } => run_delay_recipe(&input, &positions, &output, backend),
        Command::Pad {
            input,
            start,
            end,
            positioned,
            output,
            backend,
        } => run_pad_recipe(&input, &start, &end, &positioned, &output, backend),
        Command::Repeat {
            input,
            count,
            output,
            backend,
        } => run_effect_recipe(&input, &output, backend, ["repeat", count.as_str()]),
        Command::Downsample {
            input,
            factor,
            output,
            backend,
        } => run_effect_recipe(&input, &output, backend, ["downsample", factor.as_str()]),
        Command::Upsample {
            input,
            factor,
            output,
            backend,
        } => run_effect_recipe(&input, &output, backend, ["upsample", factor.as_str()]),
        Command::Hilbert {
            input,
            taps,
            output,
            backend,
        } => run_hilbert_recipe(&input, taps.as_deref(), &output, backend),
        Command::Loudness {
            input,
            gain,
            reference,
            half_points,
            output,
            backend,
        } => run_effect_recipe(
            &input,
            &output,
            backend,
            [
                "loudness",
                gain.as_str(),
                reference.as_str(),
                half_points.as_str(),
            ],
        ),
        Command::Dither {
            input,
            sloped,
            noise_shape,
            precision,
            output,
            backend,
        } => run_dither_recipe(
            &input,
            sloped,
            noise_shape.as_deref(),
            &precision,
            &output,
            backend,
        ),
        Command::Reverb {
            input,
            wet_only,
            reverberance,
            hf_damping,
            room_scale,
            stereo_depth,
            pre_delay,
            wet_gain,
            output,
            backend,
        } => run_reverb_recipe(
            &input,
            wet_only,
            &reverberance,
            &hf_damping,
            &room_scale,
            &stereo_depth,
            &pre_delay,
            &wet_gain,
            &output,
            backend,
        ),
        Command::Stretch {
            input,
            factor,
            window,
            fade,
            shift,
            fading,
            output,
            backend,
        } => run_stretch_recipe(
            &input,
            StretchRecipeOptions {
                factor: &factor,
                window: &window,
                fade: &fade,
                shift: shift.as_deref(),
                fading: fading.as_deref(),
            },
            &output,
            backend,
        ),
        Command::Mix {
            inputs,
            output,
            backend,
        } => run_mix_recipe(&inputs, &output, backend),
        Command::Concat {
            inputs,
            output,
            backend,
        } => run_combine_recipe(
            &inputs,
            &output,
            backend,
            auralis::CombineMethod::Concatenate,
        ),
        Command::MixPower {
            inputs,
            output,
            backend,
        } => run_combine_recipe(&inputs, &output, backend, auralis::CombineMethod::MixPower),
        Command::Merge {
            inputs,
            output,
            backend,
        } => run_combine_recipe(&inputs, &output, backend, auralis::CombineMethod::Merge),
        Command::Multiply {
            inputs,
            output,
            backend,
        } => run_combine_recipe(&inputs, &output, backend, auralis::CombineMethod::Multiply),
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
    graph_plan::plan_graph_spec(spec, json, locked)
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
        name: "norm",
        options: &["-o", "--output", "--backend"],
    },
    CompletionSpec {
        name: "rate",
        options: &["-o", "--output", "--backend"],
    },
    CompletionSpec {
        name: "channels",
        options: &["-o", "--output", "--backend"],
    },
    CompletionSpec {
        name: "gain",
        options: &["-o", "--output", "--backend"],
    },
    CompletionSpec {
        name: "reverse",
        options: &["-o", "--output", "--backend"],
    },
    CompletionSpec {
        name: "deemph",
        options: &["-o", "--output", "--backend"],
    },
    CompletionSpec {
        name: "earwax",
        options: &["-o", "--output", "--backend"],
    },
    CompletionSpec {
        name: "echo",
        options: &[
            "-o",
            "--output",
            "--gain-in",
            "--gain-out",
            "--tap",
            "--backend",
        ],
    },
    CompletionSpec {
        name: "echos",
        options: &[
            "-o",
            "--output",
            "--gain-in",
            "--gain-out",
            "--tap",
            "--backend",
        ],
    },
    CompletionSpec {
        name: "chorus",
        options: &[
            "-o",
            "--output",
            "--gain-in",
            "--gain-out",
            "--interpolation",
            "--wave",
            "--stage",
            "--backend",
        ],
    },
    CompletionSpec {
        name: "flanger",
        options: &[
            "-o",
            "--output",
            "--delay",
            "--depth",
            "--regen",
            "--width",
            "--speed",
            "--wave",
            "--phase",
            "--interpolation",
            "--backend",
        ],
    },
    CompletionSpec {
        name: "phaser",
        options: &[
            "-o",
            "--output",
            "--gain-in",
            "--gain-out",
            "--delay",
            "--regen",
            "--speed",
            "--wave",
            "--interpolation",
            "--backend",
        ],
    },
    CompletionSpec {
        name: "oops",
        options: &["-o", "--output", "--backend"],
    },
    CompletionSpec {
        name: "riaa",
        options: &["-o", "--output", "--backend"],
    },
    CompletionSpec {
        name: "swap",
        options: &["-o", "--output", "--backend"],
    },
    CompletionSpec {
        name: "contrast",
        options: &["-o", "--output", "--amount", "--backend"],
    },
    CompletionSpec {
        name: "overdrive",
        options: &["-o", "--output", "--gain", "--color", "--backend"],
    },
    CompletionSpec {
        name: "saturation",
        options: &[
            "-o",
            "--output",
            "--type",
            "--blend",
            "--offset",
            "--parameter",
            "--backend",
        ],
    },
    CompletionSpec {
        name: "dcshift",
        options: &["-o", "--output", "--limiter-gain", "--backend"],
    },
    CompletionSpec {
        name: "vol",
        options: &["-o", "--output", "--type", "--limiter-gain", "--backend"],
    },
    CompletionSpec {
        name: "softvol",
        options: &[
            "-o",
            "--output",
            "--volume",
            "--double-time",
            "--headroom",
            "--backend",
        ],
    },
    CompletionSpec {
        name: "tremolo",
        options: &["-o", "--output", "--depth", "--backend"],
    },
    CompletionSpec {
        name: "speed",
        options: &["-o", "--output", "--backend"],
    },
    CompletionSpec {
        name: "tempo",
        options: &[
            "-o",
            "--output",
            "--quick",
            "--profile",
            "--segment",
            "--search",
            "--overlap",
            "--backend",
        ],
    },
    CompletionSpec {
        name: "pitch",
        options: &[
            "-o",
            "--output",
            "--quick",
            "--segment",
            "--search",
            "--overlap",
            "--backend",
        ],
    },
    CompletionSpec {
        name: "bass",
        options: &["-o", "--output", "--frequency", "--width", "--backend"],
    },
    CompletionSpec {
        name: "treble",
        options: &["-o", "--output", "--frequency", "--width", "--backend"],
    },
    CompletionSpec {
        name: "equalizer",
        options: &[
            "-o",
            "--output",
            "--frequency",
            "--width",
            "--gain",
            "--backend",
        ],
    },
    CompletionSpec {
        name: "allpass",
        options: &[
            "-o",
            "--output",
            "--frequency",
            "--width",
            "--poles",
            "--backend",
        ],
    },
    CompletionSpec {
        name: "band",
        options: &[
            "-o",
            "--output",
            "--frequency",
            "--width",
            "--unpitched",
            "--backend",
        ],
    },
    CompletionSpec {
        name: "bandpass",
        options: &[
            "-o",
            "--output",
            "--frequency",
            "--width",
            "--constant-skirt",
            "--backend",
        ],
    },
    CompletionSpec {
        name: "bandreject",
        options: &["-o", "--output", "--frequency", "--width", "--backend"],
    },
    CompletionSpec {
        name: "highpass",
        options: &[
            "-o",
            "--output",
            "--frequency",
            "--width",
            "--poles",
            "--backend",
        ],
    },
    CompletionSpec {
        name: "lowpass",
        options: &[
            "-o",
            "--output",
            "--frequency",
            "--width",
            "--poles",
            "--backend",
        ],
    },
    CompletionSpec {
        name: "fade",
        options: &["-o", "--output", "--in", "--out", "--curve", "--backend"],
    },
    CompletionSpec {
        name: "delay",
        options: &["-o", "--output", "--position", "--backend"],
    },
    CompletionSpec {
        name: "pad",
        options: &["-o", "--output", "--start", "--end", "--at", "--backend"],
    },
    CompletionSpec {
        name: "repeat",
        options: &["-o", "--output", "--backend"],
    },
    CompletionSpec {
        name: "downsample",
        options: &["-o", "--output", "--backend"],
    },
    CompletionSpec {
        name: "upsample",
        options: &["-o", "--output", "--backend"],
    },
    CompletionSpec {
        name: "hilbert",
        options: &["-o", "--output", "--taps", "--backend"],
    },
    CompletionSpec {
        name: "loudness",
        options: &[
            "-o",
            "--output",
            "--gain",
            "--reference",
            "--half-points",
            "--backend",
        ],
    },
    CompletionSpec {
        name: "dither",
        options: &[
            "-o",
            "--output",
            "--sloped",
            "--noise-shape",
            "--precision",
            "--backend",
        ],
    },
    CompletionSpec {
        name: "reverb",
        options: &[
            "-o",
            "--output",
            "--wet-only",
            "--reverberance",
            "--hf-damping",
            "--room-scale",
            "--stereo-depth",
            "--pre-delay",
            "--wet-gain",
            "--backend",
        ],
    },
    CompletionSpec {
        name: "stretch",
        options: &[
            "-o",
            "--output",
            "--window",
            "--fade",
            "--shift",
            "--fading",
            "--backend",
        ],
    },
    CompletionSpec {
        name: "mix",
        options: &["-o", "--output", "--backend"],
    },
    CompletionSpec {
        name: "concat",
        options: &["-o", "--output", "--backend"],
    },
    CompletionSpec {
        name: "mix-power",
        options: &["-o", "--output", "--backend"],
    },
    CompletionSpec {
        name: "merge",
        options: &["-o", "--output", "--backend"],
    },
    CompletionSpec {
        name: "multiply",
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
            ("norm", "Normalize with the typed norm effect."),
            ("rate", "Resample with the typed rate effect."),
            ("channels", "Convert to a target channel count."),
            ("gain", "Adjust one audio file by a gain amount."),
            ("reverse", "Reverse one audio file."),
            ("deemph", "Apply CD/DAT de-emphasis to one audio file."),
            ("earwax", "Apply a stereo headphone-cue filter."),
            ("echo", "Add one or more parallel delayed echoes."),
            ("echos", "Add one or more cascaded delayed echoes."),
            ("chorus", "Add chorus modulation."),
            ("flanger", "Add flanger modulation."),
            ("phaser", "Add phaser modulation."),
            ("oops", "Extract out-of-phase stereo content."),
            ("riaa", "Apply RIAA vinyl playback equalization."),
            ("swap", "Swap adjacent channel pairs."),
            ("contrast", "Enhance sample contrast."),
            ("overdrive", "Apply overdrive distortion."),
            ("saturation", "Apply saturation distortion."),
            ("dcshift", "Shift DC level."),
            ("vol", "Apply SoX-ng volume scaling."),
            ("softvol", "Apply soft volume changes."),
            ("tremolo", "Apply tremolo modulation."),
            ("speed", "Change playback speed and sample rate."),
            ("tempo", "Change tempo without changing pitch."),
            ("pitch", "Shift pitch without changing tempo."),
            ("bass", "Boost or cut bass frequencies."),
            ("treble", "Boost or cut treble frequencies."),
            ("equalizer", "Apply one peaking equalizer band."),
            ("allpass", "Apply an all-pass filter."),
            ("band", "Apply a resonator band-pass filter."),
            ("bandpass", "Apply an RBJ band-pass filter."),
            ("bandreject", "Apply an RBJ band-reject filter."),
            ("highpass", "Apply a high-pass filter."),
            ("lowpass", "Apply a low-pass filter."),
            ("fade", "Fade one audio file in or out."),
            ("delay", "Delay audio channels."),
            ("pad", "Add silence padding."),
            ("repeat", "Append finite copies."),
            ("downsample", "Keep every Nth sample."),
            ("upsample", "Insert zero samples between input samples."),
            ("hilbert", "Apply Hilbert transform phase shifting."),
            ("loudness", "Apply loudness compensation filtering."),
            ("dither", "Apply deterministic dithering."),
            ("reverb", "Apply stereo reverberation."),
            ("stretch", "Change duration with windowed stretching."),
            ("mix", "Mix two or more audio files into one output."),
            ("concat", "Concatenate two or more audio files end-to-end."),
            (
                "mix-power",
                "Mix two or more audio files with equal-power scaling.",
            ),
            ("merge", "Merge channels from two or more audio files."),
            (
                "multiply",
                "Multiply corresponding samples from two or more audio files.",
            ),
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
        name: "norm",
        summary: "normalize with the typed norm effect",
        synopsis: "auralis norm INPUT.wav [DBFS] -o OUTPUT.wav [--backend BACKEND]",
        description: "Norm is a recipe alias for the typed peak-normalization effect. It lowers to the same typed effect pipeline as `render --fx 'norm ...'`.",
        options: &[
            ("DBFS", "Peak target in dBFS."),
            ("-o, --output FILE", "Output WAV file to create."),
            ("--backend BACKEND", "Request scalar or simd processing."),
        ],
    },
    ManPage {
        name: "rate",
        summary: "resample with the typed rate effect",
        synopsis: "auralis rate INPUT.wav RATE -o OUTPUT.wav [--backend BACKEND]",
        description: "Rate is a recipe alias for typed sample-rate conversion. It lowers to the same typed effect pipeline as `render --fx 'rate ...'`.",
        options: &[
            ("RATE", "Target sample rate in Hz."),
            ("-o, --output FILE", "Output WAV file to create."),
            ("--backend BACKEND", "Request scalar or simd processing."),
        ],
    },
    ManPage {
        name: "channels",
        summary: "convert to a target channel count",
        synopsis: "auralis channels INPUT.wav CHANNELS -o OUTPUT.wav [--backend BACKEND]",
        description: "Channels is a recipe alias for typed channel-count conversion. It lowers to the same typed effect pipeline as `render --fx 'channels ...'`.",
        options: &[
            ("CHANNELS", "Target channel count."),
            ("-o, --output FILE", "Output WAV file to create."),
            ("--backend BACKEND", "Request scalar or simd processing."),
        ],
    },
    ManPage {
        name: "gain",
        summary: "adjust one audio file by gain",
        synopsis: "auralis gain INPUT.wav DB -o OUTPUT.wav [--backend BACKEND]",
        description: "Gain is a recipe alias for a single gain adjustment. It lowers to the same typed effect pipeline as `render --fx 'gain ...'`.",
        options: &[
            (
                "DB",
                "Gain adjustment in dB, accepting values like `-3` or `-3dB`.",
            ),
            ("-o, --output FILE", "Output WAV file to create."),
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
        name: "deemph",
        summary: "apply de-emphasis",
        synopsis: "auralis deemph INPUT.wav -o OUTPUT.wav [--backend BACKEND]",
        description: "Deemph is a recipe alias for CD/DAT de-emphasis. It lowers to the same typed effect pipeline as `render --fx deemph`.",
        options: &[
            ("-o, --output FILE", "Output WAV file to create."),
            ("--backend BACKEND", "Request scalar or simd processing."),
        ],
    },
    ManPage {
        name: "earwax",
        summary: "apply headphone-cue filtering",
        synopsis: "auralis earwax INPUT.wav -o OUTPUT.wav [--backend BACKEND]",
        description: "Earwax is a recipe alias for the stereo headphone-cue filter. It lowers to the same typed effect pipeline as `render --fx earwax`.",
        options: &[
            ("-o, --output FILE", "Output WAV file to create."),
            ("--backend BACKEND", "Request scalar or simd processing."),
        ],
    },
    ManPage {
        name: "echo",
        summary: "add parallel delayed echoes",
        synopsis: "auralis echo INPUT.wav [--gain-in GAIN] [--gain-out GAIN] --tap DELAY_MS,DECAY... -o OUTPUT.wav [--backend BACKEND]",
        description: "Echo is a recipe alias for one or more parallel delay taps. It lowers to the same typed effect pipeline as `render --fx 'echo ...'`.",
        options: &[
            ("--gain-in GAIN", "Clean input gain."),
            ("--gain-out GAIN", "Output gain."),
            (
                "--tap DELAY_MS,DECAY",
                "Echo tap delay in milliseconds and decay; repeat for multiple taps.",
            ),
            ("-o, --output FILE", "Output WAV file to create."),
            ("--backend BACKEND", "Request scalar or simd processing."),
        ],
    },
    ManPage {
        name: "echos",
        summary: "add cascaded delayed echoes",
        synopsis: "auralis echos INPUT.wav [--gain-in GAIN] [--gain-out GAIN] --tap DELAY_MS,DECAY... -o OUTPUT.wav [--backend BACKEND]",
        description: "Echos is a recipe alias for one or more cascaded delay taps. It lowers to the same typed effect pipeline as `render --fx 'echos ...'`.",
        options: &[
            ("--gain-in GAIN", "Clean input gain."),
            ("--gain-out GAIN", "Output gain."),
            (
                "--tap DELAY_MS,DECAY",
                "Echo tap delay in milliseconds and decay; repeat for multiple taps.",
            ),
            ("-o, --output FILE", "Output WAV file to create."),
            ("--backend BACKEND", "Request scalar or simd processing."),
        ],
    },
    ManPage {
        name: "chorus",
        summary: "add chorus modulation",
        synopsis: "auralis chorus INPUT.wav [--gain-in GAIN] [--gain-out GAIN] [--interpolation MODE] [--wave WAVE] [--stage DELAY_MS,DECAY,SPEED_HZ,DEPTH_MS[,WAVE]]... -o OUTPUT.wav [--backend BACKEND]",
        description: "Chorus is a recipe alias for chorus modulation. It lowers to the same typed effect pipeline as `render --fx 'chorus ...'`.",
        options: &[
            ("--gain-in GAIN", "Clean input gain."),
            ("--gain-out GAIN", "Output gain."),
            (
                "--interpolation MODE",
                "Interpolation mode: none, linear, or quadratic.",
            ),
            ("--wave WAVE", "Default modulation wave: sine or triangle."),
            (
                "--stage STAGE",
                "Chorus stage as delay, decay, speed, and depth; repeat for multiple stages.",
            ),
            ("-o, --output FILE", "Output WAV file to create."),
            ("--backend BACKEND", "Request scalar or simd processing."),
        ],
    },
    ManPage {
        name: "flanger",
        summary: "add flanger modulation",
        synopsis: "auralis flanger INPUT.wav [--delay MS] [--depth MS] [--regen PERCENT] [--width PERCENT] [--speed HZ] [--wave WAVE] [--phase PERCENT] [--interpolation MODE] -o OUTPUT.wav [--backend BACKEND]",
        description: "Flanger is a recipe alias for swept-delay flanger modulation. It lowers to the same typed effect pipeline as `render --fx 'flanger ...'`.",
        options: &[
            ("--delay MS", "Base delay in milliseconds."),
            ("--depth MS", "Sweep depth in milliseconds."),
            ("--regen PERCENT", "Regeneration percentage."),
            ("--width PERCENT", "Wet width percentage."),
            ("--speed HZ", "Modulation speed."),
            ("--wave WAVE", "Modulation wave: sine or triangle."),
            ("--phase PERCENT", "Stereo phase percentage."),
            (
                "--interpolation MODE",
                "Interpolation mode: none, linear, or quadratic.",
            ),
            ("-o, --output FILE", "Output WAV file to create."),
            ("--backend BACKEND", "Request scalar or simd processing."),
        ],
    },
    ManPage {
        name: "phaser",
        summary: "add phaser modulation",
        synopsis: "auralis phaser INPUT.wav [--gain-in GAIN] [--gain-out GAIN] [--delay MS] [--regen AMOUNT] [--speed HZ] [--wave WAVE] [--interpolation MODE] -o OUTPUT.wav [--backend BACKEND]",
        description: "Phaser is a recipe alias for swept-delay phaser modulation. It lowers to the same typed effect pipeline as `render --fx 'phaser ...'`.",
        options: &[
            ("--gain-in GAIN", "Clean input gain."),
            ("--gain-out GAIN", "Output gain."),
            ("--delay MS", "Delay in milliseconds."),
            ("--regen AMOUNT", "Regeneration amount."),
            ("--speed HZ", "Modulation speed."),
            ("--wave WAVE", "Modulation wave: sine or triangle."),
            (
                "--interpolation MODE",
                "Interpolation mode: none, linear, or quadratic.",
            ),
            ("-o, --output FILE", "Output WAV file to create."),
            ("--backend BACKEND", "Request scalar or simd processing."),
        ],
    },
    ManPage {
        name: "oops",
        summary: "extract out-of-phase stereo",
        synopsis: "auralis oops INPUT.wav -o OUTPUT.wav [--backend BACKEND]",
        description: "Oops is a recipe alias for extracting out-of-phase stereo content. It lowers to the same typed effect pipeline as `render --fx oops`.",
        options: &[
            ("-o, --output FILE", "Output WAV file to create."),
            ("--backend BACKEND", "Request scalar or simd processing."),
        ],
    },
    ManPage {
        name: "riaa",
        summary: "apply RIAA equalization",
        synopsis: "auralis riaa INPUT.wav -o OUTPUT.wav [--backend BACKEND]",
        description: "Riaa is a recipe alias for vinyl playback equalization. It lowers to the same typed effect pipeline as `render --fx riaa`.",
        options: &[
            ("-o, --output FILE", "Output WAV file to create."),
            ("--backend BACKEND", "Request scalar or simd processing."),
        ],
    },
    ManPage {
        name: "swap",
        summary: "swap adjacent channel pairs",
        synopsis: "auralis swap INPUT.wav -o OUTPUT.wav [--backend BACKEND]",
        description: "Swap is a recipe alias for exchanging adjacent channel pairs. It lowers to the same typed effect pipeline as `render --fx swap`.",
        options: &[
            ("-o, --output FILE", "Output WAV file to create."),
            ("--backend BACKEND", "Request scalar or simd processing."),
        ],
    },
    ManPage {
        name: "contrast",
        summary: "enhance sample contrast",
        synopsis: "auralis contrast INPUT.wav [--amount AMOUNT] -o OUTPUT.wav [--backend BACKEND]",
        description: "Contrast is a recipe alias for phase contrast enhancement. It lowers to the same typed effect pipeline as `render --fx 'contrast ...'`.",
        options: &[
            ("--amount AMOUNT", "Contrast amount from 0 to 100."),
            ("-o, --output FILE", "Output WAV file to create."),
            ("--backend BACKEND", "Request scalar or simd processing."),
        ],
    },
    ManPage {
        name: "overdrive",
        summary: "apply overdrive distortion",
        synopsis: "auralis overdrive INPUT.wav [--gain GAIN] [--color COLOR] -o OUTPUT.wav [--backend BACKEND]",
        description: "Overdrive is a recipe alias for applying one overdrive distortion stage. It lowers to the same typed effect pipeline as `render --fx 'overdrive ...'`.",
        options: &[
            ("--gain GAIN", "Overdrive gain."),
            ("--color COLOR", "Overdrive color."),
            ("-o, --output FILE", "Output WAV file to create."),
            ("--backend BACKEND", "Request scalar or simd processing."),
        ],
    },
    ManPage {
        name: "saturation",
        summary: "apply saturation distortion",
        synopsis: "auralis saturation INPUT.wav [--type TYPE] [--blend BLEND] [--offset OFFSET] [--parameter VALUE] -o OUTPUT.wav [--backend BACKEND]",
        description: "Saturation is a recipe alias for applying one saturation curve. It lowers to the same typed effect pipeline as `render --fx 'saturation ...'`.",
        options: &[
            (
                "--type TYPE",
                "Saturation curve type: tanh, sqrt, or diode.",
            ),
            ("--blend BLEND", "Wet/dry blend amount."),
            ("--offset OFFSET", "Input offset before saturation."),
            (
                "--parameter VALUE",
                "Curve-specific parameter: drive, color, or threshold.",
            ),
            ("-o, --output FILE", "Output WAV file to create."),
            ("--backend BACKEND", "Request scalar or simd processing."),
        ],
    },
    ManPage {
        name: "dcshift",
        summary: "shift DC level",
        synopsis: "auralis dcshift INPUT.wav SHIFT [--limiter-gain GAIN] -o OUTPUT.wav [--backend BACKEND]",
        description: "Dcshift is a recipe alias for shifting sample DC offset. It lowers to the same typed effect pipeline as `render --fx 'dcshift ...'`.",
        options: &[
            ("SHIFT", "DC shift amount."),
            ("--limiter-gain GAIN", "Optional limiter gain."),
            ("-o, --output FILE", "Output WAV file to create."),
            ("--backend BACKEND", "Request scalar or simd processing."),
        ],
    },
    ManPage {
        name: "vol",
        summary: "apply SoX-ng volume scaling",
        synopsis: "auralis vol INPUT.wav GAIN [--type TYPE] [--limiter-gain GAIN] -o OUTPUT.wav [--backend BACKEND]",
        description: "Vol is a recipe alias for SoX-ng volume scaling. It lowers to the same typed effect pipeline as `render --fx 'vol ...'`.",
        options: &[
            ("GAIN", "Volume gain value, for example `0.5` or `-6dB`."),
            (
                "--type TYPE",
                "Gain interpretation: amplitude, power, or dB.",
            ),
            ("--limiter-gain GAIN", "Optional limiter gain."),
            ("-o, --output FILE", "Output WAV file to create."),
            ("--backend BACKEND", "Request scalar or simd processing."),
        ],
    },
    ManPage {
        name: "softvol",
        summary: "apply soft volume changes",
        synopsis: "auralis softvol INPUT.wav [--volume VOLUME] [--double-time SECONDS] [--headroom DB] -o OUTPUT.wav [--backend BACKEND]",
        description: "Softvol is a recipe alias for soft volume adjustment. It lowers to the same typed effect pipeline as `render --fx 'softvol ...'`.",
        options: &[
            ("--volume VOLUME", "Volume multiplier."),
            (
                "--double-time SECONDS",
                "Seconds required for volume doubling.",
            ),
            ("--headroom DB", "Extra headroom in dB."),
            ("-o, --output FILE", "Output WAV file to create."),
            ("--backend BACKEND", "Request scalar or simd processing."),
        ],
    },
    ManPage {
        name: "tremolo",
        summary: "apply tremolo modulation",
        synopsis: "auralis tremolo INPUT.wav SPEED_HZ [--depth PERCENT] -o OUTPUT.wav [--backend BACKEND]",
        description: "Tremolo is a recipe alias for amplitude modulation. It lowers to the same typed effect pipeline as `render --fx 'tremolo ...'`.",
        options: &[
            ("SPEED_HZ", "Modulation speed in Hz."),
            ("--depth PERCENT", "Modulation depth percentage."),
            ("-o, --output FILE", "Output WAV file to create."),
            ("--backend BACKEND", "Request scalar or simd processing."),
        ],
    },
    ManPage {
        name: "speed",
        summary: "change playback speed",
        synopsis: "auralis speed INPUT.wav FACTOR -o OUTPUT.wav [--backend BACKEND]",
        description: "Speed is a recipe alias for changing playback speed and output sample rate. It lowers to the same typed effect pipeline as `render --fx 'speed ...'`.",
        options: &[
            ("FACTOR", "Speed factor, or cents with a `c` suffix."),
            ("-o, --output FILE", "Output WAV file to create."),
            ("--backend BACKEND", "Request scalar or simd processing."),
        ],
    },
    ManPage {
        name: "tempo",
        summary: "change tempo without changing pitch",
        synopsis: "auralis tempo INPUT.wav FACTOR [--quick] [--profile PROFILE] [--segment MS [--search MS [--overlap MS]]] -o OUTPUT.wav [--backend BACKEND]",
        description: "Tempo is a recipe alias for time stretching without pitch shift. It lowers to the same typed effect pipeline as `render --fx 'tempo ...'`.",
        options: &[
            ("FACTOR", "Tempo factor."),
            ("--quick", "Prefer quicker search."),
            (
                "--profile PROFILE",
                "Tuning profile: music, speech, or linear.",
            ),
            ("--segment MS", "Segment length in milliseconds."),
            ("--search MS", "Search length in milliseconds."),
            ("--overlap MS", "Overlap length in milliseconds."),
            ("-o, --output FILE", "Output WAV file to create."),
            ("--backend BACKEND", "Request scalar or simd processing."),
        ],
    },
    ManPage {
        name: "pitch",
        summary: "shift pitch without changing tempo",
        synopsis: "auralis pitch INPUT.wav CENTS [--quick] [--segment MS [--search MS [--overlap MS]]] -o OUTPUT.wav [--backend BACKEND]",
        description: "Pitch is a recipe alias for shifting pitch while preserving duration. It lowers to the same typed effect pipeline as `render --fx 'pitch ...'`.",
        options: &[
            ("CENTS", "Pitch shift in cents."),
            ("--quick", "Prefer quicker search."),
            ("--segment MS", "Segment length in milliseconds."),
            ("--search MS", "Search length in milliseconds."),
            ("--overlap MS", "Overlap length in milliseconds."),
            ("-o, --output FILE", "Output WAV file to create."),
            ("--backend BACKEND", "Request scalar or simd processing."),
        ],
    },
    ManPage {
        name: "bass",
        summary: "boost or cut bass frequencies",
        synopsis: "auralis bass INPUT.wav DB [--frequency HZ] [--width WIDTH] -o OUTPUT.wav [--backend BACKEND]",
        description: "Bass is a recipe alias for one low-shelf EQ stage. It lowers to the same typed effect pipeline as `render --fx 'bass ...'`.",
        options: &[
            ("DB", "Shelf gain in dB."),
            ("--frequency HZ", "Shelf frequency in Hz."),
            (
                "--width WIDTH",
                "Shelf width, accepting values like `0.5s`, `0.707q`, or `1o`.",
            ),
            ("-o, --output FILE", "Output WAV file to create."),
            ("--backend BACKEND", "Request scalar or simd processing."),
        ],
    },
    ManPage {
        name: "treble",
        summary: "boost or cut treble frequencies",
        synopsis: "auralis treble INPUT.wav DB [--frequency HZ] [--width WIDTH] -o OUTPUT.wav [--backend BACKEND]",
        description: "Treble is a recipe alias for one high-shelf EQ stage. It lowers to the same typed effect pipeline as `render --fx 'treble ...'`.",
        options: &[
            ("DB", "Shelf gain in dB."),
            ("--frequency HZ", "Shelf frequency in Hz."),
            (
                "--width WIDTH",
                "Shelf width, accepting values like `0.5s`, `0.707q`, or `1o`.",
            ),
            ("-o, --output FILE", "Output WAV file to create."),
            ("--backend BACKEND", "Request scalar or simd processing."),
        ],
    },
    ManPage {
        name: "equalizer",
        summary: "apply one peaking equalizer band",
        synopsis: "auralis equalizer INPUT.wav --frequency HZ --width WIDTH --gain DB -o OUTPUT.wav [--backend BACKEND]",
        description: "Equalizer is a recipe alias for one peaking EQ band. It lowers to the same typed effect pipeline as `render --fx 'equalizer ...'`.",
        options: &[
            ("--frequency HZ", "Center frequency in Hz."),
            (
                "--width WIDTH",
                "Band width, accepting values like `500h`, `0.707q`, or `1o`.",
            ),
            ("--gain DB", "Band gain in dB."),
            ("-o, --output FILE", "Output WAV file to create."),
            ("--backend BACKEND", "Request scalar or simd processing."),
        ],
    },
    ManPage {
        name: "allpass",
        summary: "apply an all-pass filter",
        synopsis: "auralis allpass INPUT.wav --frequency HZ [--width WIDTH] [--poles 1|2] -o OUTPUT.wav [--backend BACKEND]",
        description: "Allpass is a recipe alias for one all-pass filter stage. It lowers to the same typed effect pipeline as `render --fx 'allpass ...'`.",
        options: &[
            ("--frequency HZ", "Filter frequency in Hz."),
            (
                "--width WIDTH",
                "Filter width, accepting values like `500h`, `0.707q`, or `1o`.",
            ),
            ("--poles 1|2", "Use the one-pole or two-pole simple form."),
            ("-o, --output FILE", "Output WAV file to create."),
            ("--backend BACKEND", "Request scalar or simd processing."),
        ],
    },
    ManPage {
        name: "band",
        summary: "apply a resonator band-pass filter",
        synopsis: "auralis band INPUT.wav --frequency HZ [--width WIDTH] [--unpitched] -o OUTPUT.wav [--backend BACKEND]",
        description: "Band is a recipe alias for one resonator band-pass filter stage. It lowers to the same typed effect pipeline as `render --fx 'band ...'`.",
        options: &[
            ("--frequency HZ", "Filter frequency in Hz."),
            (
                "--width WIDTH",
                "Optional filter width, accepting values like `500h`, `0.707q`, or `1o`.",
            ),
            ("--unpitched", "Use the unpitched noise mode."),
            ("-o, --output FILE", "Output WAV file to create."),
            ("--backend BACKEND", "Request scalar or simd processing."),
        ],
    },
    ManPage {
        name: "bandpass",
        summary: "apply an RBJ band-pass filter",
        synopsis: "auralis bandpass INPUT.wav --frequency HZ --width WIDTH [--constant-skirt] -o OUTPUT.wav [--backend BACKEND]",
        description: "Bandpass is a recipe alias for one RBJ band-pass filter stage. It lowers to the same typed effect pipeline as `render --fx 'bandpass ...'`.",
        options: &[
            ("--frequency HZ", "Filter frequency in Hz."),
            (
                "--width WIDTH",
                "Filter width, accepting values like `500h`, `0.707q`, or `1o`.",
            ),
            ("--constant-skirt", "Use constant-skirt-gain mode."),
            ("-o, --output FILE", "Output WAV file to create."),
            ("--backend BACKEND", "Request scalar or simd processing."),
        ],
    },
    ManPage {
        name: "bandreject",
        summary: "apply an RBJ band-reject filter",
        synopsis: "auralis bandreject INPUT.wav --frequency HZ --width WIDTH -o OUTPUT.wav [--backend BACKEND]",
        description: "Bandreject is a recipe alias for one RBJ band-reject filter stage. It lowers to the same typed effect pipeline as `render --fx 'bandreject ...'`.",
        options: &[
            ("--frequency HZ", "Filter frequency in Hz."),
            (
                "--width WIDTH",
                "Filter width, accepting values like `500h`, `0.707q`, or `1o`.",
            ),
            ("-o, --output FILE", "Output WAV file to create."),
            ("--backend BACKEND", "Request scalar or simd processing."),
        ],
    },
    ManPage {
        name: "highpass",
        summary: "apply a high-pass filter",
        synopsis: "auralis highpass INPUT.wav --frequency HZ [--width WIDTH] [--poles 1|2] -o OUTPUT.wav [--backend BACKEND]",
        description: "Highpass is a recipe alias for one high-pass filter stage. It lowers to the same typed effect pipeline as `render --fx 'highpass ...'`.",
        options: &[
            ("--frequency HZ", "Filter cutoff frequency in Hz."),
            (
                "--width WIDTH",
                "Optional filter width, accepting values like `500h`, `0.707q`, or `1o`.",
            ),
            ("--poles 1|2", "Use the one-pole or two-pole form."),
            ("-o, --output FILE", "Output WAV file to create."),
            ("--backend BACKEND", "Request scalar or simd processing."),
        ],
    },
    ManPage {
        name: "lowpass",
        summary: "apply a low-pass filter",
        synopsis: "auralis lowpass INPUT.wav --frequency HZ [--width WIDTH] [--poles 1|2] -o OUTPUT.wav [--backend BACKEND]",
        description: "Lowpass is a recipe alias for one low-pass filter stage. It lowers to the same typed effect pipeline as `render --fx 'lowpass ...'`.",
        options: &[
            ("--frequency HZ", "Filter cutoff frequency in Hz."),
            (
                "--width WIDTH",
                "Optional filter width, accepting values like `500h`, `0.707q`, or `1o`.",
            ),
            ("--poles 1|2", "Use the one-pole or two-pole form."),
            ("-o, --output FILE", "Output WAV file to create."),
            ("--backend BACKEND", "Request scalar or simd processing."),
        ],
    },
    ManPage {
        name: "fade",
        summary: "fade one audio file in or out",
        synopsis: "auralis fade INPUT.wav [--in FRAMES] [--out FRAMES] [--curve CURVE] -o OUTPUT.wav [--backend BACKEND]",
        description: "Fade is a recipe alias for applying one fade envelope. It lowers to the same typed effect pipeline as `render --fx 'fade ...'`.",
        options: &[
            (
                "--in FRAMES",
                "Fade-in length in frames, accepting values like `24000` or `24000f`.",
            ),
            (
                "--out FRAMES",
                "Fade-out length in frames, accepting values like `24000` or `24000f`.",
            ),
            (
                "--curve CURVE",
                "Fade curve family: linear, quarter-sine, half-sine, log, or parabola.",
            ),
            ("-o, --output FILE", "Output WAV file to create."),
            ("--backend BACKEND", "Request scalar or simd processing."),
        ],
    },
    ManPage {
        name: "delay",
        summary: "delay audio channels",
        synopsis: "auralis delay INPUT.wav --position POSITION... -o OUTPUT.wav [--backend BACKEND]",
        description: "Delay is a recipe alias for per-channel delay positions. It lowers to the same typed effect pipeline as `render --fx 'delay ...'`.",
        options: &[
            (
                "--position POSITION",
                "Delay position such as `2s`, `0.25`, or `+1s`; repeat per channel.",
            ),
            ("-o, --output FILE", "Output WAV file to create."),
            ("--backend BACKEND", "Request scalar or simd processing."),
        ],
    },
    ManPage {
        name: "pad",
        summary: "add silence padding",
        synopsis: "auralis pad INPUT.wav [--start FRAMES] [--at FRAMES@POSITION]... [--end FRAMES] -o OUTPUT.wav [--backend BACKEND]",
        description: "Pad is a recipe alias for inserting silence before, after, or inside one audio file. It lowers to the same typed effect pipeline as `render --fx 'pad ...'`.",
        options: &[
            ("--start FRAMES", "Silence to prepend, in frames."),
            ("--at FRAMES@POSITION", "Positioned silence insert."),
            ("--end FRAMES", "Silence to append, in frames."),
            ("-o, --output FILE", "Output WAV file to create."),
            ("--backend BACKEND", "Request scalar or simd processing."),
        ],
    },
    ManPage {
        name: "repeat",
        summary: "append finite copies",
        synopsis: "auralis repeat INPUT.wav [COUNT] -o OUTPUT.wav [--backend BACKEND]",
        description: "Repeat is a recipe alias for appending finite copies of one audio file. It lowers to the same typed effect pipeline as `render --fx 'repeat ...'`.",
        options: &[
            ("COUNT", "Number of extra copies to append."),
            ("-o, --output FILE", "Output WAV file to create."),
            ("--backend BACKEND", "Request scalar or simd processing."),
        ],
    },
    ManPage {
        name: "downsample",
        summary: "keep every Nth sample",
        synopsis: "auralis downsample INPUT.wav [FACTOR] -o OUTPUT.wav [--backend BACKEND]",
        description: "Downsample is a recipe alias for dropping samples by an integer factor. It lowers to the same typed effect pipeline as `render --fx 'downsample ...'`.",
        options: &[
            ("FACTOR", "Integer downsample factor."),
            ("-o, --output FILE", "Output WAV file to create."),
            ("--backend BACKEND", "Request scalar or simd processing."),
        ],
    },
    ManPage {
        name: "upsample",
        summary: "insert zero samples",
        synopsis: "auralis upsample INPUT.wav [FACTOR] -o OUTPUT.wav [--backend BACKEND]",
        description: "Upsample is a recipe alias for inserting zero samples between input samples. It lowers to the same typed effect pipeline as `render --fx 'upsample ...'`.",
        options: &[
            ("FACTOR", "Integer upsample factor."),
            ("-o, --output FILE", "Output WAV file to create."),
            ("--backend BACKEND", "Request scalar or simd processing."),
        ],
    },
    ManPage {
        name: "hilbert",
        summary: "apply Hilbert transform",
        synopsis: "auralis hilbert INPUT.wav [--taps TAPS] -o OUTPUT.wav [--backend BACKEND]",
        description: "Hilbert is a recipe alias for phase shifting with a Hilbert transform FIR. It lowers to the same typed effect pipeline as `render --fx 'hilbert ...'`.",
        options: &[
            ("--taps TAPS", "Optional odd FIR tap count."),
            ("-o, --output FILE", "Output WAV file to create."),
            ("--backend BACKEND", "Request scalar or simd processing."),
        ],
    },
    ManPage {
        name: "loudness",
        summary: "apply loudness compensation",
        synopsis: "auralis loudness INPUT.wav [--gain DB] [--reference DB] [--half-points N] -o OUTPUT.wav [--backend BACKEND]",
        description: "Loudness is a recipe alias for loudness compensation filtering. It lowers to the same typed effect pipeline as `render --fx 'loudness ...'`.",
        options: &[
            ("--gain DB", "Gain in dB."),
            ("--reference DB", "Reference level in dB."),
            ("--half-points N", "Number of FIR half-points."),
            ("-o, --output FILE", "Output WAV file to create."),
            ("--backend BACKEND", "Request scalar or simd processing."),
        ],
    },
    ManPage {
        name: "dither",
        summary: "apply deterministic dithering",
        synopsis: "auralis dither INPUT.wav [--sloped] [--noise-shape shibata] [--precision BITS] -o OUTPUT.wav [--backend BACKEND]",
        description: "Dither is a recipe alias for deterministic dither processing. It lowers to the same typed effect pipeline as `render --fx 'dither ...'`.",
        options: &[
            ("--sloped", "Use sloped TPDF dither."),
            ("--noise-shape SHAPE", "Noise-shaping filter: shibata."),
            ("--precision BITS", "Target precision in bits."),
            ("-o, --output FILE", "Output WAV file to create."),
            ("--backend BACKEND", "Request scalar or simd processing."),
        ],
    },
    ManPage {
        name: "reverb",
        summary: "apply stereo reverberation",
        synopsis: "auralis reverb INPUT.wav [--wet-only] [--reverberance PERCENT] [--hf-damping PERCENT] [--room-scale PERCENT] [--stereo-depth PERCENT] [--pre-delay MS] [--wet-gain DB] -o OUTPUT.wav [--backend BACKEND]",
        description: "Reverb is a recipe alias for stereo reverberation. It lowers to the same typed effect pipeline as `render --fx 'reverb ...'`.",
        options: &[
            ("--wet-only", "Output only the wet reverberated signal."),
            ("--reverberance PERCENT", "Reverberance amount."),
            ("--hf-damping PERCENT", "High-frequency damping amount."),
            ("--room-scale PERCENT", "Room scale amount."),
            ("--stereo-depth PERCENT", "Stereo depth amount."),
            ("--pre-delay MS", "Pre-delay in milliseconds."),
            ("--wet-gain DB", "Wet gain in dB."),
            ("-o, --output FILE", "Output WAV file to create."),
            ("--backend BACKEND", "Request scalar or simd processing."),
        ],
    },
    ManPage {
        name: "stretch",
        summary: "change duration with windowed stretching",
        synopsis: "auralis stretch INPUT.wav [FACTOR] [--window MS] [--fade SHAPE] [--shift RATIO [--fading RATIO]] -o OUTPUT.wav [--backend BACKEND]",
        description: "Stretch is a recipe alias for basic windowed cross-fade stretching. It lowers to the same typed effect pipeline as `render --fx 'stretch ...'`.",
        options: &[
            ("FACTOR", "Stretch factor."),
            ("--window MS", "Analysis window length in milliseconds."),
            (
                "--fade SHAPE",
                "Fade shape: linear, sqrt, half, or quarter.",
            ),
            ("--shift RATIO", "Window shift ratio."),
            ("--fading RATIO", "Cross-fade ratio."),
            ("-o, --output FILE", "Output WAV file to create."),
            ("--backend BACKEND", "Request scalar or simd processing."),
        ],
    },
    ManPage {
        name: "mix",
        summary: "mix audio files",
        synopsis: "auralis mix INPUT.wav INPUT.wav... -o OUTPUT.wav [--backend BACKEND]",
        description: "Mix is a recipe alias for combining two or more inputs. It lowers to the same typed render pipeline as `render --combine mix --input ...`.",
        options: &[
            ("INPUT", "Two or more WAV input files to mix."),
            ("-o, --output FILE", "Output WAV file to create."),
            ("--backend BACKEND", "Request scalar or simd processing."),
        ],
    },
    ManPage {
        name: "concat",
        summary: "concatenate audio files",
        synopsis: "auralis concat INPUT.wav INPUT.wav... -o OUTPUT.wav [--backend BACKEND]",
        description: "Concat is a recipe alias for joining two or more inputs end-to-end. It lowers to the same typed render pipeline as `render --combine concatenate --input ...`.",
        options: &[
            ("INPUT", "Two or more WAV input files to concatenate."),
            ("-o, --output FILE", "Output WAV file to create."),
            ("--backend BACKEND", "Request scalar or simd processing."),
        ],
    },
    ManPage {
        name: "mix-power",
        summary: "equal-power mix audio files",
        synopsis: "auralis mix-power INPUT.wav INPUT.wav... -o OUTPUT.wav [--backend BACKEND]",
        description: "Mix-power is a recipe alias for combining two or more inputs with equal-power scaling. It lowers to the same typed render pipeline as `render --combine mix-power --input ...`.",
        options: &[
            ("INPUT", "Two or more WAV input files to equal-power mix."),
            ("-o, --output FILE", "Output WAV file to create."),
            ("--backend BACKEND", "Request scalar or simd processing."),
        ],
    },
    ManPage {
        name: "merge",
        summary: "merge audio channels",
        synopsis: "auralis merge INPUT.wav INPUT.wav... -o OUTPUT.wav [--backend BACKEND]",
        description: "Merge is a recipe alias for placing every input channel into one multichannel output. It lowers to the same typed render pipeline as `render --combine merge --input ...`.",
        options: &[
            (
                "INPUT",
                "Two or more WAV input files whose channels should be merged.",
            ),
            ("-o, --output FILE", "Output WAV file to create."),
            ("--backend BACKEND", "Request scalar or simd processing."),
        ],
    },
    ManPage {
        name: "multiply",
        summary: "multiply audio files",
        synopsis: "auralis multiply INPUT.wav INPUT.wav... -o OUTPUT.wav [--backend BACKEND]",
        description: "Multiply is a recipe alias for multiplying corresponding samples from two or more inputs. It lowers to the same typed render pipeline as `render --combine multiply --input ...`.",
        options: &[
            ("INPUT", "Two or more WAV input files to multiply."),
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
    graph_plan::node_display_label(node)
}

fn classify_plan_step(label: &str) -> (graph_plan::PlanStepMode, &'static str) {
    graph_plan::classify_plan_step(label)
}

fn classify_node_plan_mode(node: &spec::CheckedNode) -> (graph_plan::PlanStepMode, &'static str) {
    graph_plan::classify_node_plan_mode(node)
}

fn run_graph_spec(spec: &Path, locked: bool) -> Result<(), CliError> {
    graph_runtime::run_graph_spec(spec, locked)
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

fn parse_filter_poles(value: &str) -> Result<u8, String> {
    match value {
        "1" => Ok(1),
        "2" => Ok(2),
        _ => Err(format!("invalid pole count `{value}`: expected 1 or 2")),
    }
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

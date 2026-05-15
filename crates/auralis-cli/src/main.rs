//! Auralis command-line entrypoint.

mod completions;
mod effect_tokens;
mod executor;
mod graph_commands;
mod graph_plan;
mod graph_runtime;
mod man_pages;
mod recipes;
mod spec;

use std::{
    ffi::OsStr,
    path::{Path, PathBuf},
    process::ExitCode,
};

use auralis::{EffectRegistry, SUPPORTED_EFFECTS};
use auralis_wav::{WavError, decode_pcm16_path};
use clap::{Parser, Subcommand, ValueEnum};
use serde::Serialize;

use completions::{CompletionShell, print_completions};
use executor::{
    ConvertOptions, OutputDither, OutputGuard, RenderOptions, convert_audio, run_pipeline,
};
use graph_commands::{explain_graph_target, format_graph_spec, graph_spec};
use man_pages::print_man_page;
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
        Command::Completions { shell } => {
            print_completions(shell);
            Ok(())
        }
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

//! Auralis command-line entrypoint.

mod command_args;
mod command_support;
mod completions;
mod effect_tokens;
mod errors;
mod executor;
mod graph_commands;
mod graph_plan;
mod graph_runtime;
mod man_pages;
mod parsers;
mod recipes;
mod spec;

use std::{path::PathBuf, process::ExitCode};

use clap::{Parser, Subcommand};

pub(crate) use command_args::GraphFormat;
use command_args::{
    CheckArgs, CompletionsArgs, ConvertArgs, ExplainArgs, FmtArgs, GraphArgs, InspectArgs, ManArgs,
    OpsArgs, PlanArgs, RenderArgs, RunArgs,
};
use command_support::{
    PathRole, check_command, effect_input_to_chain_tokens, inspect, plan_graph_spec, print_ops,
    run_graph_spec,
};
use completions::print_completions;
pub(crate) use errors::CliError;
use executor::{
    ConvertOptions, OutputDither, OutputGuard, RenderOptions, convert_audio, run_pipeline,
};
use graph_commands::{explain_graph_target, format_graph_spec, graph_spec};
use man_pages::print_man_page;
use parsers::{parse_backend, parse_dbfs, parse_filter_poles};
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
    Inspect(InspectArgs),

    /// Convert one supported audio file into another container format.
    Convert(ConvertArgs),

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
    Render(RenderArgs),

    /// Validate typed effect syntax without running audio processing.
    Check(CheckArgs),

    /// Preview the execution shape for an Auralis graph spec.
    Plan(PlanArgs),

    /// Emit an Auralis graph spec as a graph description.
    Graph(GraphArgs),

    /// Format an Auralis graph spec.
    Fmt(FmtArgs),

    /// Generate shell completion scripts.
    Completions(CompletionsArgs),

    /// Print built-in manual pages for Auralis commands.
    Man(ManArgs),

    /// Explain how one graph node or target participates in execution.
    Explain(ExplainArgs),

    /// Run an Auralis graph spec.
    Run(RunArgs),

    /// List implemented typed effects or inspect one effect descriptor.
    Ops(OpsArgs),
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
        Command::Inspect(InspectArgs { input, json }) => inspect(&input, json),
        Command::Convert(ConvertArgs {
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
        }) => convert_audio(
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
        Command::Render(RenderArgs {
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
        }) => {
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
        Command::Check(CheckArgs {
            spec,
            locked,
            effects_file,
            fx,
            chain,
        }) => check_command(
            spec.as_deref(),
            locked,
            effects_file.as_deref(),
            &fx,
            chain.as_deref(),
        ),
        Command::Plan(PlanArgs { spec, json, locked }) => plan_graph_spec(&spec, json, locked),
        Command::Graph(GraphArgs { spec, format }) => graph_spec(&spec, format),
        Command::Fmt(FmtArgs { spec, check }) => format_graph_spec(&spec, check),
        Command::Completions(CompletionsArgs { shell }) => {
            print_completions(shell);
            Ok(())
        }
        Command::Man(ManArgs { topic }) => print_man_page(topic.as_deref()),
        Command::Explain(ExplainArgs { spec, target }) => explain_graph_target(&spec, &target),
        Command::Run(RunArgs { spec, locked }) => run_graph_spec(&spec, locked),
        Command::Ops(OpsArgs { effect, schema }) => print_ops(effect.as_deref(), schema),
    }
}

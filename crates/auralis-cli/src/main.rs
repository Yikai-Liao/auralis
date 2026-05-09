//! Auralis command-line entrypoint.

use std::{
    ffi::OsStr,
    path::{Path, PathBuf},
    process::ExitCode,
};

use auralis_wav::{WavError, decode_pcm16_path};
use clap::{Parser, Subcommand};

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

    /// Decode, process, and re-encode a PCM16 WAV file.
    Run {
        /// PCM16 WAV input file to read.
        input: PathBuf,

        /// PCM16 WAV output file to create.
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

        /// Constant gain to apply, in decibels.
        #[arg(long, value_name = "DB", allow_hyphen_values = true)]
        gain_db: Option<f64>,

        /// Constant normalized DC offset to add, in full-scale sample units.
        #[arg(long, value_name = "SHIFT", allow_hyphen_values = true)]
        dc_shift: Option<f32>,

        /// First frame to keep for an end-exclusive trim.
        #[arg(long, value_name = "FRAME")]
        trim_start_frame: Option<u64>,

        /// End-exclusive frame to keep for a frame-based trim.
        #[arg(long, value_name = "FRAME")]
        trim_end_frame: Option<u64>,

        /// Start time in seconds for an end-exclusive trim.
        #[arg(long, value_name = "SECONDS", allow_hyphen_values = true)]
        trim_start_seconds: Option<f64>,

        /// End time in seconds for a seconds-based trim.
        #[arg(long, value_name = "SECONDS", allow_hyphen_values = true)]
        trim_end_seconds: Option<f64>,

        /// Silent frames to add before the input audio.
        #[arg(long, value_name = "FRAMES")]
        pad_start_frame: Option<u64>,

        /// Silent frames to add after the input audio.
        #[arg(long, value_name = "FRAMES")]
        pad_end_frame: Option<u64>,

        /// Frames over which to linearly fade in from silence.
        #[arg(long, value_name = "FRAMES")]
        fade_in_frame: Option<u64>,

        /// Frames over which to linearly fade out to silence.
        #[arg(long, value_name = "FRAMES")]
        fade_out_frame: Option<u64>,

        /// Reverse frame order within each channel.
        #[arg(long)]
        reverse: bool,

        /// Read the effect chain from a SoX-ng-style effects file.
        #[arg(long, value_name = "FILE")]
        effects_file: Option<PathBuf>,

        /// Positional SoX-ng-style effect chain tokens, such as `gain -3 : reverse`.
        #[arg(value_name = "EFFECT", num_args = 0.., allow_hyphen_values = true)]
        effect_chain: Vec<String>,
    },
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

fn run(cli: Cli) -> Result<(), CliError> {
    match cli.command {
        Command::Inspect { input } => inspect(&input),
        Command::Run {
            input,
            output,
            backend,
            combine,
            additional_inputs,
            gain_db,
            dc_shift,
            trim_start_frame,
            trim_end_frame,
            trim_start_seconds,
            trim_end_seconds,
            pad_start_frame,
            pad_end_frame,
            fade_in_frame,
            fade_out_frame,
            reverse,
            effects_file,
            effect_chain,
        } => {
            let options = RunOptions {
                backend,
                combine,
                additional_inputs,
                gain_db,
                dc_shift,
                trim_start_frame,
                trim_end_frame,
                trim_start_seconds,
                trim_end_seconds,
                pad_start_frame,
                pad_end_frame,
                fade_in_frame,
                fade_out_frame,
                reverse,
                effects_file,
                effect_chain,
            };

            run_pipeline(&input, &output, &options)
        }
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

fn run_pipeline(input: &Path, output: &Path, options: &RunOptions) -> Result<(), CliError> {
    ensure_wav_extension(input, PathRole::Input)?;
    ensure_wav_extension(output, PathRole::Output)?;
    for input in &options.additional_inputs {
        ensure_wav_extension(input, PathRole::Input)?;
    }
    let backend = options.backend;
    let effect_chain = options.effect_chain()?;
    let pipeline = open_pipeline(input, options)?.with_backend(backend);

    if let Some(effect_chain) = effect_chain {
        pipeline
            .apply_effect_chain(&effect_chain)
            .write_wav(output)?;
        return Ok(());
    }

    let trim = options.trim_mode()?;
    let pipeline = if let Some(gain_db) = options.gain_db {
        pipeline.gain_db(gain_db)
    } else {
        pipeline
    };
    let pipeline = if let Some(dc_shift) = options.dc_shift {
        pipeline.dc_shift(dc_shift)
    } else {
        pipeline
    };
    let pipeline = match trim {
        Some(TrimMode::Frames { start, end }) => pipeline.trim_frames(start, end),
        Some(TrimMode::Seconds { start, end }) => pipeline.trim_seconds(start, end),
        None => pipeline,
    };
    let pipeline = match (options.pad_start_frame, options.pad_end_frame) {
        (Some(start), Some(end)) => pipeline.pad_frames(start, end),
        (Some(start), None) => pipeline.pad_frames(start, 0),
        (None, Some(end)) => pipeline.pad_frames(0, end),
        (None, None) => pipeline,
    };
    let pipeline = match (options.fade_in_frame, options.fade_out_frame) {
        (Some(fade_in), Some(fade_out)) => pipeline.fade_frames(fade_in, fade_out),
        (Some(fade_in), None) => pipeline.fade_frames(fade_in, 0),
        (None, Some(fade_out)) => pipeline.fade_frames(0, fade_out),
        (None, None) => pipeline,
    };
    let pipeline = if options.reverse {
        pipeline.reverse()
    } else {
        pipeline
    };

    pipeline.write_wav(output)?;

    Ok(())
}

#[derive(Debug)]
struct RunOptions {
    backend: auralis::BackendKind,
    combine: auralis::CombineMethod,
    additional_inputs: Vec<PathBuf>,
    gain_db: Option<f64>,
    dc_shift: Option<f32>,
    trim_start_frame: Option<u64>,
    trim_end_frame: Option<u64>,
    trim_start_seconds: Option<f64>,
    trim_end_seconds: Option<f64>,
    pad_start_frame: Option<u64>,
    pad_end_frame: Option<u64>,
    fade_in_frame: Option<u64>,
    fade_out_frame: Option<u64>,
    reverse: bool,
    effects_file: Option<PathBuf>,
    effect_chain: Vec<String>,
}

impl RunOptions {
    fn effect_chain(&self) -> Result<Option<auralis::EffectChain>, CliError> {
        let has_positional_chain = !self.effect_chain.is_empty();
        let has_effects_file = self.effects_file.is_some();

        match (has_positional_chain, has_effects_file) {
            (false, false) => return Ok(None),
            (true, true) => return Err(CliError::MixedEffectsFileAndPositionalChain),
            (true, false) if self.has_legacy_effect_options() => {
                return Err(CliError::MixedEffectSyntax);
            }
            (false, true) if self.has_legacy_effect_options() => {
                return Err(CliError::MixedEffectsFileAndLegacyEffectFlags);
            }
            _ => {}
        }

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

    fn has_legacy_effect_options(&self) -> bool {
        self.gain_db.is_some()
            || self.dc_shift.is_some()
            || self.trim_start_frame.is_some()
            || self.trim_end_frame.is_some()
            || self.trim_start_seconds.is_some()
            || self.trim_end_seconds.is_some()
            || self.pad_start_frame.is_some()
            || self.pad_end_frame.is_some()
            || self.fade_in_frame.is_some()
            || self.fade_out_frame.is_some()
            || self.reverse
    }

    fn trim_mode(&self) -> Result<Option<TrimMode>, CliError> {
        let has_frame_trim = self.trim_start_frame.is_some() || self.trim_end_frame.is_some();
        let has_seconds_trim = self.trim_start_seconds.is_some() || self.trim_end_seconds.is_some();

        match (has_frame_trim, has_seconds_trim) {
            (false, false) => Ok(None),
            (true, true) => Err(CliError::MixedTrimUnits),
            (true, false) => match (self.trim_start_frame, self.trim_end_frame) {
                (Some(start), Some(end)) => Ok(Some(TrimMode::Frames { start, end })),
                _ => Err(CliError::IncompleteTrimRange {
                    unit: TrimUnit::Frames,
                }),
            },
            (false, true) => match (self.trim_start_seconds, self.trim_end_seconds) {
                (Some(start), Some(end)) => Ok(Some(TrimMode::Seconds { start, end })),
                _ => Err(CliError::IncompleteTrimRange {
                    unit: TrimUnit::Seconds,
                }),
            },
        }
    }
}

fn open_pipeline(input: &Path, options: &RunOptions) -> Result<auralis::Pipeline, CliError> {
    if options.additional_inputs.is_empty() {
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
        _ => unreachable!("the CLI parser only accepts implemented combine methods"),
    };

    Ok(audio.into_pipeline())
}

#[derive(Debug, Clone, Copy)]
enum TrimMode {
    Frames { start: u64, end: u64 },
    Seconds { start: f64, end: f64 },
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
    auralis::CombineMethod::from_name(value)
        .ok_or_else(|| "combine method must be `concatenate` or `sequence`".to_owned())
}

#[derive(Debug)]
enum CliError {
    Auralis(auralis::Error),
    ChainParse(auralis::EffectChainParseError),
    EffectsFile(auralis::EffectsFileReadError),
    Wav(WavError),
    IncompleteTrimRange { unit: TrimUnit },
    MixedTrimUnits,
    MixedEffectSyntax,
    MixedEffectsFileAndPositionalChain,
    MixedEffectsFileAndLegacyEffectFlags,
    UnsupportedFormat { path: PathBuf, role: PathRole },
}

#[derive(Debug, Clone, Copy)]
enum PathRole {
    Input,
    Output,
}

#[derive(Debug, Clone, Copy)]
enum TrimUnit {
    Frames,
    Seconds,
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
            Self::EffectsFile(error) => write!(formatter, "{error}"),
            Self::Wav(error) => write!(formatter, "{error}"),
            Self::IncompleteTrimRange { unit } => match unit {
                TrimUnit::Frames => formatter
                    .write_str("frame trim requires both --trim-start-frame and --trim-end-frame"),
                TrimUnit::Seconds => formatter.write_str(
                    "seconds trim requires both --trim-start-seconds and --trim-end-seconds",
                ),
            },
            Self::MixedTrimUnits => formatter
                .write_str("trim range must use either frame units or seconds units, not both"),
            Self::MixedEffectSyntax => formatter
                .write_str("positional effect chains cannot be combined with legacy effect flags"),
            Self::MixedEffectsFileAndPositionalChain => formatter
                .write_str("effects files cannot be combined with positional effect chain tokens"),
            Self::MixedEffectsFileAndLegacyEffectFlags => {
                formatter.write_str("effects files cannot be combined with legacy effect flags")
            }
            Self::UnsupportedFormat { path, role } => {
                write!(
                    formatter,
                    "unsupported {role} format for {}; only PCM16 WAV is supported",
                    path.display()
                )
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

impl From<auralis::EffectsFileReadError> for CliError {
    fn from(error: auralis::EffectsFileReadError) -> Self {
        Self::EffectsFile(error)
    }
}

impl From<WavError> for CliError {
    fn from(error: WavError) -> Self {
        Self::Wav(error)
    }
}

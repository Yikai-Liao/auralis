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

        /// Constant gain to apply, in decibels.
        #[arg(long, value_name = "DB", allow_hyphen_values = true)]
        gain_db: Option<f64>,

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
            gain_db,
            trim_start_frame,
            trim_end_frame,
            trim_start_seconds,
            trim_end_seconds,
            pad_start_frame,
            pad_end_frame,
        } => run_pipeline(
            &input,
            &output,
            RunOptions {
                gain_db,
                trim_start_frame,
                trim_end_frame,
                trim_start_seconds,
                trim_end_seconds,
                pad_start_frame,
                pad_end_frame,
            },
        ),
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

fn run_pipeline(input: &Path, output: &Path, options: RunOptions) -> Result<(), CliError> {
    ensure_wav_extension(input, PathRole::Input)?;
    ensure_wav_extension(output, PathRole::Output)?;
    let trim = options.trim_mode()?;
    let pipeline = auralis::AudioFile::open_wav(input)?.into_pipeline();
    let pipeline = if let Some(gain_db) = options.gain_db {
        pipeline.gain_db(gain_db)
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

    pipeline.write_wav(output)?;

    Ok(())
}

#[derive(Debug, Clone, Copy)]
struct RunOptions {
    gain_db: Option<f64>,
    trim_start_frame: Option<u64>,
    trim_end_frame: Option<u64>,
    trim_start_seconds: Option<f64>,
    trim_end_seconds: Option<f64>,
    pad_start_frame: Option<u64>,
    pad_end_frame: Option<u64>,
}

impl RunOptions {
    fn trim_mode(self) -> Result<Option<TrimMode>, CliError> {
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

#[derive(Debug)]
enum CliError {
    Auralis(auralis::Error),
    Wav(WavError),
    IncompleteTrimRange { unit: TrimUnit },
    MixedTrimUnits,
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

impl From<WavError> for CliError {
    fn from(error: WavError) -> Self {
        Self::Wav(error)
    }
}

//! Auralis command-line entrypoint.

use std::{
    ffi::OsStr,
    path::{Path, PathBuf},
    process::ExitCode,
};

use auralis_wav::{WavError, decode_pcm16_path, encode_pcm16_path};
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

    /// Decode and re-encode a PCM16 WAV file without applying effects.
    Run {
        /// PCM16 WAV input file to read.
        input: PathBuf,

        /// PCM16 WAV output file to create.
        output: PathBuf,
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
        Command::Run { input, output } => run_copy_pipeline(&input, &output),
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

fn run_copy_pipeline(input: &Path, output: &Path) -> Result<(), CliError> {
    ensure_wav_extension(input, PathRole::Input)?;
    ensure_wav_extension(output, PathRole::Output)?;
    let audio = decode_pcm16_path(input)?;
    encode_pcm16_path(output, &audio)?;

    Ok(())
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
    Wav(WavError),
    UnsupportedFormat { path: PathBuf, role: PathRole },
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
            Self::Wav(error) => write!(formatter, "{error}"),
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

impl From<WavError> for CliError {
    fn from(error: WavError) -> Self {
        Self::Wav(error)
    }
}

//! Auralis command-line entrypoint.

use std::{
    ffi::OsStr,
    path::{Path, PathBuf},
    process::ExitCode,
};

use auralis::{EffectRegistry, SUPPORTED_EFFECTS};
use auralis_wav::{decode_pcm16_path, WavError};
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

    /// List implemented typed effects or inspect one effect descriptor.
    Ops {
        /// Optional canonical effect name or alias to inspect.
        effect: Option<String>,
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
            let options = RunOptions {
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
                gain_db: None,
                dc_shift: None,
                trim_start_frame: None,
                trim_end_frame: None,
                trim_start_seconds: None,
                trim_end_seconds: None,
                pad_start_frame: None,
                pad_end_frame: None,
                fade_in_frame: None,
                fade_out_frame: None,
                reverse: false,
                effects_file,
                effect_chain: effect_input_to_chain_tokens(&fx, chain.as_deref())?,
            };

            run_pipeline(&input, &output, &options)
        }
        Command::Check {
            effects_file,
            fx,
            chain,
        } => check_effects(effects_file.as_deref(), &fx, chain.as_deref()),
        Command::Ops { effect } => print_ops(effect.as_deref()),
        Command::Run {
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
                output_channels,
                no_auto_channels,
                output_sample_rate,
                no_auto_rate,
                guard: OutputGuard::from(guard),
                norm,
                dither: OutputDither::from(dither),
                dither_seed,
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

fn run_pipeline(input: &Path, output: &Path, options: &RunOptions) -> Result<(), CliError> {
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
struct RunOptions {
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
    options: &RunOptions,
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
enum TrimMode {
    Frames { start: u64, end: u64 },
    Seconds { start: f64, end: f64 },
}

#[derive(Debug, Clone, Copy)]
enum OutputGuard {
    Disabled,
    Enabled,
}

impl From<bool> for OutputGuard {
    fn from(value: bool) -> Self {
        if value {
            Self::Enabled
        } else {
            Self::Disabled
        }
    }
}

#[derive(Debug, Clone, Copy)]
enum OutputDither {
    Disabled,
    Enabled,
}

impl From<bool> for OutputDither {
    fn from(value: bool) -> Self {
        if value {
            Self::Enabled
        } else {
            Self::Disabled
        }
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
    Wav(WavError),
    EmptyEffectSpec,
    InvalidEffectSpec { spec: String },
    IncompleteTrimRange { unit: TrimUnit },
    MissingEffectSpec,
    MixedEffectInputs,
    MixedTrimUnits,
    MixedEffectSyntax,
    MixedEffectsFileAndPositionalChain,
    MixedEffectsFileAndLegacyEffectFlags,
    MixedGuardAndNorm,
    DitherSeedWithoutDither,
    NoAutoChannelsWithoutOutputChannels,
    NoAutoRateWithoutOutputRate,
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
            Self::EffectName(error) => write!(formatter, "{error}"),
            Self::EffectsFile(error) => write!(formatter, "{error}"),
            Self::Wav(error) => write!(formatter, "{error}"),
            Self::EmptyEffectSpec => formatter.write_str("effect input requires a non-empty effect"),
            Self::InvalidEffectSpec { spec } => write!(
                formatter,
                "effect string `{spec}` contains unmatched shell quoting"
            ),
            Self::IncompleteTrimRange { unit } => match unit {
                TrimUnit::Frames => formatter
                    .write_str("frame trim requires both --trim-start-frame and --trim-end-frame"),
                TrimUnit::Seconds => formatter.write_str(
                    "seconds trim requires both --trim-start-seconds and --trim-end-seconds",
                ),
            },
            Self::MissingEffectSpec => {
                formatter.write_str("one of --fx, --chain, or --effects-file is required")
            }
            Self::MixedEffectInputs => {
                formatter.write_str("--fx, --chain, and --effects-file are mutually exclusive")
            }
            Self::MixedTrimUnits => formatter
                .write_str("trim range must use either frame units or seconds units, not both"),
            Self::MixedEffectSyntax => formatter
                .write_str("positional effect chains cannot be combined with legacy effect flags"),
            Self::MixedEffectsFileAndPositionalChain => formatter
                .write_str("effects files cannot be combined with positional effect chain tokens"),
            Self::MixedEffectsFileAndLegacyEffectFlags => {
                formatter.write_str("effects files cannot be combined with legacy effect flags")
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

impl From<WavError> for CliError {
    fn from(error: WavError) -> Self {
        Self::Wav(error)
    }
}

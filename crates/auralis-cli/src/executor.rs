use std::{
    ffi::OsStr,
    path::{Path, PathBuf},
};

use auralis_wav::WavError;

use crate::{
    CliError,
    command_support::{PathRole, ensure_wav_extension},
};

#[derive(Debug)]
pub(super) struct RenderOptions {
    pub(super) backend: auralis::BackendKind,
    pub(super) combine: auralis::CombineMethod,
    pub(super) additional_inputs: Vec<PathBuf>,
    pub(super) output_channels: Option<auralis::ChannelCount>,
    pub(super) no_auto_channels: bool,
    pub(super) output_sample_rate: Option<auralis::SampleRate>,
    pub(super) no_auto_rate: bool,
    pub(super) guard: OutputGuard,
    pub(super) norm: Option<f64>,
    pub(super) dither: OutputDither,
    pub(super) dither_seed: Option<u32>,
    pub(super) effects_file: Option<PathBuf>,
    pub(super) effect_chain: Vec<String>,
}

#[derive(Debug, Clone, Copy)]
pub(super) struct ConvertOptions {
    pub(super) backend: auralis::BackendKind,
    pub(super) output_channels: Option<auralis::ChannelCount>,
    pub(super) no_auto_channels: bool,
    pub(super) output_sample_rate: Option<auralis::SampleRate>,
    pub(super) no_auto_rate: bool,
    pub(super) guard: OutputGuard,
    pub(super) norm: Option<f64>,
    pub(super) sample: Option<auralis::WavSampleFormat>,
}

#[derive(Debug, Clone, Copy)]
pub(super) enum OutputGuard {
    Disabled,
    Enabled,
}

impl From<bool> for OutputGuard {
    fn from(value: bool) -> Self {
        if value { Self::Enabled } else { Self::Disabled }
    }
}

#[derive(Debug, Clone, Copy)]
pub(super) enum OutputDither {
    Disabled,
    Enabled,
}

impl From<bool> for OutputDither {
    fn from(value: bool) -> Self {
        if value { Self::Enabled } else { Self::Disabled }
    }
}

pub(super) fn convert_audio(
    input: &Path,
    output: &Path,
    options: ConvertOptions,
) -> Result<(), CliError> {
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

pub(super) fn run_pipeline(
    input: &Path,
    output: &Path,
    options: &RenderOptions,
) -> Result<(), CliError> {
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

    write_pipeline_with_effect_chain(pipeline, output, effect_chain.as_ref())?;

    Ok(())
}

pub(super) fn apply_effect_tokens_to_buffer(
    input: auralis::AudioBuffer,
    tokens: &[String],
) -> Result<auralis::AudioBuffer, CliError> {
    let token_refs: Vec<&str> = tokens.iter().map(String::as_str).collect();
    apply_effect_token_refs_to_buffer(input, &token_refs)
}

pub(super) fn apply_effect_token_refs_to_buffer(
    input: auralis::AudioBuffer,
    tokens: &[&str],
) -> Result<auralis::AudioBuffer, CliError> {
    let effect_chain = auralis::parse_effect_chain(tokens)?;
    auralis::AudioFile::from_audio_buffer(input)
        .into_pipeline()
        .apply_effect_chain(&effect_chain)
        .into_audio_buffer()
        .map_err(CliError::from)
}

fn write_pipeline_with_effect_chain(
    pipeline: auralis::Pipeline,
    output: &Path,
    effect_chain: Option<&auralis::EffectChain>,
) -> Result<(), CliError> {
    match effect_chain {
        Some(effect_chain) => pipeline
            .apply_effect_chain(effect_chain)
            .write_wav(output)?,
        None => pipeline.write_wav(output)?,
    }

    Ok(())
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

impl RenderOptions {
    fn effect_chain(&self) -> Result<Option<auralis::EffectChain>, CliError> {
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
}

fn open_pipeline(
    input: &Path,
    options: &RenderOptions,
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

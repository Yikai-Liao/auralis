use crate::{AudioBuffer, AudioSpec, BackendKind, ChannelCount};
use thiserror::Error;

/// Explicit policy for changing channel count at an output boundary.
///
/// Auralis library APIs never change channel count implicitly. The default
/// [`Self::Preserve`] policy writes the current pipeline channel count.
/// [`Self::Automatic`] applies SoX-ng-style `channels` conversion to the target
/// count before encoding. [`Self::Require`] is useful for tests and strict
/// callers: it records an expected output count while failing instead of
/// automatically inserting conversion when the pipeline count differs.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum ChannelConversionPolicy {
    /// Preserve the current pipeline channel count.
    Preserve,

    /// Convert to the target channel count before writing.
    Automatic(ChannelCount),

    /// Require the target channel count without automatic conversion.
    Require(ChannelCount),
}

impl ChannelConversionPolicy {
    /// Returns the default policy: preserve the current channel count.
    #[must_use]
    pub const fn preserve() -> Self {
        Self::Preserve
    }

    /// Returns a policy that automatically converts to `target`.
    #[must_use]
    pub const fn automatic(target: ChannelCount) -> Self {
        Self::Automatic(target)
    }

    /// Returns a policy that requires `target` without automatic conversion.
    #[must_use]
    pub const fn require(target: ChannelCount) -> Self {
        Self::Require(target)
    }

    /// Returns the target output channel count when one is configured.
    #[must_use]
    pub const fn target_channels(self) -> Option<ChannelCount> {
        match self {
            Self::Preserve => None,
            Self::Automatic(target) | Self::Require(target) => Some(target),
        }
    }

    /// Returns whether this policy may insert channel conversion.
    #[must_use]
    pub const fn automatic_conversion_enabled(self) -> bool {
        matches!(self, Self::Automatic(_))
    }
}

/// Errors produced by explicit output channel conversion.
#[derive(Debug, Clone, PartialEq, Error)]
#[non_exhaustive]
pub enum ChannelConversionError {
    /// A target channel count was requested while automatic conversion was disabled.
    #[error("automatic channel conversion from {actual} to {target} is disabled")]
    AutomaticConversionDisabled {
        /// Channel count currently present in the pipeline.
        actual: ChannelCount,

        /// Required output channel count.
        target: ChannelCount,
    },

    /// The converted buffer shape was rejected by the core buffer model.
    #[error(transparent)]
    Core(#[from] auralis_core::AuralisError),

    /// A backend-dispatched downmix kernel rejected channel slices.
    #[error(transparent)]
    Mix(#[from] auralis_simd::MixError),
}

/// Converts a decoded planar buffer to `target_channels` with SoX-ng `channels` semantics.
///
/// When the target matches the input channel count, the buffer is cloned
/// unchanged. Downmixing averages deterministic input-channel groups in the
/// same pattern as SoX-ng's automatic `channels` effect. Upmixing duplicates
/// input channels in round-robin order. The transform preserves sample rate,
/// sample format, frame count, and sample values except for required downmix
/// averaging. The scalar backend is used as the reference path.
///
/// # Errors
///
/// Returns [`ChannelConversionError::Core`] if the converted buffer shape
/// cannot be represented, or [`ChannelConversionError::Mix`] if a backend
/// downmix kernel rejects the generated channel slices.
pub fn convert_audio_channels(
    audio: &AudioBuffer,
    target_channels: ChannelCount,
) -> std::result::Result<AudioBuffer, ChannelConversionError> {
    convert_audio_channels_with_backend(audio, target_channels, BackendKind::Scalar)
}

/// Converts a decoded planar buffer to `target_channels` with a requested backend.
///
/// `requested_backend` selects the scalar/SIMD mix kernel used for downmixing
/// through the same deterministic backend fallback rules used by other
/// backend-aware sample processing. Upmixing is a structural copy and does not
/// select a SIMD kernel.
///
/// # Errors
///
/// Returns the same errors as [`convert_audio_channels`].
pub fn convert_audio_channels_with_backend(
    audio: &AudioBuffer,
    target_channels: ChannelCount,
    requested_backend: BackendKind,
) -> std::result::Result<AudioBuffer, ChannelConversionError> {
    let input_channels = audio.channels();
    if input_channels == target_channels {
        return Ok(audio.clone());
    }

    let output_spec = AudioSpec::new(
        audio.spec().sample_rate(),
        target_channels,
        audio.spec().sample_format(),
    );
    let mut output = AudioBuffer::zeroed(output_spec, audio.frames())?;

    if input_channels < target_channels {
        duplicate_channels(audio, &mut output)?;
    } else {
        downmix_channels_with_backend(audio, &mut output, requested_backend)?;
    }

    Ok(output)
}

pub(crate) fn apply_channel_conversion_policy_with_backend(
    audio: AudioBuffer,
    policy: ChannelConversionPolicy,
    requested_backend: BackendKind,
) -> std::result::Result<AudioBuffer, ChannelConversionError> {
    match policy {
        ChannelConversionPolicy::Preserve => Ok(audio),
        ChannelConversionPolicy::Automatic(target) => {
            convert_audio_channels_with_backend(&audio, target, requested_backend)
        }
        ChannelConversionPolicy::Require(target) if audio.channels() == target => Ok(audio),
        ChannelConversionPolicy::Require(target) => {
            Err(ChannelConversionError::AutomaticConversionDisabled {
                actual: audio.channels(),
                target,
            })
        }
    }
}

fn duplicate_channels(
    input: &AudioBuffer,
    output: &mut AudioBuffer,
) -> std::result::Result<(), ChannelConversionError> {
    let input_channels = input.channels().as_usize();
    for output_channel_index in 0..output.channels().as_usize() {
        let input_channel_index = output_channel_index % input_channels;
        let input_channel = input
            .channel(input_channel_index)
            .ok_or(auralis_core::AuralisError::InvalidAudioBufferShape)?;
        let output_channel = output
            .channel_mut(output_channel_index)
            .ok_or(auralis_core::AuralisError::InvalidAudioBufferShape)?;
        output_channel.copy_from_slice(input_channel);
    }

    Ok(())
}

fn downmix_channels_with_backend(
    input: &AudioBuffer,
    output: &mut AudioBuffer,
    requested_backend: BackendKind,
) -> std::result::Result<(), ChannelConversionError> {
    let input_channels = input.channels().as_usize();
    let output_channels = output.channels().as_usize();
    let selection = auralis_simd::select_backend(requested_backend);

    for output_channel_index in 0..output_channels {
        let input_channels_per_output =
            (input_channels + output_channels - 1 - output_channel_index) / output_channels;
        let mut channel_inputs = Vec::with_capacity(input_channels_per_output);
        for input_group_index in 0..input_channels_per_output {
            let input_channel_index = input_group_index * output_channels + output_channel_index;
            channel_inputs.push(
                input
                    .channel(input_channel_index)
                    .ok_or(auralis_core::AuralisError::InvalidAudioBufferShape)?,
            );
        }

        let output_channel = output
            .channel_mut(output_channel_index)
            .ok_or(auralis_core::AuralisError::InvalidAudioBufferShape)?;
        auralis_simd::mix_f32_with_backend(
            selection,
            &channel_inputs,
            output_channel,
            reciprocal_usize(input_channels_per_output),
        )?;
    }

    Ok(())
}

fn reciprocal_usize(value: usize) -> f32 {
    #[allow(
        clippy::cast_precision_loss,
        reason = "Channel conversion scales small channel groups as f32 sample arithmetic."
    )]
    {
        1.0 / value as f32
    }
}

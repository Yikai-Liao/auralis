use crate::{AudioBuffer, AudioSpec, FrameCount, SampleRate};
use thiserror::Error;

/// Explicit policy for changing sample rate at an output boundary.
///
/// Auralis library APIs never change sample rate implicitly. The default
/// [`Self::Preserve`] policy writes the current pipeline sample rate.
/// [`Self::Automatic`] applies Auralis' deterministic scalar output-boundary
/// resampler before encoding. [`Self::Require`] is useful for tests and strict
/// callers: it records an expected output rate while failing instead of
/// automatically inserting conversion when the pipeline rate differs.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum SampleRateConversionPolicy {
    /// Preserve the current pipeline sample rate.
    Preserve,

    /// Convert to the target sample rate before writing.
    Automatic(SampleRate),

    /// Require the target sample rate without automatic conversion.
    Require(SampleRate),
}

impl SampleRateConversionPolicy {
    /// Returns the default policy: preserve the current sample rate.
    #[must_use]
    pub const fn preserve() -> Self {
        Self::Preserve
    }

    /// Returns a policy that automatically converts to `target`.
    #[must_use]
    pub const fn automatic(target: SampleRate) -> Self {
        Self::Automatic(target)
    }

    /// Returns a policy that requires `target` without automatic conversion.
    #[must_use]
    pub const fn require(target: SampleRate) -> Self {
        Self::Require(target)
    }

    /// Returns the target output sample rate when one is configured.
    #[must_use]
    pub const fn target_sample_rate(self) -> Option<SampleRate> {
        match self {
            Self::Preserve => None,
            Self::Automatic(target) | Self::Require(target) => Some(target),
        }
    }

    /// Returns whether this policy may insert sample-rate conversion.
    #[must_use]
    pub const fn automatic_conversion_enabled(self) -> bool {
        matches!(self, Self::Automatic(_))
    }
}

/// Errors produced by explicit output sample-rate conversion.
#[derive(Debug, Clone, PartialEq, Error)]
#[non_exhaustive]
pub enum SampleRateConversionError {
    /// A target sample rate was requested while automatic conversion was disabled.
    #[error("automatic sample-rate conversion from {actual} to {target} is disabled")]
    AutomaticConversionDisabled {
        /// Sample rate currently present in the pipeline.
        actual: SampleRate,

        /// Required output sample rate.
        target: SampleRate,
    },

    /// The converted frame count could not be represented.
    #[error("sample-rate conversion from {actual} to {target} overflows for {frames}")]
    FrameCountOverflow {
        /// Frame count currently present in the pipeline.
        frames: FrameCount,

        /// Sample rate currently present in the pipeline.
        actual: SampleRate,

        /// Required output sample rate.
        target: SampleRate,
    },

    /// The converted buffer shape was rejected by the core buffer model.
    #[error(transparent)]
    Core(#[from] auralis_core::AuralisError),
}

/// Converts a decoded planar buffer to `target_sample_rate`.
///
/// When the target matches the input sample rate, the buffer is cloned
/// unchanged. Otherwise this output-boundary converter preserves channel count
/// and sample format, computes `round(input_frames * target / source)` output
/// frames, and samples each output channel with deterministic linear
/// interpolation at `output_frame * source_rate / target_rate`. This is a
/// scalar policy implementation for SoX-ng-style automatic output-rate
/// insertion; the fuller user-visible `rate` effect and quality modes are
/// tracked by later DEVELOPMENT.md features.
///
/// # Errors
///
/// Returns [`SampleRateConversionError::FrameCountOverflow`] when the converted
/// frame count cannot fit in `u64`, or [`SampleRateConversionError::Core`] if
/// the converted buffer shape cannot be represented.
pub fn convert_audio_sample_rate(
    audio: &AudioBuffer,
    target_sample_rate: SampleRate,
) -> std::result::Result<AudioBuffer, SampleRateConversionError> {
    let input_sample_rate = audio.spec().sample_rate();
    if input_sample_rate == target_sample_rate {
        return Ok(audio.clone());
    }

    let output_frames =
        converted_sample_rate_frames(audio.frames(), input_sample_rate, target_sample_rate)?;
    let output_spec = AudioSpec::new(
        target_sample_rate,
        audio.channels(),
        audio.spec().sample_format(),
    );
    let mut output = AudioBuffer::zeroed(output_spec, output_frames)?;

    for channel_index in 0..audio.channels().as_usize() {
        let input_channel = audio
            .channel(channel_index)
            .ok_or(auralis_core::AuralisError::InvalidAudioBufferShape)?;
        let output_channel = output
            .channel_mut(channel_index)
            .ok_or(auralis_core::AuralisError::InvalidAudioBufferShape)?;
        resample_channel_linear(
            input_channel,
            output_channel,
            input_sample_rate,
            target_sample_rate,
        );
    }

    Ok(output)
}

pub(crate) fn apply_sample_rate_conversion_policy(
    audio: AudioBuffer,
    policy: SampleRateConversionPolicy,
) -> std::result::Result<AudioBuffer, SampleRateConversionError> {
    match policy {
        SampleRateConversionPolicy::Preserve => Ok(audio),
        SampleRateConversionPolicy::Automatic(target) => convert_audio_sample_rate(&audio, target),
        SampleRateConversionPolicy::Require(target) if audio.spec().sample_rate() == target => {
            Ok(audio)
        }
        SampleRateConversionPolicy::Require(target) => {
            Err(SampleRateConversionError::AutomaticConversionDisabled {
                actual: audio.spec().sample_rate(),
                target,
            })
        }
    }
}

fn converted_sample_rate_frames(
    frames: FrameCount,
    actual: SampleRate,
    target: SampleRate,
) -> std::result::Result<FrameCount, SampleRateConversionError> {
    let frame_count = frames.as_u64();
    if frame_count == 0 {
        return Ok(FrameCount::new(0));
    }

    let numerator = u128::from(frame_count)
        .checked_mul(u128::from(target.as_u32()))
        .ok_or(SampleRateConversionError::FrameCountOverflow {
            frames,
            actual,
            target,
        })?;
    let denominator = u128::from(actual.as_u32());
    let rounded = numerator.checked_add(denominator / 2).ok_or(
        SampleRateConversionError::FrameCountOverflow {
            frames,
            actual,
            target,
        },
    )? / denominator;
    let output_frames =
        u64::try_from(rounded).map_err(|_| SampleRateConversionError::FrameCountOverflow {
            frames,
            actual,
            target,
        })?;

    Ok(FrameCount::new(output_frames))
}

fn resample_channel_linear(
    input: &[f32],
    output: &mut [f32],
    actual: SampleRate,
    target: SampleRate,
) {
    if input.is_empty() || output.is_empty() {
        return;
    }

    let last_input_index = input.len() - 1;
    let actual = f64::from(actual.as_u32());
    let target = f64::from(target.as_u32());
    for (output_index, output_sample) in output.iter_mut().enumerate() {
        #[allow(
            clippy::cast_precision_loss,
            reason = "Sample-rate conversion maps usize frame positions into f64 time coordinates."
        )]
        let source_position = output_index as f64 * actual / target;
        #[allow(
            clippy::cast_possible_truncation,
            clippy::cast_sign_loss,
            reason = "source_position is finite and non-negative because sample rates are positive."
        )]
        let source_floor_index = source_position.floor() as usize;

        if source_floor_index >= last_input_index {
            *output_sample = input[last_input_index];
            continue;
        }

        #[allow(
            clippy::cast_precision_loss,
            reason = "The source frame index is subtracted in the same f64 coordinate system used for interpolation."
        )]
        let source_floor = source_floor_index as f64;
        let fraction = source_position - source_floor;
        #[allow(
            clippy::cast_possible_truncation,
            reason = "The interpolation fraction is in 0.0..1.0 and represented as f32 sample arithmetic."
        )]
        let fraction = fraction as f32;
        let left = input[source_floor_index];
        let right = input[source_floor_index + 1];
        *output_sample = left.mul_add(1.0 - fraction, right * fraction);
    }
}

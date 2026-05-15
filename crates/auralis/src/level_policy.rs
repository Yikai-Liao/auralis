use crate::{AudioBuffer, BackendKind, Decibels};
use thiserror::Error;

/// Explicit policy for changing sample levels at an output boundary.
///
/// Auralis library APIs never adjust final level implicitly. The default
/// [`Self::Preserve`] policy lets the PCM16 encoder apply its documented
/// clipping. [`Self::Guard`] attenuates the final buffer only when its absolute
/// peak would exceed full scale. [`Self::Normalize`] scales non-silent audio so
/// its absolute peak reaches the requested level before encoding.
#[derive(Debug, Clone, Copy, PartialEq)]
#[non_exhaustive]
pub enum OutputLevelPolicy {
    /// Preserve current sample levels and let the encoder clip if needed.
    Preserve,

    /// Attenuate only when the output peak exceeds full scale.
    Guard,

    /// Scale non-silent output so its peak reaches the requested level.
    Normalize(Decibels),
}

impl OutputLevelPolicy {
    /// Returns the default policy: preserve current sample levels.
    #[must_use]
    pub const fn preserve() -> Self {
        Self::Preserve
    }

    /// Returns a policy that attenuates output only when clipping would occur.
    #[must_use]
    pub const fn guard() -> Self {
        Self::Guard
    }

    /// Returns a policy that normalizes output to `target`.
    #[must_use]
    pub const fn normalize(target: Decibels) -> Self {
        Self::Normalize(target)
    }

    /// Returns whether this policy may change sample values.
    #[must_use]
    pub const fn adjustment_enabled(self) -> bool {
        !matches!(self, Self::Preserve)
    }
}

/// Errors produced by explicit output level adjustment.
#[derive(Debug, Clone, PartialEq, Error)]
#[non_exhaustive]
pub enum OutputLevelError {
    /// A non-finite sample prevented deterministic peak scanning.
    #[error(
        "output level policy encountered non-finite sample at channel {channel_index}, frame {frame_index}"
    )]
    NonFiniteSample {
        /// Zero-based channel index containing the non-finite sample.
        channel_index: usize,

        /// Zero-based frame index containing the non-finite sample.
        frame_index: u64,
    },

    /// A target level was too large to become a finite `f32` multiplier.
    #[error("normalization target {target} is too large for finite f32 sample scaling")]
    TargetLevelOverflow {
        /// Requested normalization target.
        target: Decibels,
    },

    /// The adjusted buffer shape was rejected by the core buffer model.
    #[error(transparent)]
    Core(#[from] auralis_core::AuralisError),
}

/// Attenuates a decoded planar buffer only when its absolute peak exceeds full scale.
///
/// This is the library counterpart to `auralis render ... --guard`. It scans
/// finite samples, clones the input when the peak is already within
/// `[-1.0, 1.0]`, and otherwise applies a deterministic `1 / peak` linear
/// scale to every channel using the scalar backend.
///
/// # Errors
///
/// Returns [`OutputLevelError::NonFiniteSample`] when a sample is NaN or
/// infinite, or [`OutputLevelError::Core`] if the cloned buffer shape is
/// rejected while applying the adjustment.
pub fn guard_audio_level(
    audio: &AudioBuffer,
) -> std::result::Result<AudioBuffer, OutputLevelError> {
    guard_audio_level_with_backend(audio, BackendKind::Scalar)
}

/// Attenuates a decoded planar buffer only when its absolute peak exceeds full scale.
///
/// `requested_backend` selects the scalar/SIMD gain kernel used for any
/// required attenuation. When no attenuation is needed, no backend kernel runs.
///
/// # Errors
///
/// Returns the same errors as [`guard_audio_level`].
pub fn guard_audio_level_with_backend(
    audio: &AudioBuffer,
    requested_backend: BackendKind,
) -> std::result::Result<AudioBuffer, OutputLevelError> {
    let peak = absolute_peak(audio)?;
    if peak <= 1.0 {
        return Ok(audio.clone());
    }

    let mut output = audio.clone();
    apply_output_level_multiplier(&mut output, 1.0 / peak, requested_backend)?;
    Ok(output)
}

/// Normalizes a decoded planar buffer to a target full-scale peak.
///
/// This is the library counterpart to `auralis render ... --norm[=DB]`.
/// Silent input is returned unchanged because there is no finite multiplier
/// that can create a peak from silence. Non-silent input is scaled so its
/// absolute peak reaches `target`.
///
/// # Errors
///
/// Returns [`OutputLevelError::NonFiniteSample`] when a sample is NaN or
/// infinite, [`OutputLevelError::TargetLevelOverflow`] when `target` is too
/// large to become a finite `f32` scale, or [`OutputLevelError::Core`] if the
/// cloned buffer shape is rejected while applying the adjustment.
pub fn normalize_audio_level(
    audio: &AudioBuffer,
    target: Decibels,
) -> std::result::Result<AudioBuffer, OutputLevelError> {
    normalize_audio_level_with_backend(audio, target, BackendKind::Scalar)
}

/// Normalizes a decoded planar buffer to a target full-scale peak with a requested backend.
///
/// `requested_backend` selects the scalar/SIMD gain kernel used for non-silent
/// normalization. Unsupported SIMD requests follow the same deterministic
/// fallback metadata as other backend-aware sample processing.
///
/// # Errors
///
/// Returns the same errors as [`normalize_audio_level`].
pub fn normalize_audio_level_with_backend(
    audio: &AudioBuffer,
    target: Decibels,
    requested_backend: BackendKind,
) -> std::result::Result<AudioBuffer, OutputLevelError> {
    let peak = absolute_peak(audio)?;
    if peak == 0.0 {
        return Ok(audio.clone());
    }

    let target = output_level_target_linear(target)?;
    let mut output = audio.clone();
    apply_output_level_multiplier(&mut output, target / peak, requested_backend)?;
    Ok(output)
}

pub(crate) fn apply_output_level_policy_with_backend(
    audio: AudioBuffer,
    policy: OutputLevelPolicy,
    requested_backend: BackendKind,
) -> std::result::Result<AudioBuffer, OutputLevelError> {
    match policy {
        OutputLevelPolicy::Preserve => Ok(audio),
        OutputLevelPolicy::Guard => guard_audio_level_with_backend(&audio, requested_backend),
        OutputLevelPolicy::Normalize(target) => {
            normalize_audio_level_with_backend(&audio, target, requested_backend)
        }
    }
}

fn absolute_peak(audio: &AudioBuffer) -> std::result::Result<f32, OutputLevelError> {
    let mut peak = 0.0_f32;
    for channel_index in 0..audio.channels().as_usize() {
        let channel = audio
            .channel(channel_index)
            .ok_or(auralis_core::AuralisError::InvalidAudioBufferShape)?;
        for (frame_index, &sample) in channel.iter().enumerate() {
            if !sample.is_finite() {
                return Err(OutputLevelError::NonFiniteSample {
                    channel_index,
                    frame_index: u64::try_from(frame_index)
                        .map_err(|_| auralis_core::AuralisError::InvalidAudioBufferShape)?,
                });
            }
            peak = peak.max(sample.abs());
        }
    }

    Ok(peak)
}

fn output_level_target_linear(target: Decibels) -> std::result::Result<f32, OutputLevelError> {
    let linear = 10.0_f64.powf(target.as_f64() / 20.0);
    if !linear.is_finite() || linear > f64::from(f32::MAX) {
        return Err(OutputLevelError::TargetLevelOverflow { target });
    }

    #[allow(
        clippy::cast_possible_truncation,
        reason = "the public DSP sample format is f32, so the f64 unit value is intentionally rounded once"
    )]
    Ok(linear as f32)
}

fn apply_output_level_multiplier(
    audio: &mut AudioBuffer,
    multiplier: f32,
    requested_backend: BackendKind,
) -> std::result::Result<(), OutputLevelError> {
    if multiplier.to_bits() == 1.0_f32.to_bits() {
        return Ok(());
    }

    let selection = auralis_simd::select_backend(requested_backend);
    for channel_index in 0..audio.channels().as_usize() {
        let channel = audio
            .channel_mut(channel_index)
            .ok_or(auralis_core::AuralisError::InvalidAudioBufferShape)?;
        auralis_simd::gain_f32_in_place_with_backend(selection, channel, multiplier);
    }

    Ok(())
}

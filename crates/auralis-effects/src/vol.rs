use auralis_core::{AudioBuffer, Decibels};
use auralis_dsp::linear_gain;
use auralis_simd::{BackendKind, gain_f32_in_place_with_backend, select_backend};

use crate::{EffectError, Result};

/// Unit used to interpret a SoX-ng `vol` gain argument.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum VolGainType {
    /// Linear amplitude multiplier.
    Amplitude,

    /// Power multiplier, converted to amplitude with a signed square root.
    Power,

    /// Decibel gain, converted to amplitude with `10^(dB / 20)`.
    Decibels,
}

/// SoX-ng-style `vol` effect processor.
///
/// `Vol` applies a linear amplitude multiplier derived from the configured
/// gain type. Unlike [`crate::Gain`], SoX-ng `vol` clips its output inside the
/// effect. Negative amplitude and power values invert phase. When a limiter
/// gain is configured, peaks above SoX-ng's limiter threshold are folded toward
/// full scale instead of being multiplied directly.
///
/// The operation is deterministic and streaming-safe because every sample is
/// transformed independently. The no-limiter multiply path can use Auralis'
/// selected SIMD gain backend before applying the scalar clip step; limiter
/// processing is scalar because it is branchy and threshold-dependent.
///
/// # Examples
///
/// ```
/// use auralis_effects::Vol;
///
/// let mut samples = [0.25, -0.5, 1.0];
/// Vol::amplitude(0.5)?.process_samples(&mut samples);
///
/// assert_eq!(samples, [0.125, -0.25, 0.5]);
/// # Ok::<(), auralis_effects::EffectError>(())
/// ```
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Vol {
    /// User-facing gain value in [`Self::gain_type`] units.
    pub gain: f32,

    /// Unit used to interpret [`Self::gain`].
    pub gain_type: VolGainType,

    /// Optional SoX-ng limiter gain.
    pub limiter_gain: Option<f32>,

    multiplier: f32,
}

impl Vol {
    /// Creates a `vol` processor from a linear amplitude multiplier.
    ///
    /// # Errors
    ///
    /// Returns [`EffectError::InvalidVolGain`] when `gain` is not finite.
    pub fn amplitude(gain: f32) -> Result<Self> {
        Self::from_gain(VolGainType::Amplitude, gain)
    }

    /// Creates a `vol` processor from a signed power multiplier.
    ///
    /// # Errors
    ///
    /// Returns [`EffectError::InvalidVolGain`] when `gain` is not finite.
    pub fn power(gain: f32) -> Result<Self> {
        Self::from_gain(VolGainType::Power, gain)
    }

    /// Creates a `vol` processor from a decibel gain.
    #[must_use]
    #[allow(
        clippy::cast_possible_truncation,
        reason = "the public effect sample format is f32, so the parsed dB value is intentionally retained in f32 command units"
    )]
    pub fn decibels(gain: Decibels) -> Self {
        Self {
            gain: gain.as_f64() as f32,
            gain_type: VolGainType::Decibels,
            limiter_gain: None,
            multiplier: linear_gain(gain),
        }
    }

    /// Creates a processor from an explicit SoX-ng gain type and value.
    ///
    /// # Errors
    ///
    /// Returns [`EffectError::InvalidVolGain`] when `gain` is not finite.
    pub fn from_gain(gain_type: VolGainType, gain: f32) -> Result<Self> {
        if !gain.is_finite() {
            return Err(EffectError::InvalidVolGain);
        }

        let multiplier = match gain_type {
            VolGainType::Amplitude => gain,
            VolGainType::Power if gain.is_sign_negative() => -(-gain).sqrt(),
            VolGainType::Power => gain.sqrt(),
            VolGainType::Decibels => linear_gain(
                Decibels::new(f64::from(gain)).map_err(|_| EffectError::InvalidVolGain)?,
            ),
        };

        Ok(Self {
            gain,
            gain_type,
            limiter_gain: None,
            multiplier,
        })
    }

    /// Returns this processor with SoX-ng limiter-gain processing enabled.
    ///
    /// # Errors
    ///
    /// Returns [`EffectError::InvalidVolLimiterGain`] when `limiter_gain` is
    /// not finite, is outside `(0, 1)`, or the configured absolute amplitude
    /// multiplier is less than `1`.
    pub fn with_limiter_gain(mut self, limiter_gain: f32) -> Result<Self> {
        if !limiter_gain.is_finite()
            || limiter_gain <= 0.0
            || limiter_gain >= 1.0
            || self.multiplier.abs() < 1.0
        {
            return Err(EffectError::InvalidVolLimiterGain);
        }

        self.limiter_gain = Some(limiter_gain);
        Ok(self)
    }

    /// Returns the linear amplitude multiplier applied by this processor.
    #[must_use]
    pub const fn multiplier(self) -> f32 {
        self.multiplier
    }

    /// Applies the volume adjustment to all samples in an audio buffer.
    pub fn process_buffer(self, audio: &mut AudioBuffer) {
        self.process_samples(audio.as_planar_f32_mut());
    }

    /// Applies the volume adjustment using the requested backend.
    pub fn process_buffer_with_backend(
        self,
        audio: &mut AudioBuffer,
        requested_backend: BackendKind,
    ) {
        self.process_samples_with_backend(audio.as_planar_f32_mut(), requested_backend);
    }

    /// Applies the volume adjustment to a planar sample slice.
    pub fn process_samples(self, samples: &mut [f32]) {
        self.process_samples_with_backend(samples, BackendKind::Scalar);
    }

    /// Applies the volume adjustment to a planar sample slice using the requested backend.
    pub fn process_samples_with_backend(self, samples: &mut [f32], requested_backend: BackendKind) {
        if let Some(limiter_gain) = self.limiter_gain {
            apply_limiter(samples, self.multiplier, limiter_gain);
            return;
        }

        if self.multiplier.to_bits() == 1.0_f32.to_bits() {
            return;
        }

        gain_f32_in_place_with_backend(select_backend(requested_backend), samples, self.multiplier);
        clip_samples(samples);
    }
}

fn apply_limiter(samples: &mut [f32], multiplier: f32, limiter_gain: f32) {
    let threshold = (1.0 - limiter_gain) / (multiplier.abs() - limiter_gain);

    for sample in samples {
        *sample = if *sample > threshold {
            1.0 - limiter_gain * (1.0 - *sample)
        } else if *sample < -threshold {
            -(1.0 - limiter_gain * (1.0 + *sample))
        } else {
            multiplier * *sample
        }
        .clamp(-1.0, 1.0);
    }
}

fn clip_samples(samples: &mut [f32]) {
    for sample in samples {
        *sample = sample.clamp(-1.0, 1.0);
    }
}

#[cfg(test)]
mod tests {
    use super::{Vol, VolGainType};
    use auralis_core::Decibels;

    #[test]
    fn amplitude_gain_scales_and_clips_samples() {
        let mut samples = [0.25, -0.75, 0.75];

        Vol::amplitude(2.0).unwrap().process_samples(&mut samples);

        assert_samples_close(&samples, &[0.5, -1.0, 1.0]);
    }

    #[test]
    fn negative_power_gain_inverts_phase() {
        let mut samples = [0.25, -0.5];

        Vol::power(-0.25).unwrap().process_samples(&mut samples);

        assert_samples_close(&samples, &[-0.125, 0.25]);
    }

    #[test]
    fn decibel_gain_uses_shared_conversion() {
        let vol = Vol::decibels(Decibels::new(6.0).unwrap());

        assert_eq!(vol.gain_type, VolGainType::Decibels);
        assert!((vol.multiplier() - 1.995_262_3).abs() <= 0.000_001);
    }

    #[test]
    fn limiter_matches_sox_ng_threshold_formula() {
        let mut samples = [0.25, 0.5, 1.0, -1.0];

        Vol::amplitude(2.0)
            .unwrap()
            .with_limiter_gain(0.05)
            .unwrap()
            .process_samples(&mut samples);

        assert_samples_close(&[samples[0]], &[0.5]);
        assert!((samples[1] - 0.975).abs() <= 0.000_001);
        assert_samples_close(&samples[2..], &[1.0, -1.0]);
    }

    #[test]
    fn invalid_values_are_rejected() {
        assert!(Vol::amplitude(f32::NAN).is_err());
        assert!(
            Vol::amplitude(0.5)
                .unwrap()
                .with_limiter_gain(0.05)
                .is_err()
        );
        assert!(Vol::amplitude(2.0).unwrap().with_limiter_gain(1.0).is_err());
    }

    fn assert_samples_close(actual: &[f32], expected: &[f32]) {
        assert_eq!(actual.len(), expected.len());
        for (index, (&actual, &expected)) in actual.iter().zip(expected).enumerate() {
            assert!(
                (actual - expected).abs() <= 0.000_001,
                "sample {index}: actual={actual}, expected={expected}"
            );
        }
    }
}

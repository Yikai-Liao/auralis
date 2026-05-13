use auralis_core::AudioBuffer;

use crate::{EffectError, Result};

const DEFAULT_AMOUNT: f64 = 75.0;
const AMOUNT_SCALE: f64 = 750.0;

/// SoX-ng-style phase contrast enhancement.
///
/// `Contrast` maps each normalized sample through SoX-ng's phase contrast
/// curve. The user-facing amount is in the documented `0..=100` range, with
/// `75` as the default. Amount `0` still applies SoX-ng's base sine contrast
/// curve rather than preserving identity.
///
/// The operation is deterministic and streaming-safe because every sample is
/// transformed independently. It currently uses scalar `sin` evaluation; SIMD
/// is intentionally not used for this branch-heavy transcendental transform.
///
/// # Examples
///
/// ```
/// use auralis_effects::Contrast;
///
/// let mut samples = [0.0, 0.5, -0.5];
/// Contrast::new(0.0)?.process_samples(&mut samples);
///
/// assert_eq!(samples[0], 0.0);
/// assert!(samples[1] > 0.70 && samples[1] < 0.71);
/// assert!(samples[2] < -0.70 && samples[2] > -0.71);
/// # Ok::<(), auralis_effects::EffectError>(())
/// ```
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Contrast {
    /// User-facing contrast amount in the SoX-ng `0..=100` range.
    pub amount: f64,

    scaled_amount: f64,
}

impl Contrast {
    /// Creates a contrast processor for `amount`.
    ///
    /// # Errors
    ///
    /// Returns [`EffectError::InvalidContrastAmount`] when `amount` is not
    /// finite or is outside `0..=100`.
    pub fn new(amount: f64) -> Result<Self> {
        if !amount.is_finite() || !(0.0..=100.0).contains(&amount) {
            return Err(EffectError::InvalidContrastAmount);
        }

        Ok(Self {
            amount,
            scaled_amount: amount / AMOUNT_SCALE,
        })
    }

    /// Creates the SoX-ng default `contrast` processor.
    #[must_use]
    pub const fn default_amount() -> Self {
        Self {
            amount: DEFAULT_AMOUNT,
            scaled_amount: DEFAULT_AMOUNT / AMOUNT_SCALE,
        }
    }

    /// Applies contrast enhancement to all samples in an audio buffer.
    pub fn process_buffer(self, audio: &mut AudioBuffer) {
        self.process_samples(audio.as_planar_f32_mut());
    }

    /// Applies contrast enhancement to a planar sample slice.
    pub fn process_samples(self, samples: &mut [f32]) {
        #[allow(
            clippy::cast_possible_truncation,
            reason = "contrast processing works in the public f32 sample format and rounds the validated amount once per call"
        )]
        let scaled_amount = self.scaled_amount as f32;
        for sample in samples {
            *sample = contrast_sample(*sample, scaled_amount);
        }
    }
}

fn contrast_sample(sample: f32, amount: f32) -> f32 {
    let phase = sample * std::f32::consts::FRAC_PI_2;
    (phase + amount * (phase * 4.0).sin()).sin()
}

#[cfg(test)]
mod tests {
    use super::{Contrast, contrast_sample};
    use crate::EffectError;

    #[test]
    fn amount_zero_applies_sox_ng_base_sine_curve() {
        let mut samples = [-1.0, -0.5, 0.0, 0.5, 1.0];

        Contrast::new(0.0).unwrap().process_samples(&mut samples);

        assert_samples_close(
            &samples,
            &[
                -1.0,
                -std::f32::consts::FRAC_1_SQRT_2,
                0.0,
                std::f32::consts::FRAC_1_SQRT_2,
                1.0,
            ],
        );
    }

    #[test]
    fn default_amount_matches_sox_ng_formula() {
        let mut samples = [-0.25, 0.0, 0.25];

        Contrast::default_amount().process_samples(&mut samples);

        let expected_positive = contrast_sample(0.25, 0.1);
        assert_samples_close(&samples, &[-expected_positive, 0.0, expected_positive]);
    }

    #[test]
    fn invalid_amounts_are_rejected() {
        assert_eq!(
            Contrast::new(-0.001).unwrap_err(),
            EffectError::InvalidContrastAmount
        );
        assert_eq!(
            Contrast::new(100.001).unwrap_err(),
            EffectError::InvalidContrastAmount
        );
        assert_eq!(
            Contrast::new(f64::NAN).unwrap_err(),
            EffectError::InvalidContrastAmount
        );
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

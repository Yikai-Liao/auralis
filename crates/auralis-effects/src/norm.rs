use auralis_core::{AudioBuffer, Decibels};
use auralis_dsp::linear_gain;
use auralis_simd::{BackendKind, gain_f32_in_place_with_backend, select_backend};

use crate::{EffectError, Result};

/// SoX-ng-style `norm` effect processor.
///
/// `Norm` scans the whole decoded buffer, scales non-silent audio so the peak
/// reaches the configured target level, and leaves silence unchanged. This is
/// effect-chain behavior equivalent to SoX-ng's `norm [level]` command, which
/// is an alias for `gain -n [level]`.
///
/// This type is intentionally separate from Auralis' output `--norm` policy:
/// effect-level `norm` runs at its position in an [`crate::EffectChain`], while
/// output normalization runs only as the final write policy.
///
/// # Examples
///
/// ```
/// use auralis_core::Decibels;
/// use auralis_effects::Norm;
///
/// let mut samples = [0.25, -0.5, 0.0];
/// Norm::new(Decibels::new(-6.0)?).process_samples(&mut samples)?;
///
/// assert!(samples[1] < -0.50 && samples[1] > -0.51);
/// # Ok::<(), Box<dyn std::error::Error>>(())
/// ```
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Norm {
    /// Target peak level in dBFS.
    pub target: Decibels,
}

impl Norm {
    /// Creates a normalization processor for `target` dBFS.
    #[must_use]
    pub const fn new(target: Decibels) -> Self {
        Self { target }
    }

    /// Creates the SoX-ng default `norm` processor, targeting 0 dBFS.
    ///
    /// # Errors
    ///
    /// Returns the same validation error as [`Decibels::new`] if 0 dB is ever
    /// rejected by the core decibel type.
    pub fn zero_db() -> std::result::Result<Self, auralis_core::AuralisError> {
        Ok(Self::new(Decibels::new(0.0)?))
    }

    /// Applies normalization to all samples in an audio buffer.
    ///
    /// # Errors
    ///
    /// Returns [`EffectError::NonFiniteNormSample`] if any sample is NaN or
    /// infinite.
    pub fn process_buffer(self, audio: &mut AudioBuffer) -> Result<()> {
        self.process_samples(audio.as_planar_f32_mut())
    }

    /// Applies normalization with a requested backend for the multiply pass.
    ///
    /// The peak scan is scalar and deterministic. The final multiply can use
    /// Auralis' SIMD backend when available; unsupported SIMD requests follow
    /// the standard scalar fallback.
    ///
    /// # Errors
    ///
    /// Returns the same errors as [`Self::process_buffer`].
    pub fn process_buffer_with_backend(
        self,
        audio: &mut AudioBuffer,
        requested_backend: BackendKind,
    ) -> Result<()> {
        self.process_samples_with_backend(audio.as_planar_f32_mut(), requested_backend)
    }

    /// Applies normalization to a planar sample slice.
    ///
    /// This operation is whole-buffer by design and is not chunk-invariant:
    /// callers should pass the complete signal segment that should be scanned.
    ///
    /// # Errors
    ///
    /// Returns the same errors as [`Self::process_buffer`].
    pub fn process_samples(self, samples: &mut [f32]) -> Result<()> {
        self.process_samples_with_backend(samples, BackendKind::Scalar)
    }

    /// Applies normalization to a planar sample slice using a requested backend.
    ///
    /// # Errors
    ///
    /// Returns the same errors as [`Self::process_buffer`].
    pub fn process_samples_with_backend(
        self,
        samples: &mut [f32],
        requested_backend: BackendKind,
    ) -> Result<()> {
        let peak = finite_peak_amplitude(samples)?;
        if peak == 0.0 {
            return Ok(());
        }

        let multiplier = linear_gain(self.target) / peak;
        gain_f32_in_place_with_backend(select_backend(requested_backend), samples, multiplier);
        Ok(())
    }
}

fn finite_peak_amplitude(samples: &[f32]) -> Result<f32> {
    let mut peak = 0.0_f32;
    for (sample_index, &sample) in samples.iter().enumerate() {
        if !sample.is_finite() {
            return Err(EffectError::NonFiniteNormSample { sample_index });
        }
        peak = peak.max(sample.abs());
    }

    Ok(peak)
}

#[cfg(test)]
mod tests {
    use super::Norm;
    use auralis_core::Decibels;

    #[test]
    fn normalizes_peak_to_target_level() {
        let mut samples = [0.25, -0.5, 0.125];
        let target = auralis_dsp::linear_gain(Decibels::new(-6.0).unwrap());

        Norm::new(Decibels::new(-6.0).unwrap())
            .process_samples(&mut samples)
            .unwrap();

        assert_samples_close(&samples, &[0.5 * target, -target, 0.25 * target]);
    }

    #[test]
    fn leaves_silence_silent() {
        let mut samples = [0.0, -0.0, 0.0];

        Norm::zero_db()
            .unwrap()
            .process_samples(&mut samples)
            .unwrap();

        assert_eq!(
            samples.map(f32::to_bits),
            [0.0_f32, -0.0, 0.0].map(f32::to_bits)
        );
    }

    #[test]
    fn reports_non_finite_samples() {
        let mut samples = [0.25, f32::NAN];

        let error = Norm::zero_db()
            .unwrap()
            .process_samples(&mut samples)
            .unwrap_err();

        assert_eq!(
            error,
            crate::EffectError::NonFiniteNormSample { sample_index: 1 }
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

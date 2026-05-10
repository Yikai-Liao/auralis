use auralis_core::{AudioBuffer, SampleRate};

use crate::{Biquad, BiquadCoefficients, EffectError, Result};

/// SoX-ng-style RIAA vinyl playback equalization filter.
///
/// `riaa` applies SoX-ng's fixed second-order IIR coefficient families for
/// common archival sample rates, then normalizes the response to 0 dB at 1 kHz.
/// SoX-ng accepts only 44.1 kHz, 48 kHz, 88.2 kHz, 96 kHz, and 192 kHz input
/// for this effect.
///
/// Processing is deterministic and streaming-safe when callers preserve a
/// [`crate::BiquadState`] per channel across chunks.
///
/// # Examples
///
/// ```
/// use auralis_effects::Riaa;
///
/// let mut audio = auralis_core::AudioBuffer::from_planar_f32(
///     auralis_core::AudioSpec::new(
///         auralis_core::SampleRate::new(48_000)?,
///         auralis_core::ChannelCount::new(1)?,
///         auralis_core::SampleFormat::Float32,
///     ),
///     auralis_core::FrameCount::new(3),
///     vec![0.25, 0.0, -0.25],
/// )?;
/// Riaa::new().process_buffer(&mut audio)?;
///
/// assert!(audio.as_planar_f32().iter().all(|sample| sample.is_finite()));
/// # Ok::<(), Box<dyn std::error::Error>>(())
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Riaa;

impl Riaa {
    /// Creates an RIAA equalization filter.
    #[must_use]
    pub const fn new() -> Self {
        Self
    }

    /// Returns normalized coefficients for SoX-ng's sample-rate-specific preset.
    ///
    /// # Errors
    ///
    /// Returns [`EffectError::InvalidBiquadDesign`] when the sample rate is not
    /// one of SoX-ng's supported RIAA rates: 44.1 kHz, 48 kHz, 88.2 kHz, 96 kHz,
    /// or 192 kHz.
    pub fn coefficients(self, sample_rate: SampleRate) -> Result<BiquadCoefficients> {
        let (zeros, poles) = match sample_rate.as_u32() {
            44_100 => ([-0.201_489_8, 0.923_382_0], [0.708_314_9, 0.992_409_1]),
            48_000 => ([-0.176_606_9, 0.932_159_0], [0.739_632_5, 0.993_133_0]),
            88_200 => ([-0.116_873_5, 0.964_831_2], [0.859_064_6, 0.996_400_2]),
            96_000 => ([-0.114_148_6, 0.967_681_7], [0.869_913_7, 0.996_694_6]),
            192_000 => (
                [-0.104_061_096_5, 0.983_752_326_3],
                [0.932_899_297_1, 0.998_363_312_5],
            ),
            _ => return Err(EffectError::InvalidBiquadDesign),
        };
        let mut numerator = polynomial_from_roots(zeros);
        let denominator = polynomial_from_roots(poles);

        normalize_at_1khz(&mut numerator, denominator, f64::from(sample_rate.as_u32()));

        BiquadCoefficients::normalized(
            numerator[0],
            numerator[1],
            numerator[2],
            denominator[1],
            denominator[2],
        )
        .map_err(|_| EffectError::InvalidBiquadDesign)
    }

    /// Applies the RIAA filter independently to every channel.
    ///
    /// # Errors
    ///
    /// Returns [`EffectError::InvalidBiquadDesign`] when the buffer sample rate
    /// is not one of SoX-ng's supported RIAA rates.
    pub fn process_buffer(self, audio: &mut AudioBuffer) -> Result<()> {
        let coefficients = self.coefficients(audio.spec().sample_rate())?;
        Biquad::new(coefficients).process_buffer(audio);
        Ok(())
    }
}

fn polynomial_from_roots(roots: [f64; 2]) -> [f64; 3] {
    [1.0, -(roots[0] + roots[1]), roots[0] * roots[1]]
}

fn normalize_at_1khz(numerator: &mut [f64; 3], denominator: [f64; 3], sample_rate_hz: f64) {
    let y = 2.0 * std::f64::consts::PI * 1000.0 / sample_rate_hz;
    let b_re = numerator[0] + numerator[1] * (-y).cos() + numerator[2] * (-2.0 * y).cos();
    let a_re = denominator[0] + denominator[1] * (-y).cos() + denominator[2] * (-2.0 * y).cos();
    let b_im = numerator[1] * (-y).sin() + numerator[2] * (-2.0 * y).sin();
    let a_im = denominator[1] * (-y).sin() + denominator[2] * (-2.0 * y).sin();
    let gain = ((a_re.mul_add(a_re, a_im * a_im)) / (b_re.mul_add(b_re, b_im * b_im))).sqrt();

    for coefficient in numerator {
        *coefficient *= gain;
    }
}

#[cfg(test)]
mod tests {
    use super::Riaa;
    use crate::{BiquadCoefficients, EffectError};
    use auralis_core::SampleRate;

    #[test]
    fn uses_sox_ng_48k_roots_normalized_at_1khz() {
        assert_eq!(
            Riaa::new()
                .coefficients(SampleRate::new(48_000).unwrap())
                .unwrap(),
            BiquadCoefficients::normalized(
                0.219_737_823_085_859_87,
                -0.166_023_373_681_949_9,
                -0.036_174_495_424_386,
                -1.732_765_500_000_000_2,
                0.734_553_443_622_500_1,
            )
            .unwrap()
        );
    }

    #[test]
    fn supports_all_sox_ng_riaa_sample_rates() {
        for sample_rate in [44_100, 48_000, 88_200, 96_000, 192_000] {
            let coefficients = Riaa::new()
                .coefficients(SampleRate::new(sample_rate).unwrap())
                .unwrap();

            assert!(coefficients.b0.is_finite());
            assert!(coefficients.b1.is_finite());
            assert!(coefficients.b2.is_finite());
            assert!(coefficients.a1.is_finite());
            assert!(coefficients.a2.is_finite());
        }
    }

    #[test]
    fn rejects_sample_rates_without_sox_ng_presets() {
        assert_eq!(
            Riaa::new()
                .coefficients(SampleRate::new(32_000).unwrap())
                .unwrap_err(),
            EffectError::InvalidBiquadDesign
        );
    }
}

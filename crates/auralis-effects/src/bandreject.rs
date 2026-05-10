use auralis_core::{AudioBuffer, SampleRate};

use crate::{Biquad, BiquadCoefficients, BiquadWidth, EffectError, Result};

/// SoX-ng-style RBJ band-reject filter.
///
/// `bandreject frequency width` applies the RBJ notch filter shape used by
/// SoX-ng's `bandreject` effect. Width accepts hertz, kilohertz, quality-factor,
/// or octave units.
///
/// Processing is deterministic and streaming-safe when callers preserve a
/// [`crate::BiquadState`] per channel across chunks.
///
/// # Examples
///
/// ```
/// use auralis_effects::{BandReject, BiquadWidth};
///
/// let mut audio = auralis_core::AudioBuffer::from_planar_f32(
///     auralis_core::AudioSpec::new(
///         auralis_core::SampleRate::new(48_000)?,
///         auralis_core::ChannelCount::new(1)?,
///         auralis_core::SampleFormat::Float32,
///     ),
///     auralis_core::FrameCount::new(3),
///     vec![1.0, 0.0, -1.0],
/// )?;
/// BandReject::new(1_000.0, BiquadWidth::q(2.0))?.process_buffer(&mut audio)?;
///
/// assert!(audio.as_planar_f32().iter().all(|sample| sample.is_finite()));
/// # Ok::<(), Box<dyn std::error::Error>>(())
/// ```
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct BandReject {
    /// Center frequency in hertz.
    pub frequency_hz: f64,

    /// Bandwidth setting.
    pub width: BiquadWidth,
}

impl BandReject {
    /// Creates a SoX-ng-style RBJ band-reject filter.
    ///
    /// # Errors
    ///
    /// Returns [`EffectError::InvalidBiquadDesign`] when `frequency_hz` or
    /// `width` is outside the supported design range.
    pub fn new(frequency_hz: f64, width: BiquadWidth) -> Result<Self> {
        let filter = Self {
            frequency_hz,
            width,
        };
        filter.validate_static()?;
        Ok(filter)
    }

    /// Returns normalized RBJ coefficients for a concrete sample rate.
    ///
    /// # Errors
    ///
    /// Returns [`EffectError::InvalidBiquadDesign`] when the configured
    /// frequency is at or above Nyquist or width conversion produces an invalid
    /// coefficient set.
    pub fn coefficients(self, sample_rate: SampleRate) -> Result<BiquadCoefficients> {
        Ok(BiquadCoefficients::rbj_band_reject(
            f64::from(sample_rate.as_u32()),
            self.frequency_hz,
            self.width,
        )?)
    }

    /// Applies the band-reject filter independently to every channel.
    ///
    /// # Errors
    ///
    /// Returns [`EffectError::InvalidBiquadDesign`] if this buffer's sample
    /// rate makes the configured filter invalid.
    pub fn process_buffer(self, audio: &mut AudioBuffer) -> Result<()> {
        let coefficients = self.coefficients(audio.spec().sample_rate())?;
        Biquad::new(coefficients).process_buffer(audio);
        Ok(())
    }

    fn validate_static(self) -> Result<()> {
        if !self.frequency_hz.is_finite() || self.frequency_hz <= 0.0 {
            return Err(EffectError::InvalidBiquadDesign);
        }

        reject_invalid_width(self.width)
    }
}

fn reject_invalid_width(width: BiquadWidth) -> Result<()> {
    let valid = match width {
        BiquadWidth::Q(value)
        | BiquadWidth::Octaves(value)
        | BiquadWidth::Hertz(value)
        | BiquadWidth::Kilohertz(value) => value.is_finite() && value > 0.0,
        BiquadWidth::Slope(_) => false,
    };

    if valid {
        Ok(())
    } else {
        Err(EffectError::InvalidBiquadDesign)
    }
}

#[cfg(test)]
mod tests {
    use super::BandReject;
    use crate::{BiquadCoefficients, BiquadWidth, EffectError};
    use auralis_core::SampleRate;

    #[test]
    fn uses_rbj_band_reject_coefficients() {
        let band_reject = BandReject::new(1_000.0, BiquadWidth::q(2.0)).unwrap();

        assert_eq!(
            band_reject
                .coefficients(SampleRate::new(48_000).unwrap())
                .unwrap(),
            BiquadCoefficients::rbj_band_reject(48_000.0, 1_000.0, BiquadWidth::q(2.0)).unwrap()
        );
    }

    #[test]
    fn rejects_invalid_design_values() {
        assert_eq!(
            BandReject::new(0.0, BiquadWidth::q(1.0)).unwrap_err(),
            EffectError::InvalidBiquadDesign
        );
        assert_eq!(
            BandReject::new(1_000.0, BiquadWidth::slope(0.5)).unwrap_err(),
            EffectError::InvalidBiquadDesign
        );
        assert_eq!(
            BandReject::new(24_000.0, BiquadWidth::q(1.0))
                .unwrap()
                .coefficients(SampleRate::new(48_000).unwrap())
                .unwrap_err(),
            EffectError::InvalidBiquadDesign
        );
    }
}

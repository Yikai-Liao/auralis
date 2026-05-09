use auralis_core::{AudioBuffer, SampleRate};

use crate::{Biquad, BiquadCoefficients, BiquadWidth, EffectError, Result};

/// SoX-ng-style RBJ peaking equalizer filter.
///
/// `equalizer frequency width gain` applies a peaking EQ centered at
/// `frequency`. Width accepts hertz, kilohertz, quality-factor, or octave
/// units, and gain is measured in decibels.
///
/// Processing is deterministic and streaming-safe when callers preserve a
/// [`crate::BiquadState`] per channel across chunks.
///
/// # Examples
///
/// ```
/// use auralis_effects::{BiquadWidth, Equalizer};
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
/// Equalizer::new(1_000.0, BiquadWidth::q(1.0), 6.0)?.process_buffer(&mut audio)?;
///
/// assert!(audio.as_planar_f32().iter().all(|sample| sample.is_finite()));
/// # Ok::<(), Box<dyn std::error::Error>>(())
/// ```
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Equalizer {
    /// Center frequency in hertz.
    pub frequency_hz: f64,

    /// Bandwidth setting.
    pub width: BiquadWidth,

    /// Peak gain in decibels.
    pub gain_db: f64,
}

impl Equalizer {
    /// Creates a SoX-ng-style RBJ peaking equalizer.
    ///
    /// # Errors
    ///
    /// Returns [`EffectError::InvalidBiquadDesign`] when `frequency_hz`,
    /// `width`, or `gain_db` is outside the supported design range.
    pub fn new(frequency_hz: f64, width: BiquadWidth, gain_db: f64) -> Result<Self> {
        let equalizer = Self {
            frequency_hz,
            width,
            gain_db,
        };
        equalizer.validate_static()?;
        Ok(equalizer)
    }

    /// Returns normalized RBJ peaking-EQ coefficients for a concrete sample rate.
    ///
    /// # Errors
    ///
    /// Returns [`EffectError::InvalidBiquadDesign`] when the configured
    /// frequency is at or above Nyquist or width conversion produces an invalid
    /// coefficient set.
    pub fn coefficients(self, sample_rate: SampleRate) -> Result<BiquadCoefficients> {
        BiquadCoefficients::rbj_peaking_eq(
            f64::from(sample_rate.as_u32()),
            self.frequency_hz,
            self.width,
            self.gain_db,
        )
    }

    /// Applies the peaking equalizer independently to every channel.
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
        if !self.frequency_hz.is_finite() || self.frequency_hz <= 0.0 || !self.gain_db.is_finite() {
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
    use super::Equalizer;
    use crate::{BiquadCoefficients, BiquadWidth, EffectError};
    use auralis_core::SampleRate;

    #[test]
    fn uses_rbj_peaking_eq_coefficients() {
        let equalizer = Equalizer::new(1_000.0, BiquadWidth::q(1.0), 6.0).unwrap();

        assert_eq!(
            equalizer
                .coefficients(SampleRate::new(48_000).unwrap())
                .unwrap(),
            BiquadCoefficients::rbj_peaking_eq(48_000.0, 1_000.0, BiquadWidth::q(1.0), 6.0)
                .unwrap()
        );
    }

    #[test]
    fn rejects_invalid_design_values() {
        assert_eq!(
            Equalizer::new(0.0, BiquadWidth::q(1.0), 6.0).unwrap_err(),
            EffectError::InvalidBiquadDesign
        );
        assert_eq!(
            Equalizer::new(1_000.0, BiquadWidth::slope(0.5), 6.0).unwrap_err(),
            EffectError::InvalidBiquadDesign
        );
        assert_eq!(
            Equalizer::new(1_000.0, BiquadWidth::q(1.0), f64::NAN).unwrap_err(),
            EffectError::InvalidBiquadDesign
        );
        assert_eq!(
            Equalizer::new(24_000.0, BiquadWidth::q(1.0), 6.0)
                .unwrap()
                .coefficients(SampleRate::new(48_000).unwrap())
                .unwrap_err(),
            EffectError::InvalidBiquadDesign
        );
    }
}

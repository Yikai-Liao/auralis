use auralis_core::AudioBuffer;

use crate::{Biquad, BiquadCoefficients, BiquadWidth, EffectError, Result};

/// SoX-ng-style bass tone control backed by an RBJ low-shelf biquad.
///
/// `bass gain [frequency [width]]` applies low-frequency boost or cut. The
/// command defaults are 100 Hz and shelf slope `0.5s`; typed constructors expose
/// the same defaults through [`Bass::new`].
///
/// Processing is deterministic and streaming-safe when callers preserve a
/// [`crate::BiquadState`] per channel across chunks.
///
/// # Examples
///
/// ```
/// use auralis_effects::{Bass, BiquadWidth};
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
/// Bass::with_width(6.0, 100.0, BiquadWidth::slope(0.5))?.process_buffer(&mut audio)?;
///
/// assert!(audio.as_planar_f32().iter().all(|sample| sample.is_finite()));
/// # Ok::<(), Box<dyn std::error::Error>>(())
/// ```
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Bass {
    /// Gain at 0 Hz in decibels.
    pub gain_db: f64,

    /// Shelf midpoint frequency in hertz.
    pub frequency_hz: f64,

    /// Shelf slope or bandwidth setting.
    pub width: BiquadWidth,
}

impl Bass {
    /// SoX-ng's default bass shelf frequency in hertz.
    pub const DEFAULT_FREQUENCY_HZ: f64 = 100.0;

    /// SoX-ng's default bass shelf slope.
    pub const DEFAULT_WIDTH: BiquadWidth = BiquadWidth::slope(0.5);

    /// Creates a bass tone control with SoX-ng's default frequency and width.
    ///
    /// # Errors
    ///
    /// Returns [`EffectError::InvalidBiquadDesign`] when `gain_db` is not
    /// finite.
    pub fn new(gain_db: f64) -> Result<Self> {
        Self::with_width(gain_db, Self::DEFAULT_FREQUENCY_HZ, Self::DEFAULT_WIDTH)
    }

    /// Creates a bass tone control with a custom frequency and default width.
    ///
    /// # Errors
    ///
    /// Returns [`EffectError::InvalidBiquadDesign`] when the gain or frequency
    /// is outside the supported design range.
    pub fn with_frequency(gain_db: f64, frequency_hz: f64) -> Result<Self> {
        Self::with_width(gain_db, frequency_hz, Self::DEFAULT_WIDTH)
    }

    /// Creates a bass tone control with custom frequency and width.
    ///
    /// # Errors
    ///
    /// Returns [`EffectError::InvalidBiquadDesign`] when a design parameter is
    /// outside the supported range.
    pub fn with_width(gain_db: f64, frequency_hz: f64, width: BiquadWidth) -> Result<Self> {
        let bass = Self {
            gain_db,
            frequency_hz,
            width,
        };
        bass.validate_static()?;
        Ok(bass)
    }

    /// Returns normalized RBJ low-shelf coefficients for a concrete sample rate.
    ///
    /// # Errors
    ///
    /// Returns [`EffectError::InvalidBiquadDesign`] when this configuration is
    /// invalid for the sample rate, such as a frequency at or above Nyquist.
    pub fn coefficients(self, sample_rate: auralis_core::SampleRate) -> Result<BiquadCoefficients> {
        Ok(BiquadCoefficients::rbj_low_shelf(
            f64::from(sample_rate.as_u32()),
            self.frequency_hz,
            self.width,
            self.gain_db,
        )?)
    }

    /// Applies the bass tone control independently to every channel.
    ///
    /// # Errors
    ///
    /// Returns [`EffectError::InvalidBiquadDesign`] if this buffer's sample
    /// rate makes the configured shelf invalid.
    pub fn process_buffer(self, audio: &mut AudioBuffer) -> Result<()> {
        let coefficients = self.coefficients(audio.spec().sample_rate())?;
        Biquad::new(coefficients).process_buffer(audio);
        Ok(())
    }

    fn validate_static(self) -> Result<()> {
        if !self.gain_db.is_finite() || !self.frequency_hz.is_finite() || self.frequency_hz <= 0.0 {
            return Err(EffectError::InvalidBiquadDesign);
        }

        reject_invalid_shelf_width(self.width)
    }
}

pub(crate) fn reject_invalid_shelf_width(width: BiquadWidth) -> Result<()> {
    let valid = match width {
        BiquadWidth::Q(value)
        | BiquadWidth::Octaves(value)
        | BiquadWidth::Hertz(value)
        | BiquadWidth::Kilohertz(value) => value.is_finite() && value > 0.0,
        BiquadWidth::Slope(value) => value.is_finite() && value > 0.0 && value <= 1.0,
    };

    if valid {
        Ok(())
    } else {
        Err(EffectError::InvalidBiquadDesign)
    }
}

#[cfg(test)]
mod tests {
    use super::Bass;
    use crate::{BiquadCoefficients, BiquadWidth, EffectError};
    use auralis_core::SampleRate;

    #[test]
    fn uses_rbj_low_shelf_coefficients() {
        let bass = Bass::with_width(6.0, 100.0, BiquadWidth::slope(0.5)).unwrap();

        assert_eq!(
            bass.coefficients(SampleRate::new(48_000).unwrap()).unwrap(),
            BiquadCoefficients::rbj_low_shelf(48_000.0, 100.0, BiquadWidth::slope(0.5), 6.0)
                .unwrap()
        );
    }

    #[test]
    fn constructors_apply_sox_ng_defaults() {
        assert_eq!(
            Bass::new(3.0).unwrap(),
            Bass::with_width(3.0, 100.0, BiquadWidth::slope(0.5)).unwrap()
        );
        assert_eq!(
            Bass::with_frequency(-3.0, 150.0).unwrap(),
            Bass::with_width(-3.0, 150.0, BiquadWidth::slope(0.5)).unwrap()
        );
    }

    #[test]
    fn rejects_invalid_design_values() {
        assert_eq!(
            Bass::new(f64::NAN).unwrap_err(),
            EffectError::InvalidBiquadDesign
        );
        assert_eq!(
            Bass::with_frequency(6.0, 0.0).unwrap_err(),
            EffectError::InvalidBiquadDesign
        );
        assert_eq!(
            Bass::with_width(6.0, 100.0, BiquadWidth::slope(1.5)).unwrap_err(),
            EffectError::InvalidBiquadDesign
        );
        assert_eq!(
            Bass::with_width(6.0, 24_000.0, BiquadWidth::q(1.0))
                .unwrap()
                .coefficients(SampleRate::new(48_000).unwrap())
                .unwrap_err(),
            EffectError::InvalidBiquadDesign
        );
    }
}

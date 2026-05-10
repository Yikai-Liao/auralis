use auralis_core::AudioBuffer;

use crate::{
    Biquad, BiquadCoefficients, BiquadWidth, EffectError, Result, bass::reject_invalid_shelf_width,
};

/// SoX-ng-style treble tone control backed by an RBJ high-shelf biquad.
///
/// `treble gain [frequency [width]]` applies high-frequency boost or cut. The
/// command defaults are 3000 Hz and shelf slope `0.5s`; typed constructors
/// expose the same defaults through [`Treble::new`].
///
/// Processing is deterministic and streaming-safe when callers preserve a
/// [`crate::BiquadState`] per channel across chunks.
///
/// # Examples
///
/// ```
/// use auralis_effects::{BiquadWidth, Treble};
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
/// Treble::with_width(-6.0, 3000.0, BiquadWidth::slope(0.5))?.process_buffer(&mut audio)?;
///
/// assert!(audio.as_planar_f32().iter().all(|sample| sample.is_finite()));
/// # Ok::<(), Box<dyn std::error::Error>>(())
/// ```
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Treble {
    /// Gain at the high-frequency shelf in decibels.
    pub gain_db: f64,

    /// Shelf midpoint frequency in hertz.
    pub frequency_hz: f64,

    /// Shelf slope or bandwidth setting.
    pub width: BiquadWidth,
}

impl Treble {
    /// SoX-ng's default treble shelf frequency in hertz.
    pub const DEFAULT_FREQUENCY_HZ: f64 = 3000.0;

    /// SoX-ng's default treble shelf slope.
    pub const DEFAULT_WIDTH: BiquadWidth = BiquadWidth::slope(0.5);

    /// Creates a treble tone control with SoX-ng's default frequency and width.
    ///
    /// # Errors
    ///
    /// Returns [`EffectError::InvalidBiquadDesign`] when `gain_db` is not
    /// finite.
    pub fn new(gain_db: f64) -> Result<Self> {
        Self::with_width(gain_db, Self::DEFAULT_FREQUENCY_HZ, Self::DEFAULT_WIDTH)
    }

    /// Creates a treble tone control with a custom frequency and default width.
    ///
    /// # Errors
    ///
    /// Returns [`EffectError::InvalidBiquadDesign`] when the gain or frequency
    /// is outside the supported design range.
    pub fn with_frequency(gain_db: f64, frequency_hz: f64) -> Result<Self> {
        Self::with_width(gain_db, frequency_hz, Self::DEFAULT_WIDTH)
    }

    /// Creates a treble tone control with custom frequency and width.
    ///
    /// # Errors
    ///
    /// Returns [`EffectError::InvalidBiquadDesign`] when a design parameter is
    /// outside the supported range.
    pub fn with_width(gain_db: f64, frequency_hz: f64, width: BiquadWidth) -> Result<Self> {
        let treble = Self {
            gain_db,
            frequency_hz,
            width,
        };
        treble.validate_static()?;
        Ok(treble)
    }

    /// Returns normalized RBJ high-shelf coefficients for a concrete sample rate.
    ///
    /// # Errors
    ///
    /// Returns [`EffectError::InvalidBiquadDesign`] when this configuration is
    /// invalid for the sample rate, such as a frequency at or above Nyquist.
    pub fn coefficients(self, sample_rate: auralis_core::SampleRate) -> Result<BiquadCoefficients> {
        Ok(BiquadCoefficients::rbj_high_shelf(
            f64::from(sample_rate.as_u32()),
            self.frequency_hz,
            self.width,
            self.gain_db,
        )?)
    }

    /// Applies the treble tone control independently to every channel.
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

#[cfg(test)]
mod tests {
    use super::Treble;
    use crate::{BiquadCoefficients, BiquadWidth, EffectError};
    use auralis_core::SampleRate;

    #[test]
    fn uses_rbj_high_shelf_coefficients() {
        let treble = Treble::with_width(-6.0, 3000.0, BiquadWidth::slope(0.5)).unwrap();

        assert_eq!(
            treble
                .coefficients(SampleRate::new(48_000).unwrap())
                .unwrap(),
            BiquadCoefficients::rbj_high_shelf(48_000.0, 3000.0, BiquadWidth::slope(0.5), -6.0)
                .unwrap()
        );
    }

    #[test]
    fn constructors_apply_sox_ng_defaults() {
        assert_eq!(
            Treble::new(3.0).unwrap(),
            Treble::with_width(3.0, 3000.0, BiquadWidth::slope(0.5)).unwrap()
        );
        assert_eq!(
            Treble::with_frequency(-3.0, 3500.0).unwrap(),
            Treble::with_width(-3.0, 3500.0, BiquadWidth::slope(0.5)).unwrap()
        );
    }

    #[test]
    fn rejects_invalid_design_values() {
        assert_eq!(
            Treble::new(f64::NAN).unwrap_err(),
            EffectError::InvalidBiquadDesign
        );
        assert_eq!(
            Treble::with_frequency(6.0, 0.0).unwrap_err(),
            EffectError::InvalidBiquadDesign
        );
        assert_eq!(
            Treble::with_width(6.0, 3000.0, BiquadWidth::slope(1.5)).unwrap_err(),
            EffectError::InvalidBiquadDesign
        );
        assert_eq!(
            Treble::with_width(6.0, 24_000.0, BiquadWidth::q(1.0))
                .unwrap()
                .coefficients(SampleRate::new(48_000).unwrap())
                .unwrap_err(),
            EffectError::InvalidBiquadDesign
        );
    }
}

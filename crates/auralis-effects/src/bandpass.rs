use auralis_core::{AudioBuffer, SampleRate};

use crate::{Biquad, BiquadCoefficients, BiquadWidth, EffectError, Result};

/// SoX-ng-style RBJ band-pass filter.
///
/// `bandpass frequency width` uses SoX-ng's constant 0 dB peak-gain shape.
/// The `-c` form selects constant-skirt gain, where the peak gain equals `Q`
/// when the width is expressed as a quality factor. Width accepts hertz,
/// kilohertz, quality-factor, or octave units.
///
/// Processing is deterministic and streaming-safe when callers preserve a
/// [`crate::BiquadState`] per channel across chunks.
///
/// # Examples
///
/// ```
/// use auralis_effects::{BandPass, BiquadWidth};
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
/// BandPass::new(1_000.0, BiquadWidth::q(2.0))?.process_buffer(&mut audio)?;
///
/// assert!(audio.as_planar_f32().iter().all(|sample| sample.is_finite()));
/// # Ok::<(), Box<dyn std::error::Error>>(())
/// ```
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct BandPass {
    /// Center frequency in hertz.
    pub frequency_hz: f64,

    /// Bandwidth setting.
    pub width: BiquadWidth,

    /// Band-pass gain scaling mode.
    pub mode: BandPassMode,
}

impl BandPass {
    /// Creates SoX-ng's default constant 0 dB peak-gain band-pass filter.
    ///
    /// # Errors
    ///
    /// Returns [`EffectError::InvalidBiquadDesign`] when `frequency_hz` or
    /// `width` is outside the supported design range.
    pub fn new(frequency_hz: f64, width: BiquadWidth) -> Result<Self> {
        let filter = Self {
            frequency_hz,
            width,
            mode: BandPassMode::ConstantPeak,
        };
        filter.validate_static()?;
        Ok(filter)
    }

    /// Creates SoX-ng's `bandpass -c frequency width` constant-skirt form.
    ///
    /// # Errors
    ///
    /// Returns [`EffectError::InvalidBiquadDesign`] when `frequency_hz` or
    /// `width` is outside the supported design range.
    pub fn constant_skirt(frequency_hz: f64, width: BiquadWidth) -> Result<Self> {
        let filter = Self {
            frequency_hz,
            width,
            mode: BandPassMode::ConstantSkirt,
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
        let sample_rate_hz = f64::from(sample_rate.as_u32());
        match self.mode {
            BandPassMode::ConstantPeak => Ok(BiquadCoefficients::rbj_band_pass_constant_peak(
                sample_rate_hz,
                self.frequency_hz,
                self.width,
            )?),
            BandPassMode::ConstantSkirt => Ok(BiquadCoefficients::rbj_band_pass_constant_skirt(
                sample_rate_hz,
                self.frequency_hz,
                self.width,
            )?),
        }
    }

    /// Applies the band-pass filter independently to every channel.
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

/// SoX-ng `bandpass` gain scaling mode.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BandPassMode {
    /// Default constant 0 dB peak-gain RBJ band-pass form.
    ConstantPeak,

    /// `-c` constant-skirt RBJ band-pass form.
    ConstantSkirt,
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
    use super::{BandPass, BandPassMode};
    use crate::{BiquadCoefficients, BiquadWidth, EffectError};
    use auralis_core::SampleRate;

    #[test]
    fn default_mode_uses_rbj_constant_peak_coefficients() {
        let band_pass = BandPass::new(1_000.0, BiquadWidth::q(2.0)).unwrap();

        assert_eq!(
            band_pass
                .coefficients(SampleRate::new(48_000).unwrap())
                .unwrap(),
            BiquadCoefficients::rbj_band_pass_constant_peak(
                48_000.0,
                1_000.0,
                BiquadWidth::q(2.0),
            )
            .unwrap()
        );
    }

    #[test]
    fn constant_skirt_mode_uses_rbj_constant_skirt_coefficients() {
        let band_pass = BandPass::constant_skirt(1_000.0, BiquadWidth::octaves(0.5)).unwrap();

        assert_eq!(
            band_pass
                .coefficients(SampleRate::new(48_000).unwrap())
                .unwrap(),
            BiquadCoefficients::rbj_band_pass_constant_skirt(
                48_000.0,
                1_000.0,
                BiquadWidth::octaves(0.5),
            )
            .unwrap()
        );
    }

    #[test]
    fn rejects_invalid_design_values() {
        assert_eq!(
            BandPass::new(0.0, BiquadWidth::q(1.0)).unwrap_err(),
            EffectError::InvalidBiquadDesign
        );
        assert_eq!(
            BandPass::new(1_000.0, BiquadWidth::slope(0.5)).unwrap_err(),
            EffectError::InvalidBiquadDesign
        );
        assert_eq!(
            BandPass::new(24_000.0, BiquadWidth::q(1.0))
                .unwrap()
                .coefficients(SampleRate::new(48_000).unwrap())
                .unwrap_err(),
            EffectError::InvalidBiquadDesign
        );
    }

    #[test]
    fn constructors_record_mode() {
        assert_eq!(
            BandPass::new(1_000.0, BiquadWidth::q(1.0)).unwrap().mode,
            BandPassMode::ConstantPeak
        );
        assert_eq!(
            BandPass::constant_skirt(1_000.0, BiquadWidth::q(1.0))
                .unwrap()
                .mode,
            BandPassMode::ConstantSkirt
        );
    }
}

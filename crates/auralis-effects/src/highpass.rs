use std::f64::consts::{FRAC_1_SQRT_2, TAU};

use auralis_core::{AudioBuffer, SampleRate};

use crate::{Biquad, BiquadCoefficients, BiquadWidth, EffectError, Result};

/// SoX-ng-style high-pass filter family.
///
/// `highpass frequency [width]` applies SoX-ng's default two-pole RBJ
/// high-pass filter with a Butterworth `0.707q` width when omitted. `highpass
/// -1 frequency` selects SoX-ng's single-pole RC-style high-pass form.
///
/// Processing is deterministic and streaming-safe when callers preserve a
/// [`crate::BiquadState`] per channel across chunks.
///
/// # Examples
///
/// ```
/// use auralis_effects::{BiquadWidth, HighPass};
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
/// HighPass::with_width(1_000.0, BiquadWidth::q(0.707))?.process_buffer(&mut audio)?;
///
/// assert!(audio.as_planar_f32().iter().all(|sample| sample.is_finite()));
/// # Ok::<(), Box<dyn std::error::Error>>(())
/// ```
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct HighPass {
    /// Cutoff frequency in hertz.
    pub frequency_hz: f64,

    /// High-pass filter mode.
    pub mode: HighPassMode,
}

impl HighPass {
    /// Default two-pole width used by SoX-ng when omitted.
    pub const DEFAULT_WIDTH: BiquadWidth = BiquadWidth::Q(FRAC_1_SQRT_2);

    /// Creates SoX-ng's default two-pole high-pass filter.
    ///
    /// # Errors
    ///
    /// Returns [`EffectError::InvalidBiquadDesign`] when `frequency_hz` is not
    /// finite and positive.
    pub fn new(frequency_hz: f64) -> Result<Self> {
        Self::with_width(frequency_hz, Self::DEFAULT_WIDTH)
    }

    /// Creates a two-pole RBJ high-pass filter with an explicit width.
    ///
    /// # Errors
    ///
    /// Returns [`EffectError::InvalidBiquadDesign`] when `frequency_hz` or
    /// `width` is outside the supported design range.
    pub fn with_width(frequency_hz: f64, width: BiquadWidth) -> Result<Self> {
        let filter = Self {
            frequency_hz,
            mode: HighPassMode::RbjTwoPole { width },
        };
        filter.validate_static()?;
        Ok(filter)
    }

    /// Creates SoX-ng's `highpass -1 frequency` single-pole form.
    ///
    /// # Errors
    ///
    /// Returns [`EffectError::InvalidBiquadDesign`] when `frequency_hz` is not
    /// finite and positive.
    pub fn one_pole(frequency_hz: f64) -> Result<Self> {
        let filter = Self {
            frequency_hz,
            mode: HighPassMode::OnePole,
        };
        filter.validate_static()?;
        Ok(filter)
    }

    /// Returns normalized coefficients for a concrete sample rate.
    ///
    /// # Errors
    ///
    /// Returns [`EffectError::InvalidBiquadDesign`] when the configured
    /// frequency is at or above Nyquist or width conversion produces an invalid
    /// coefficient set.
    pub fn coefficients(self, sample_rate: SampleRate) -> Result<BiquadCoefficients> {
        let sample_rate_hz = f64::from(sample_rate.as_u32());
        match self.mode {
            HighPassMode::RbjTwoPole { width } => Ok(BiquadCoefficients::rbj_high_pass(
                sample_rate_hz,
                self.frequency_hz,
                width,
            )?),
            HighPassMode::OnePole => self.one_pole_coefficients(sample_rate_hz),
        }
    }

    /// Applies the high-pass filter independently to every channel.
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

        if let HighPassMode::RbjTwoPole { width } = self.mode {
            reject_invalid_width(width)?;
        }

        Ok(())
    }

    fn one_pole_coefficients(self, sample_rate_hz: f64) -> Result<BiquadCoefficients> {
        let w0 = checked_w0(sample_rate_hz, self.frequency_hz)?;
        let pole = (-w0).exp();
        let feed_forward = f64::midpoint(1.0, pole);
        BiquadCoefficients::normalized(feed_forward, -feed_forward, 0.0, -pole, 0.0)
            .map_err(|_| EffectError::InvalidBiquadDesign)
    }
}

/// SoX-ng high-pass filter mode.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum HighPassMode {
    /// Default RBJ two-pole high-pass filter.
    RbjTwoPole {
        /// Width unit used by the RBJ design.
        width: BiquadWidth,
    },

    /// SoX-ng `-1` single-pole high-pass form.
    OnePole,
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

fn checked_w0(sample_rate_hz: f64, frequency_hz: f64) -> Result<f64> {
    if !sample_rate_hz.is_finite()
        || !frequency_hz.is_finite()
        || sample_rate_hz <= 0.0
        || frequency_hz <= 0.0
        || frequency_hz >= sample_rate_hz / 2.0
    {
        return Err(EffectError::InvalidBiquadDesign);
    }

    Ok(TAU * frequency_hz / sample_rate_hz)
}

#[cfg(test)]
mod tests {
    use super::{HighPass, HighPassMode};
    use crate::{BiquadCoefficients, BiquadWidth, EffectError};
    use auralis_core::SampleRate;

    #[test]
    fn default_mode_uses_rbj_high_pass_coefficients() {
        let high_pass = HighPass::new(1_000.0).unwrap();

        assert_eq!(
            high_pass
                .coefficients(SampleRate::new(48_000).unwrap())
                .unwrap(),
            BiquadCoefficients::rbj_high_pass(48_000.0, 1_000.0, HighPass::DEFAULT_WIDTH).unwrap()
        );
    }

    #[test]
    fn explicit_width_uses_rbj_high_pass_coefficients() {
        let high_pass = HighPass::with_width(1_000.0, BiquadWidth::octaves(0.5)).unwrap();

        assert_eq!(
            high_pass
                .coefficients(SampleRate::new(48_000).unwrap())
                .unwrap(),
            BiquadCoefficients::rbj_high_pass(48_000.0, 1_000.0, BiquadWidth::octaves(0.5),)
                .unwrap()
        );
    }

    #[test]
    fn one_pole_coefficients_match_sox_ng_formula() {
        let coefficients = HighPass::one_pole(1_000.0)
            .unwrap()
            .coefficients(SampleRate::new(48_000).unwrap())
            .unwrap();
        let pole = (-std::f64::consts::TAU * 1_000.0 / 48_000.0).exp();
        let feed_forward = f64::midpoint(1.0, pole);

        assert_eq!(
            coefficients,
            BiquadCoefficients::normalized(feed_forward, -feed_forward, 0.0, -pole, 0.0).unwrap()
        );
    }

    #[test]
    fn constructors_record_mode() {
        assert!(matches!(
            HighPass::new(1_000.0).unwrap().mode,
            HighPassMode::RbjTwoPole { .. }
        ));
        assert_eq!(
            HighPass::one_pole(1_000.0).unwrap().mode,
            HighPassMode::OnePole
        );
    }

    #[test]
    fn rejects_invalid_design_values() {
        assert_eq!(
            HighPass::new(0.0).unwrap_err(),
            EffectError::InvalidBiquadDesign
        );
        assert_eq!(
            HighPass::with_width(1_000.0, BiquadWidth::slope(0.5)).unwrap_err(),
            EffectError::InvalidBiquadDesign
        );
        assert_eq!(
            HighPass::one_pole(f64::NAN).unwrap_err(),
            EffectError::InvalidBiquadDesign
        );
        assert_eq!(
            HighPass::new(24_000.0)
                .unwrap()
                .coefficients(SampleRate::new(48_000).unwrap())
                .unwrap_err(),
            EffectError::InvalidBiquadDesign
        );
    }
}

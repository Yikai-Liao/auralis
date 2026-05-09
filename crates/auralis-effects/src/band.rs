use std::f64::consts::TAU;

use auralis_core::{AudioBuffer, SampleRate};

use crate::{Biquad, BiquadCoefficients, BiquadWidth, EffectError, Result};

/// SoX-ng-style resonator band-pass filter.
///
/// `band frequency [width]` uses the historical all-pole resonator from
/// SoX-ng's `band` effect. The optional `-n` mode applies the alternate
/// unpitched/noise scaling with roughly 11 dB more gain. Width defaults to half
/// the center frequency and accepts hertz, kilohertz, quality-factor, or
/// octave units.
///
/// Processing is deterministic and streaming-safe when callers preserve a
/// [`crate::BiquadState`] per channel across chunks.
///
/// # Examples
///
/// ```
/// use auralis_effects::{Band, BiquadWidth};
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
/// Band::new(1_000.0, Some(BiquadWidth::q(2.0)))?.process_buffer(&mut audio)?;
///
/// assert!(audio.as_planar_f32().iter().all(|sample| sample.is_finite()));
/// # Ok::<(), Box<dyn std::error::Error>>(())
/// ```
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Band {
    /// Center frequency in hertz.
    pub frequency_hz: f64,

    /// Bandwidth setting. `None` means SoX-ng's default `frequency / 2`.
    pub width: Option<BiquadWidth>,

    /// Resonator gain scaling mode.
    pub mode: BandMode,
}

impl Band {
    /// Creates the default pitched-audio `band frequency [width]` filter.
    ///
    /// # Errors
    ///
    /// Returns [`EffectError::InvalidBiquadDesign`] when `frequency_hz` or
    /// `width` is outside the supported design range.
    pub fn new(frequency_hz: f64, width: Option<BiquadWidth>) -> Result<Self> {
        let filter = Self {
            frequency_hz,
            width,
            mode: BandMode::Pitched,
        };
        filter.validate_static()?;
        Ok(filter)
    }

    /// Creates the `band -n frequency [width]` unpitched/noise form.
    ///
    /// # Errors
    ///
    /// Returns [`EffectError::InvalidBiquadDesign`] when `frequency_hz` or
    /// `width` is outside the supported design range.
    pub fn unpitched(frequency_hz: f64, width: Option<BiquadWidth>) -> Result<Self> {
        let filter = Self {
            frequency_hz,
            width,
            mode: BandMode::Unpitched,
        };
        filter.validate_static()?;
        Ok(filter)
    }

    /// Returns normalized resonator coefficients for a concrete sample rate.
    ///
    /// # Errors
    ///
    /// Returns [`EffectError::InvalidBiquadDesign`] when the configured
    /// frequency is at or above Nyquist or width conversion produces an invalid
    /// coefficient set.
    pub fn coefficients(self, sample_rate: SampleRate) -> Result<BiquadCoefficients> {
        let sample_rate_hz = f64::from(sample_rate.as_u32());
        if !sample_rate_hz.is_finite()
            || sample_rate_hz <= 0.0
            || !self.frequency_hz.is_finite()
            || self.frequency_hz <= 0.0
            || self.frequency_hz >= sample_rate_hz / 2.0
        {
            return Err(EffectError::InvalidBiquadDesign);
        }

        let bandwidth_hz = self.bandwidth_hz()?;
        let a2 = (-TAU * bandwidth_hz / sample_rate_hz).exp();
        let a1 = -4.0 * a2 / (1.0 + a2) * (TAU * self.frequency_hz / sample_rate_hz).cos();
        let base_gain = finite_non_negative(1.0 - a1 * a1 / (4.0 * a2))?.sqrt() * (1.0 - a2);
        let b0 = match self.mode {
            BandMode::Pitched => base_gain,
            BandMode::Unpitched => {
                let numerator = ((1.0 + a2).powi(2) - a1.powi(2)) * (1.0 - a2) / (1.0 + a2);
                finite_non_negative(numerator)?.sqrt()
            }
        };

        BiquadCoefficients::normalized(b0, 0.0, 0.0, a1, a2)
            .map_err(|_| EffectError::InvalidBiquadDesign)
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

        if let Some(width) = self.width {
            reject_invalid_width(width)?;
        }

        Ok(())
    }

    fn bandwidth_hz(self) -> Result<f64> {
        let bandwidth = match self.width {
            None => self.frequency_hz / 2.0,
            Some(BiquadWidth::Hertz(value)) => value,
            Some(BiquadWidth::Kilohertz(value)) => value * 1000.0,
            Some(BiquadWidth::Q(value)) => self.frequency_hz / value,
            Some(BiquadWidth::Octaves(value)) => {
                self.frequency_hz * (2.0_f64.powf(value) - 1.0) * 2.0_f64.powf(-0.5 * value)
            }
            Some(BiquadWidth::Slope(_)) => return Err(EffectError::InvalidBiquadDesign),
        };

        if bandwidth.is_finite() && bandwidth > 0.0 {
            Ok(bandwidth)
        } else {
            Err(EffectError::InvalidBiquadDesign)
        }
    }
}

/// SoX-ng `band` scaling mode.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BandMode {
    /// Default pitched-audio scaling.
    Pitched,

    /// `-n` alternate scaling for unpitched/noise input.
    Unpitched,
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

fn finite_non_negative(value: f64) -> Result<f64> {
    if value.is_finite() && value >= 0.0 {
        Ok(value)
    } else {
        Err(EffectError::InvalidBiquadDesign)
    }
}

#[cfg(test)]
mod tests {
    use super::{Band, BandMode};
    use crate::{BiquadWidth, EffectError};
    use auralis_core::SampleRate;

    #[test]
    fn default_coefficients_match_sox_ng_resonator_formula() {
        let coefficients = Band::new(1_000.0, Some(BiquadWidth::hertz(500.0)))
            .unwrap()
            .coefficients(SampleRate::new(48_000).unwrap())
            .unwrap();
        let a2 = (-std::f64::consts::TAU * 500.0 / 48_000.0).exp();
        let a1 = -4.0 * a2 / (1.0 + a2) * (std::f64::consts::TAU * 1_000.0 / 48_000.0).cos();
        let b0 = (1.0 - a1 * a1 / (4.0 * a2)).sqrt() * (1.0 - a2);

        assert_eq!(coefficients.b0.to_bits(), b0.to_bits());
        assert_eq!(coefficients.b1.to_bits(), 0.0_f64.to_bits());
        assert_eq!(coefficients.b2.to_bits(), 0.0_f64.to_bits());
        assert_eq!(coefficients.a1.to_bits(), a1.to_bits());
        assert_eq!(coefficients.a2.to_bits(), a2.to_bits());
    }

    #[test]
    fn unpitched_coefficients_use_noise_scaling() {
        let pitched = Band::new(1_000.0, Some(BiquadWidth::q(2.0)))
            .unwrap()
            .coefficients(SampleRate::new(48_000).unwrap())
            .unwrap();
        let unpitched = Band::unpitched(1_000.0, Some(BiquadWidth::q(2.0)))
            .unwrap()
            .coefficients(SampleRate::new(48_000).unwrap())
            .unwrap();

        assert!(unpitched.b0 > pitched.b0);
        assert_eq!(unpitched.a1.to_bits(), pitched.a1.to_bits());
        assert_eq!(unpitched.a2.to_bits(), pitched.a2.to_bits());
    }

    #[test]
    fn width_units_and_default_width_resolve_to_hertz_bandwidth() {
        let sample_rate = SampleRate::new(48_000).unwrap();
        let default = Band::new(1_000.0, None)
            .unwrap()
            .coefficients(sample_rate)
            .unwrap();
        let hertz = Band::new(1_000.0, Some(BiquadWidth::hertz(500.0)))
            .unwrap()
            .coefficients(sample_rate)
            .unwrap();
        let kilohertz = Band::new(1_000.0, Some(BiquadWidth::kilohertz(0.5)))
            .unwrap()
            .coefficients(sample_rate)
            .unwrap();
        let q = Band::new(1_000.0, Some(BiquadWidth::q(2.0)))
            .unwrap()
            .coefficients(sample_rate)
            .unwrap();

        assert_eq!(default, hertz);
        assert_eq!(hertz, kilohertz);
        assert_eq!(hertz, q);
    }

    #[test]
    fn rejects_invalid_design_values() {
        assert_eq!(
            Band::new(0.0, Some(BiquadWidth::q(1.0))).unwrap_err(),
            EffectError::InvalidBiquadDesign
        );
        assert_eq!(
            Band::new(1_000.0, Some(BiquadWidth::slope(0.5))).unwrap_err(),
            EffectError::InvalidBiquadDesign
        );
        assert_eq!(
            Band::new(24_000.0, Some(BiquadWidth::q(1.0)))
                .unwrap()
                .coefficients(SampleRate::new(48_000).unwrap())
                .unwrap_err(),
            EffectError::InvalidBiquadDesign
        );
    }

    #[test]
    fn constructors_record_mode() {
        assert_eq!(Band::new(1_000.0, None).unwrap().mode, BandMode::Pitched);
        assert_eq!(
            Band::unpitched(1_000.0, None).unwrap().mode,
            BandMode::Unpitched
        );
    }
}

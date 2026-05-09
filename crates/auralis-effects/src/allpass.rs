use std::f64::consts::TAU;

use auralis_core::{AudioBuffer, SampleRate};

use crate::{Biquad, BiquadCoefficients, BiquadWidth, EffectError, Result};

/// SoX-ng-style all-pass filter family.
///
/// The default command form, `allpass frequency width`, designs the RBJ
/// two-pole all-pass filter from the shared biquad coefficient helpers. The
/// `-1` and `-2` forms expose SoX-ng's alternate one-pole and simple two-pole
/// all-pass filters, where the width argument is intentionally absent.
///
/// The processor is deterministic and streaming-safe when callers preserve a
/// [`crate::BiquadState`] per channel across chunks.
///
/// # Examples
///
/// ```
/// use auralis_effects::{AllPass, BiquadWidth};
///
/// let mut samples = [1.0, 0.0, 0.0];
/// AllPass::new(1_000.0, BiquadWidth::q(0.707))?
///     .process_mono_samples(&mut samples, auralis_core::SampleRate::new(48_000)?)?;
///
/// assert!(samples.iter().all(|sample| sample.is_finite()));
/// # Ok::<(), auralis_effects::EffectError>(())
/// ```
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct AllPass {
    /// Center or cutoff frequency in hertz.
    pub frequency_hz: f64,

    /// All-pass filter mode.
    pub mode: AllPassMode,
}

impl AllPass {
    /// Creates the default RBJ two-pole all-pass filter.
    ///
    /// # Errors
    ///
    /// Returns [`EffectError::InvalidBiquadDesign`] when `frequency_hz` or
    /// `width` is outside the supported filter-design range.
    pub fn new(frequency_hz: f64, width: BiquadWidth) -> Result<Self> {
        let filter = Self {
            frequency_hz,
            mode: AllPassMode::RbjTwoPole { width },
        };
        filter.validate_static()?;
        Ok(filter)
    }

    /// Creates SoX-ng's experimental `allpass -1 frequency` one-pole form.
    ///
    /// # Errors
    ///
    /// Returns [`EffectError::InvalidBiquadDesign`] when `frequency_hz` is not
    /// finite and positive.
    pub fn one_pole(frequency_hz: f64) -> Result<Self> {
        let filter = Self {
            frequency_hz,
            mode: AllPassMode::OnePole,
        };
        filter.validate_static()?;
        Ok(filter)
    }

    /// Creates SoX-ng's experimental `allpass -2 frequency` two-pole form.
    ///
    /// # Errors
    ///
    /// Returns [`EffectError::InvalidBiquadDesign`] when `frequency_hz` is not
    /// finite and positive.
    pub fn two_pole(frequency_hz: f64) -> Result<Self> {
        let filter = Self {
            frequency_hz,
            mode: AllPassMode::TwoPole,
        };
        filter.validate_static()?;
        Ok(filter)
    }

    /// Returns the normalized biquad coefficients for a concrete sample rate.
    ///
    /// # Errors
    ///
    /// Returns [`EffectError::InvalidBiquadDesign`] when the configured
    /// frequency is at or above Nyquist for `sample_rate`.
    pub fn coefficients(self, sample_rate: SampleRate) -> Result<BiquadCoefficients> {
        let sample_rate_hz = f64::from(sample_rate.as_u32());
        match self.mode {
            AllPassMode::RbjTwoPole { width } => {
                BiquadCoefficients::rbj_all_pass(sample_rate_hz, self.frequency_hz, width)
            }
            AllPassMode::OnePole => self.one_pole_coefficients(sample_rate_hz),
            AllPassMode::TwoPole => self.two_pole_coefficients(sample_rate_hz),
        }
    }

    /// Applies the all-pass filter independently to every channel.
    ///
    /// # Errors
    ///
    /// Returns [`EffectError::InvalidBiquadDesign`] if this buffer's sample
    /// rate makes the configured frequency invalid.
    ///
    /// # Panics
    ///
    /// Panics only if a validated [`AudioBuffer`] cannot return one of its
    /// declared channels.
    pub fn process_buffer(self, audio: &mut AudioBuffer) -> Result<()> {
        let coefficients = self.coefficients(audio.spec().sample_rate())?;
        Biquad::new(coefficients).process_buffer(audio);
        Ok(())
    }

    /// Applies the all-pass filter to one mono slice with a fresh zero state.
    ///
    /// # Errors
    ///
    /// Returns [`EffectError::InvalidBiquadDesign`] when `sample_rate` makes
    /// the configured frequency invalid.
    pub fn process_mono_samples(self, samples: &mut [f32], sample_rate: SampleRate) -> Result<()> {
        let coefficients = self.coefficients(sample_rate)?;
        Biquad::new(coefficients).process_mono_samples(samples);
        Ok(())
    }

    fn validate_static(self) -> Result<()> {
        if !self.frequency_hz.is_finite() || self.frequency_hz <= 0.0 {
            return Err(EffectError::InvalidBiquadDesign);
        }

        match self.mode {
            AllPassMode::RbjTwoPole { width } => {
                reject_invalid_width(width)?;
            }
            AllPassMode::OnePole | AllPassMode::TwoPole => {}
        }

        Ok(())
    }

    fn one_pole_coefficients(self, sample_rate_hz: f64) -> Result<BiquadCoefficients> {
        let w0 = checked_w0(sample_rate_hz, self.frequency_hz)?;
        let pole = (-w0).exp();
        BiquadCoefficients::normalized(pole, -1.0, 0.0, -pole, 0.0)
    }

    fn two_pole_coefficients(self, sample_rate_hz: f64) -> Result<BiquadCoefficients> {
        let w0 = checked_w0(sample_rate_hz, self.frequency_hz)?;
        let sin_w0 = w0.sin();
        let cos_w0 = w0.cos();
        BiquadCoefficients::from_raw(
            1.0 - sin_w0,
            -2.0 * cos_w0,
            1.0 + sin_w0,
            1.0 + sin_w0,
            -2.0 * cos_w0,
            1.0 - sin_w0,
        )
    }
}

/// SoX-ng all-pass filter mode.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum AllPassMode {
    /// Default RBJ two-pole all-pass filter with explicit width.
    RbjTwoPole {
        /// Width unit used by the RBJ design.
        width: BiquadWidth,
    },

    /// SoX-ng `-1` one-pole all-pass form.
    OnePole,

    /// SoX-ng `-2` simple two-pole all-pass form.
    TwoPole,
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
    use super::{AllPass, AllPassMode};
    use crate::{BiquadCoefficients, BiquadWidth, EffectError};
    use auralis_core::{
        AudioBuffer, AudioSpec, ChannelCount, FrameCount, SampleFormat, SampleRate,
    };

    #[test]
    fn default_mode_uses_rbj_all_pass_coefficients() {
        let all_pass = AllPass::new(1_000.0, BiquadWidth::q(2.0_f64.sqrt().recip())).unwrap();

        assert_eq!(
            all_pass
                .coefficients(SampleRate::new(48_000).unwrap())
                .unwrap(),
            BiquadCoefficients::rbj_all_pass(
                48_000.0,
                1_000.0,
                BiquadWidth::q(2.0_f64.sqrt().recip()),
            )
            .unwrap()
        );
    }

    #[test]
    fn one_pole_coefficients_match_sox_ng_formula() {
        let coefficients = AllPass::one_pole(1_000.0)
            .unwrap()
            .coefficients(SampleRate::new(48_000).unwrap())
            .unwrap();
        let pole = (-std::f64::consts::TAU * 1_000.0 / 48_000.0).exp();

        assert_eq!(
            coefficients,
            BiquadCoefficients::normalized(pole, -1.0, 0.0, -pole, 0.0).unwrap()
        );
    }

    #[test]
    fn two_pole_coefficients_match_sox_ng_formula() {
        let coefficients = AllPass::two_pole(1_000.0)
            .unwrap()
            .coefficients(SampleRate::new(48_000).unwrap())
            .unwrap();
        let w0 = std::f64::consts::TAU * 1_000.0 / 48_000.0;
        let sin_w0 = w0.sin();
        let cos_w0 = w0.cos();

        assert_eq!(
            coefficients,
            BiquadCoefficients::from_raw(
                1.0 - sin_w0,
                -2.0 * cos_w0,
                1.0 + sin_w0,
                1.0 + sin_w0,
                -2.0 * cos_w0,
                1.0 - sin_w0,
            )
            .unwrap()
        );
    }

    #[test]
    fn process_buffer_filters_each_channel_independently() {
        let spec = AudioSpec::new(
            SampleRate::new(48_000).unwrap(),
            ChannelCount::new(2).unwrap(),
            SampleFormat::Float32,
        );
        let mut audio = AudioBuffer::from_planar_f32(
            spec,
            FrameCount::new(4),
            vec![1.0, 0.0, 0.0, 0.0, -1.0, 0.0, 0.0, 0.0],
        )
        .unwrap();

        AllPass::new(1_000.0, BiquadWidth::q(1.0))
            .unwrap()
            .process_buffer(&mut audio)
            .unwrap();

        assert_eq!(
            audio.as_planar_f32()[0].to_bits(),
            (-audio.as_planar_f32()[4]).to_bits()
        );
        assert!(
            audio
                .as_planar_f32()
                .iter()
                .all(|sample| sample.is_finite())
        );
    }

    #[test]
    fn invalid_design_inputs_are_rejected() {
        assert_eq!(
            AllPass::new(0.0, BiquadWidth::q(1.0)).unwrap_err(),
            EffectError::InvalidBiquadDesign
        );
        assert_eq!(
            AllPass::new(1_000.0, BiquadWidth::slope(0.5)).unwrap_err(),
            EffectError::InvalidBiquadDesign
        );
        assert_eq!(
            AllPass::one_pole(f64::NAN).unwrap_err(),
            EffectError::InvalidBiquadDesign
        );
        assert_eq!(
            AllPass::new(24_000.0, BiquadWidth::q(1.0))
                .unwrap()
                .coefficients(SampleRate::new(48_000).unwrap())
                .unwrap_err(),
            EffectError::InvalidBiquadDesign
        );
    }

    #[test]
    fn modes_are_distinct() {
        assert_ne!(
            AllPass::new(1_000.0, BiquadWidth::q(1.0)).unwrap().mode,
            AllPassMode::OnePole
        );
    }
}

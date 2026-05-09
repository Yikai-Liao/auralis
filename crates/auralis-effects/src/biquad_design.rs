use std::f64::consts::{LN_2, PI, TAU};

use crate::{BiquadCoefficients, EffectError, Result};

/// Width parameter used by RBJ audio-EQ biquad coefficient helpers.
///
/// SoX-ng's tone and two-pole filter commands accept the same family of width
/// units: quality factor, octave bandwidth, hertz bandwidth, kilohertz
/// bandwidth, and shelf slope. Slope is valid only for shelf filters.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum BiquadWidth {
    /// Quality factor `Q`.
    Q(f64),

    /// Bandwidth in octaves.
    Octaves(f64),

    /// Bandwidth in hertz.
    Hertz(f64),

    /// Bandwidth in kilohertz.
    Kilohertz(f64),

    /// Shelf slope `S`, valid only for shelf helpers and limited to `0 < S <= 1`.
    Slope(f64),
}

impl BiquadWidth {
    /// Creates a quality-factor width.
    #[must_use]
    pub const fn q(value: f64) -> Self {
        Self::Q(value)
    }

    /// Creates an octave-bandwidth width.
    #[must_use]
    pub const fn octaves(value: f64) -> Self {
        Self::Octaves(value)
    }

    /// Creates a hertz-bandwidth width.
    #[must_use]
    pub const fn hertz(value: f64) -> Self {
        Self::Hertz(value)
    }

    /// Creates a kilohertz-bandwidth width.
    #[must_use]
    pub const fn kilohertz(value: f64) -> Self {
        Self::Kilohertz(value)
    }

    /// Creates a shelf-slope width.
    #[must_use]
    pub const fn slope(value: f64) -> Self {
        Self::Slope(value)
    }

    fn alpha(self, frequency_hz: f64, w0: f64, sin_w0: f64) -> Result<f64> {
        let alpha = match self {
            Self::Q(q) => positive(q).map(|q| sin_w0 / (2.0 * q)),
            Self::Octaves(octaves) => positive(octaves)
                .map(|octaves| sin_w0 * (LN_2 / 2.0 * octaves * w0 / sin_w0).sinh()),
            Self::Hertz(hertz) => {
                positive(hertz).map(|hertz| sin_w0 / (2.0 * frequency_hz / hertz))
            }
            Self::Kilohertz(kilohertz) => positive(kilohertz)
                .map(|kilohertz| sin_w0 / (2.0 * frequency_hz / (kilohertz * 1000.0))),
            Self::Slope(_) => Err(EffectError::InvalidBiquadDesign),
        }?;
        finite_design_value(alpha)
    }

    fn shelf_alpha(self, frequency_hz: f64, w0: f64, sin_w0: f64, gain_db: f64) -> Result<f64> {
        match self {
            Self::Slope(slope) => {
                if !slope.is_finite() || slope <= 0.0 || slope > 1.0 {
                    return Err(EffectError::InvalidBiquadDesign);
                }
                let amplitude = db_to_rbj_amplitude(gain_db)?;
                finite_design_value(
                    sin_w0 / 2.0
                        * ((amplitude + amplitude.recip()) * (slope.recip() - 1.0) + 2.0).sqrt(),
                )
            }
            other => other.alpha(frequency_hz, w0, sin_w0),
        }
    }
}

impl BiquadCoefficients {
    /// Designs RBJ low-pass coefficients for `frequency_hz`.
    ///
    /// # Errors
    ///
    /// Returns [`EffectError::InvalidBiquadDesign`] if the sample rate,
    /// frequency, or width is not finite and positive, if the frequency is at
    /// or above Nyquist, or if the width unit is not valid for this helper.
    pub fn rbj_low_pass(
        sample_rate_hz: f64,
        frequency_hz: f64,
        width: BiquadWidth,
    ) -> Result<Self> {
        let design = RbjDesign::new(sample_rate_hz, frequency_hz)?;
        let alpha = width.alpha(frequency_hz, design.w0, design.sin_w0)?;
        Self::from_raw(
            (1.0 - design.cos_w0) / 2.0,
            1.0 - design.cos_w0,
            (1.0 - design.cos_w0) / 2.0,
            1.0 + alpha,
            -2.0 * design.cos_w0,
            1.0 - alpha,
        )
    }

    /// Designs RBJ high-pass coefficients for `frequency_hz`.
    ///
    /// # Errors
    ///
    /// Returns [`EffectError::InvalidBiquadDesign`] if a design parameter is
    /// outside the supported range.
    pub fn rbj_high_pass(
        sample_rate_hz: f64,
        frequency_hz: f64,
        width: BiquadWidth,
    ) -> Result<Self> {
        let design = RbjDesign::new(sample_rate_hz, frequency_hz)?;
        let alpha = width.alpha(frequency_hz, design.w0, design.sin_w0)?;
        Self::from_raw(
            1.0_f64.midpoint(design.cos_w0),
            -(1.0 + design.cos_w0),
            1.0_f64.midpoint(design.cos_w0),
            1.0 + alpha,
            -2.0 * design.cos_w0,
            1.0 - alpha,
        )
    }

    /// Designs RBJ constant-skirt-gain band-pass coefficients.
    ///
    /// This is SoX-ng's `bandpass -c` shape, where the peak gain equals `Q`
    /// when a quality-factor width is used.
    ///
    /// # Errors
    ///
    /// Returns [`EffectError::InvalidBiquadDesign`] if a design parameter is
    /// outside the supported range.
    pub fn rbj_band_pass_constant_skirt(
        sample_rate_hz: f64,
        frequency_hz: f64,
        width: BiquadWidth,
    ) -> Result<Self> {
        let design = RbjDesign::new(sample_rate_hz, frequency_hz)?;
        let alpha = width.alpha(frequency_hz, design.w0, design.sin_w0)?;
        Self::from_raw(
            design.sin_w0 / 2.0,
            0.0,
            -design.sin_w0 / 2.0,
            1.0 + alpha,
            -2.0 * design.cos_w0,
            1.0 - alpha,
        )
    }

    /// Designs RBJ constant-peak-gain band-pass coefficients.
    ///
    /// # Errors
    ///
    /// Returns [`EffectError::InvalidBiquadDesign`] if a design parameter is
    /// outside the supported range.
    pub fn rbj_band_pass_constant_peak(
        sample_rate_hz: f64,
        frequency_hz: f64,
        width: BiquadWidth,
    ) -> Result<Self> {
        let design = RbjDesign::new(sample_rate_hz, frequency_hz)?;
        let alpha = width.alpha(frequency_hz, design.w0, design.sin_w0)?;
        Self::from_raw(
            alpha,
            0.0,
            -alpha,
            1.0 + alpha,
            -2.0 * design.cos_w0,
            1.0 - alpha,
        )
    }

    /// Designs RBJ notch coefficients.
    ///
    /// # Errors
    ///
    /// Returns [`EffectError::InvalidBiquadDesign`] if a design parameter is
    /// outside the supported range.
    pub fn rbj_notch(sample_rate_hz: f64, frequency_hz: f64, width: BiquadWidth) -> Result<Self> {
        let design = RbjDesign::new(sample_rate_hz, frequency_hz)?;
        let alpha = width.alpha(frequency_hz, design.w0, design.sin_w0)?;
        Self::from_raw(
            1.0,
            -2.0 * design.cos_w0,
            1.0,
            1.0 + alpha,
            -2.0 * design.cos_w0,
            1.0 - alpha,
        )
    }

    /// Designs RBJ notch coefficients for SoX-ng's `bandreject` effect.
    ///
    /// # Errors
    ///
    /// Returns [`EffectError::InvalidBiquadDesign`] if a design parameter is
    /// outside the supported range.
    pub fn rbj_band_reject(
        sample_rate_hz: f64,
        frequency_hz: f64,
        width: BiquadWidth,
    ) -> Result<Self> {
        Self::rbj_notch(sample_rate_hz, frequency_hz, width)
    }

    /// Designs RBJ two-pole all-pass coefficients.
    ///
    /// # Errors
    ///
    /// Returns [`EffectError::InvalidBiquadDesign`] if a design parameter is
    /// outside the supported range.
    pub fn rbj_all_pass(
        sample_rate_hz: f64,
        frequency_hz: f64,
        width: BiquadWidth,
    ) -> Result<Self> {
        let design = RbjDesign::new(sample_rate_hz, frequency_hz)?;
        let alpha = width.alpha(frequency_hz, design.w0, design.sin_w0)?;
        Self::from_raw(
            1.0 - alpha,
            -2.0 * design.cos_w0,
            1.0 + alpha,
            1.0 + alpha,
            -2.0 * design.cos_w0,
            1.0 - alpha,
        )
    }

    /// Designs RBJ peaking-EQ coefficients for `gain_db`.
    ///
    /// # Errors
    ///
    /// Returns [`EffectError::InvalidBiquadDesign`] if a design parameter is
    /// outside the supported range.
    pub fn rbj_peaking_eq(
        sample_rate_hz: f64,
        frequency_hz: f64,
        width: BiquadWidth,
        gain_db: f64,
    ) -> Result<Self> {
        let design = RbjDesign::new(sample_rate_hz, frequency_hz)?;
        let amplitude = db_to_rbj_amplitude(gain_db)?;
        let alpha = width.alpha(frequency_hz, design.w0, design.sin_w0)?;
        Self::from_raw(
            1.0 + alpha * amplitude,
            -2.0 * design.cos_w0,
            1.0 - alpha * amplitude,
            1.0 + alpha / amplitude,
            -2.0 * design.cos_w0,
            1.0 - alpha / amplitude,
        )
    }

    /// Designs RBJ low-shelf coefficients for `gain_db`.
    ///
    /// # Errors
    ///
    /// Returns [`EffectError::InvalidBiquadDesign`] if a design parameter is
    /// outside the supported range.
    pub fn rbj_low_shelf(
        sample_rate_hz: f64,
        frequency_hz: f64,
        width: BiquadWidth,
        gain_db: f64,
    ) -> Result<Self> {
        let design = RbjDesign::new(sample_rate_hz, frequency_hz)?;
        let amplitude = db_to_rbj_amplitude(gain_db)?;
        let alpha = width.shelf_alpha(frequency_hz, design.w0, design.sin_w0, gain_db)?;
        let sqrt_amplitude = amplitude.sqrt();
        Self::from_raw(
            amplitude
                * ((amplitude + 1.0) - (amplitude - 1.0) * design.cos_w0
                    + 2.0 * sqrt_amplitude * alpha),
            2.0 * amplitude * ((amplitude - 1.0) - (amplitude + 1.0) * design.cos_w0),
            amplitude
                * ((amplitude + 1.0)
                    - (amplitude - 1.0) * design.cos_w0
                    - 2.0 * sqrt_amplitude * alpha),
            (amplitude + 1.0) + (amplitude - 1.0) * design.cos_w0 + 2.0 * sqrt_amplitude * alpha,
            -2.0 * ((amplitude - 1.0) + (amplitude + 1.0) * design.cos_w0),
            (amplitude + 1.0) + (amplitude - 1.0) * design.cos_w0 - 2.0 * sqrt_amplitude * alpha,
        )
    }

    /// Designs RBJ high-shelf coefficients for `gain_db`.
    ///
    /// # Errors
    ///
    /// Returns [`EffectError::InvalidBiquadDesign`] if a design parameter is
    /// outside the supported range.
    pub fn rbj_high_shelf(
        sample_rate_hz: f64,
        frequency_hz: f64,
        width: BiquadWidth,
        gain_db: f64,
    ) -> Result<Self> {
        let design = RbjDesign::new(sample_rate_hz, frequency_hz)?;
        let amplitude = db_to_rbj_amplitude(gain_db)?;
        let alpha = width.shelf_alpha(frequency_hz, design.w0, design.sin_w0, gain_db)?;
        let sqrt_amplitude = amplitude.sqrt();
        Self::from_raw(
            amplitude
                * ((amplitude + 1.0)
                    + (amplitude - 1.0) * design.cos_w0
                    + 2.0 * sqrt_amplitude * alpha),
            -2.0 * amplitude * ((amplitude - 1.0) + (amplitude + 1.0) * design.cos_w0),
            amplitude
                * ((amplitude + 1.0) + (amplitude - 1.0) * design.cos_w0
                    - 2.0 * sqrt_amplitude * alpha),
            (amplitude + 1.0) - (amplitude - 1.0) * design.cos_w0 + 2.0 * sqrt_amplitude * alpha,
            2.0 * ((amplitude - 1.0) - (amplitude + 1.0) * design.cos_w0),
            (amplitude + 1.0) - (amplitude - 1.0) * design.cos_w0 - 2.0 * sqrt_amplitude * alpha,
        )
    }
}

#[derive(Debug, Clone, Copy)]
struct RbjDesign {
    w0: f64,
    sin_w0: f64,
    cos_w0: f64,
}

impl RbjDesign {
    fn new(sample_rate_hz: f64, frequency_hz: f64) -> Result<Self> {
        if !sample_rate_hz.is_finite()
            || !frequency_hz.is_finite()
            || sample_rate_hz <= 0.0
            || frequency_hz <= 0.0
            || frequency_hz >= sample_rate_hz / 2.0
        {
            return Err(EffectError::InvalidBiquadDesign);
        }

        let w0 = TAU * frequency_hz / sample_rate_hz;
        if !(0.0..PI).contains(&w0) {
            return Err(EffectError::InvalidBiquadDesign);
        }
        Ok(Self {
            w0,
            sin_w0: w0.sin(),
            cos_w0: w0.cos(),
        })
    }
}

fn positive(value: f64) -> Result<f64> {
    if value.is_finite() && value > 0.0 {
        Ok(value)
    } else {
        Err(EffectError::InvalidBiquadDesign)
    }
}

fn db_to_rbj_amplitude(gain_db: f64) -> Result<f64> {
    if !gain_db.is_finite() {
        return Err(EffectError::InvalidBiquadDesign);
    }
    finite_design_value(10.0_f64.powf(gain_db / 40.0))
}

fn finite_design_value(value: f64) -> Result<f64> {
    if value.is_finite() {
        Ok(value)
    } else {
        Err(EffectError::InvalidBiquadDesign)
    }
}

#[cfg(test)]
mod tests {
    use super::{BiquadCoefficients, BiquadWidth};
    use crate::EffectError;

    #[test]
    fn rbj_low_and_high_pass_helpers_match_cookbook_values() {
        let low_pass = BiquadCoefficients::rbj_low_pass(
            48_000.0,
            1_000.0,
            BiquadWidth::q(2.0_f64.sqrt().recip()),
        )
        .unwrap();
        let high_pass = BiquadCoefficients::rbj_high_pass(
            48_000.0,
            1_000.0,
            BiquadWidth::q(2.0_f64.sqrt().recip()),
        )
        .unwrap();

        assert_coefficients_close(
            low_pass,
            BiquadCoefficients {
                b0: 0.003_916_126_660_547_383,
                b1: 0.007_832_253_321_094_766,
                b2: 0.003_916_126_660_547_383,
                a1: -1.815_341_082_704_568,
                a2: 0.831_005_589_346_757_6,
            },
        );
        assert_coefficients_close(
            high_pass,
            BiquadCoefficients {
                b0: 0.911_586_668_012_831_5,
                b1: -1.823_173_336_025_663,
                b2: 0.911_586_668_012_831_5,
                a1: -1.815_341_082_704_568,
                a2: 0.831_005_589_346_757_6,
            },
        );
    }

    #[test]
    fn rbj_band_notch_and_all_pass_helpers_match_cookbook_values() {
        let notch =
            BiquadCoefficients::rbj_band_reject(48_000.0, 5_000.0, BiquadWidth::q(2.0)).unwrap();
        let all_pass = BiquadCoefficients::rbj_all_pass(
            48_000.0,
            1_000.0,
            BiquadWidth::q(2.0_f64.sqrt().recip()),
        )
        .unwrap();

        assert_coefficients_close(
            notch,
            BiquadCoefficients {
                b0: 0.867_912_141_171_591,
                b1: -1.377_121_992_555_599_5,
                b2: 0.867_912_141_171_591,
                a1: -1.377_121_992_555_599_5,
                a2: 0.735_824_282_343_182,
            },
        );
        assert_coefficients_close(
            all_pass,
            BiquadCoefficients {
                b0: 0.831_005_589_346_757_6,
                b1: -1.815_341_082_704_568,
                b2: 1.0,
                a1: -1.815_341_082_704_568,
                a2: 0.831_005_589_346_757_6,
            },
        );
    }

    #[test]
    fn rbj_peaking_and_shelf_helpers_match_cookbook_values() {
        let peaking =
            BiquadCoefficients::rbj_peaking_eq(48_000.0, 1_000.0, BiquadWidth::q(1.0), 6.0)
                .unwrap();
        let low_shelf =
            BiquadCoefficients::rbj_low_shelf(48_000.0, 100.0, BiquadWidth::slope(0.5), 6.0)
                .unwrap();
        let high_shelf =
            BiquadCoefficients::rbj_high_shelf(48_000.0, 3_000.0, BiquadWidth::slope(0.5), -6.0)
                .unwrap();

        assert_coefficients_close(
            peaking,
            BiquadCoefficients {
                b0: 1.043_953_086_990_335,
                b1: -1.895_320_723_936_596_1,
                b2: 0.867_722_284_759_856_6,
                a1: -1.895_320_723_936_596_1,
                a2: 0.911_675_371_750_191_5,
            },
        );
        assert_coefficients_close(
            low_shelf,
            BiquadCoefficients {
                b0: 1.004_590_338_524_833_8,
                b1: -1.977_710_885_904_554_5,
                b2: 0.973_359_905_823_786_8,
                a1: -1.977_770_583_428_374,
                a2: 0.977_890_546_824_801_4,
            },
        );
        assert_coefficients_close(
            high_shelf,
            BiquadCoefficients {
                b0: 0.562_759_130_773_677_1,
                b1: -0.691_908_977_952_990_7,
                b2: 0.211_067_841_883_347_8,
                a1: -1.421_304_855_621_608_4,
                a2: 0.503_222_850_325_642_5,
            },
        );
    }

    #[test]
    fn rbj_width_units_match_sox_ng_interpretation() {
        let hertz = BiquadCoefficients::rbj_band_pass_constant_peak(
            48_000.0,
            2_000.0,
            BiquadWidth::hertz(250.0),
        )
        .unwrap();
        let kilohertz = BiquadCoefficients::rbj_band_pass_constant_peak(
            48_000.0,
            2_000.0,
            BiquadWidth::kilohertz(0.25),
        )
        .unwrap();
        let constant_skirt = BiquadCoefficients::rbj_band_pass_constant_skirt(
            48_000.0,
            2_000.0,
            BiquadWidth::octaves(0.5),
        )
        .unwrap();

        assert_eq!(hertz, kilohertz);
        assert!(constant_skirt.b0 > hertz.b0);
        assert_eq!(constant_skirt.b1.to_bits(), 0.0_f64.to_bits());
        assert!(constant_skirt.b2 < 0.0);
    }

    #[test]
    fn rbj_helpers_reject_invalid_design_inputs() {
        assert_eq!(
            BiquadCoefficients::rbj_low_pass(48_000.0, 24_000.0, BiquadWidth::q(0.707))
                .unwrap_err(),
            EffectError::InvalidBiquadDesign
        );
        assert_eq!(
            BiquadCoefficients::rbj_low_pass(48_000.0, 1_000.0, BiquadWidth::slope(0.5))
                .unwrap_err(),
            EffectError::InvalidBiquadDesign
        );
        assert_eq!(
            BiquadCoefficients::rbj_low_shelf(48_000.0, 100.0, BiquadWidth::slope(1.5), 6.0)
                .unwrap_err(),
            EffectError::InvalidBiquadDesign
        );
        assert_eq!(
            BiquadCoefficients::rbj_peaking_eq(48_000.0, 1_000.0, BiquadWidth::q(1.0), f64::NAN)
                .unwrap_err(),
            EffectError::InvalidBiquadDesign
        );
    }

    fn assert_coefficients_close(actual: BiquadCoefficients, expected: BiquadCoefficients) {
        assert!(
            (actual.b0 - expected.b0).abs() <= 1e-14,
            "b0: actual={} expected={}",
            actual.b0,
            expected.b0
        );
        assert!(
            (actual.b1 - expected.b1).abs() <= 1e-14,
            "b1: actual={} expected={}",
            actual.b1,
            expected.b1
        );
        assert!(
            (actual.b2 - expected.b2).abs() <= 1e-14,
            "b2: actual={} expected={}",
            actual.b2,
            expected.b2
        );
        assert!(
            (actual.a1 - expected.a1).abs() <= 1e-14,
            "a1: actual={} expected={}",
            actual.a1,
            expected.a1
        );
        assert!(
            (actual.a2 - expected.a2).abs() <= 1e-14,
            "a2: actual={} expected={}",
            actual.a2,
            expected.a2
        );
    }
}

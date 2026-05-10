use auralis_core::{AudioBuffer, SampleRate};

use crate::{EffectError, Fir, FirCoefficients, Result};

const DEFAULT_ATTENUATION_DB: f64 = 120.0;
const DEFAULT_TRANSITION_BANDWIDTH: f64 = 0.05;
const MIN_ATTENUATION_DB: f64 = 40.0;
const MAX_ATTENUATION_DB: f64 = 180.0;
const MIN_BETA: f64 = 0.0;
const MAX_BETA: f64 = 256.0;
const MIN_TRANSITION_WIDTH_HZ: f64 = 1.0;
const MIN_TAPS: u32 = 11;
const MAX_TAPS: u32 = 1_073_741_823;

/// SoX-ng-style windowed-sinc FIR filter.
///
/// Feature 6.8.5 covers the low-pass and high-pass forms:
/// `sinc [options] -freq` and `sinc [options] freq`. The filter is designed as
/// a deterministic scalar Kaiser-windowed FIR and executed through the shared
/// length-preserving FIR processor. Band-pass and band-reject ranges are owned
/// by Feature 6.8.6.
///
/// # Examples
///
/// ```
/// use auralis_core::{AudioBuffer, AudioSpec, ChannelCount, FrameCount, SampleFormat, SampleRate};
/// use auralis_effects::Sinc;
///
/// let spec = AudioSpec::new(
///     SampleRate::new(48_000)?,
///     ChannelCount::new(1)?,
///     SampleFormat::Float32,
/// );
/// let audio = AudioBuffer::from_planar_f32(spec, FrameCount::new(3), vec![1.0, 0.0, -1.0])?;
/// let filtered = Sinc::low_pass(4_000.0)?.process_buffer(&audio)?;
///
/// assert_eq!(filtered.frames(), audio.frames());
/// # Ok::<(), Box<dyn std::error::Error>>(())
/// ```
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Sinc {
    /// Frequency range covered by this Feature 6.8.5 sinc filter.
    pub band: SincBand,

    /// FIR design options.
    pub options: SincOptions,
}

impl Sinc {
    /// Creates a low-pass sinc filter with default SoX-ng options.
    ///
    /// # Errors
    ///
    /// Returns [`EffectError::InvalidSinc`] when `frequency_hz` is not finite
    /// and positive.
    pub fn low_pass(frequency_hz: f64) -> Result<Self> {
        Self::with_options(
            SincBand::LowPass {
                frequency_hz,
                delete_at_nyquist: false,
            },
            SincOptions::default(),
        )
    }

    /// Creates a high-pass sinc filter with default SoX-ng options.
    ///
    /// # Errors
    ///
    /// Returns [`EffectError::InvalidSinc`] when `frequency_hz` is not finite
    /// and positive.
    pub fn high_pass(frequency_hz: f64) -> Result<Self> {
        Self::with_options(SincBand::HighPass { frequency_hz }, SincOptions::default())
    }

    /// Creates a sinc filter with explicit design options.
    ///
    /// # Errors
    ///
    /// Returns [`EffectError::InvalidSinc`] when the frequency or option set is
    /// outside the supported SoX-ng ranges.
    pub fn with_options(band: SincBand, options: SincOptions) -> Result<Self> {
        band.validate()?;
        options.validate()?;
        Ok(Self { band, options })
    }

    /// Generates the Kaiser-windowed FIR coefficients for a sample rate.
    ///
    /// # Errors
    ///
    /// Returns [`EffectError::InvalidSinc`] when the cutoff is at or above
    /// Nyquist, except for low-pass filters with `delete_at_nyquist` enabled.
    pub fn coefficients_for_sample_rate(self, sample_rate: SampleRate) -> Result<FirCoefficients> {
        let sample_rate_hz = f64::from(sample_rate.as_u32());
        let nyquist_hz = sample_rate_hz * 0.5;
        let frequency_hz = self.band.frequency_hz();

        if frequency_hz >= nyquist_hz {
            if matches!(
                self.band,
                SincBand::LowPass {
                    delete_at_nyquist: true,
                    ..
                }
            ) {
                return FirCoefficients::new(Vec::new());
            }
            return Err(EffectError::InvalidSinc);
        }

        let mut coefficients = self.design_low_pass(sample_rate_hz, frequency_hz)?;
        if matches!(self.band, SincBand::HighPass { .. }) {
            invert_filter(&mut coefficients);
        }

        FirCoefficients::new(coefficients).map_err(|_| EffectError::InvalidSinc)
    }

    /// Applies this sinc filter to a decoded audio buffer.
    ///
    /// # Errors
    ///
    /// Returns [`EffectError::InvalidSinc`] for invalid runtime design
    /// parameters, or FIR execution errors if the output shape is not
    /// representable.
    pub fn process_buffer(self, audio: &AudioBuffer) -> Result<AudioBuffer> {
        let coefficients = self.coefficients_for_sample_rate(audio.spec().sample_rate())?;
        Fir::from_coefficients(coefficients).process_buffer(audio)
    }

    fn design_low_pass(self, sample_rate_hz: f64, frequency_hz: f64) -> Result<Vec<f64>> {
        let nyquist_hz = sample_rate_hz * 0.5;
        let cutoff = frequency_hz / nyquist_hz;
        if cutoff <= 0.0 || cutoff >= 1.0 {
            return Err(EffectError::InvalidSinc);
        }

        let mut taps = self.options.taps.unwrap_or(0);
        let mut beta = self.options.beta.unwrap_or(-1.0);
        let transition = self
            .options
            .transition_width_hz
            .map_or(DEFAULT_TRANSITION_BANDWIDTH, |width| width / nyquist_hz)
            * 0.5;
        kaiser_params(
            self.options.attenuation_db,
            cutoff,
            transition,
            &mut beta,
            &mut taps,
        )?;
        if self.options.taps.is_none() {
            taps = taps.clamp(MIN_TAPS, MAX_TAPS);
            if self.options.round_taps {
                taps = rounded_tap_count(taps, cutoff)?;
            }
        }
        taps |= 1;

        make_low_pass(
            usize::try_from(taps).map_err(|_| EffectError::InvalidSinc)?,
            cutoff,
            beta,
        )
    }
}

/// Frequency range selected by a Feature 6.8.5 sinc filter.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum SincBand {
    /// `sinc -freq`, low-pass filtering up to `frequency_hz`.
    LowPass {
        /// Low-pass cutoff frequency in hertz.
        frequency_hz: f64,
        /// Whether Nyquist-or-above cutoff is treated as a null effect.
        delete_at_nyquist: bool,
    },

    /// `sinc freq`, high-pass filtering above `frequency_hz`.
    HighPass {
        /// High-pass cutoff frequency in hertz.
        frequency_hz: f64,
    },
}

impl SincBand {
    /// Creates a low-pass band. The `delete_at_nyquist` command option makes a
    /// Nyquist-or-above low-pass filter a null effect instead of an error.
    #[must_use]
    pub const fn low_pass(frequency_hz: f64, delete_at_nyquist: bool) -> Self {
        Self::LowPass {
            frequency_hz,
            delete_at_nyquist,
        }
    }

    /// Returns the cutoff frequency in hertz.
    #[must_use]
    pub const fn frequency_hz(self) -> f64 {
        match self {
            Self::LowPass { frequency_hz, .. } | Self::HighPass { frequency_hz } => frequency_hz,
        }
    }

    pub(crate) fn validate(self) -> Result<()> {
        let frequency_hz = self.frequency_hz();
        if frequency_hz.is_finite() && frequency_hz > 0.0 {
            Ok(())
        } else {
            Err(EffectError::InvalidSinc)
        }
    }
}

/// SoX-ng-style sinc FIR design options.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SincOptions {
    /// Stop-band attenuation in decibels, `40..=180`.
    pub attenuation_db: f64,

    /// Explicit Kaiser beta, mutually exclusive with attenuation.
    pub beta: Option<f64>,

    /// Explicit transition bandwidth in hertz, mutually exclusive with taps.
    pub transition_width_hz: Option<f64>,

    /// Explicit tap count, rounded up to the next odd value like SoX-ng.
    pub taps: Option<u32>,

    /// Round an automatically derived tap count to the closest integer.
    pub round_taps: bool,
}

impl Default for SincOptions {
    fn default() -> Self {
        Self {
            attenuation_db: DEFAULT_ATTENUATION_DB,
            beta: None,
            transition_width_hz: None,
            taps: None,
            round_taps: false,
        }
    }
}

impl SincOptions {
    /// Creates options with an explicit odd-or-even tap count. Even values are
    /// accepted and resolved to the next odd tap count during design.
    ///
    /// # Errors
    ///
    /// Returns [`EffectError::InvalidSinc`] when `taps` is outside SoX-ng's
    /// accepted `11..=1073741823` range.
    pub fn with_taps(taps: u32) -> Result<Self> {
        let options = Self {
            taps: Some(taps),
            ..Self::default()
        };
        options.validate()?;
        Ok(options)
    }

    pub(crate) fn validate(self) -> Result<()> {
        if !self.attenuation_db.is_finite()
            || !(MIN_ATTENUATION_DB..=MAX_ATTENUATION_DB).contains(&self.attenuation_db)
        {
            return Err(EffectError::InvalidSinc);
        }
        if let Some(beta) = self.beta
            && (!beta.is_finite() || !(MIN_BETA..=MAX_BETA).contains(&beta))
        {
            return Err(EffectError::InvalidSinc);
        }
        if let Some(width) = self.transition_width_hz
            && (!width.is_finite() || width < MIN_TRANSITION_WIDTH_HZ)
        {
            return Err(EffectError::InvalidSinc);
        }
        if let Some(taps) = self.taps
            && !(MIN_TAPS..=MAX_TAPS).contains(&taps)
        {
            return Err(EffectError::InvalidSinc);
        }
        if self.transition_width_hz.is_some() && self.taps.is_some() {
            return Err(EffectError::InvalidSinc);
        }
        Ok(())
    }
}

#[allow(
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    reason = "SoX-ng derives sinc tap counts with positive floating-point ceil and integer casts"
)]
fn kaiser_params(
    attenuation_db: f64,
    cutoff: f64,
    transition: f64,
    beta: &mut f64,
    taps: &mut u32,
) -> Result<()> {
    if transition <= 0.0 || cutoff <= 0.0 {
        return Err(EffectError::InvalidSinc);
    }
    if *beta < 0.0 {
        *beta = kaiser_beta(attenuation_db, transition * 0.5 / cutoff);
    }

    let estimate = if attenuation_db < 60.0 {
        (attenuation_db - 7.95) / (2.285 * std::f64::consts::PI * 2.0)
    } else {
        (((0.000_752_835_8 - 1.577_737e-5 * *beta) * *beta + 0.624_802_2) * *beta) + 0.061_869_02
    };

    if *taps == 0 {
        let derived = (estimate / transition + 1.0).ceil();
        if !derived.is_finite() || derived < 0.0 || derived > f64::from(MAX_TAPS) {
            return Err(EffectError::InvalidSinc);
        }
        *taps = derived as u32;
    }
    Ok(())
}

#[allow(
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    reason = "SoX-ng rounds automatically derived sinc taps through integer casts"
)]
fn rounded_tap_count(taps: u32, cutoff: f64) -> Result<u32> {
    let half = f64::from(taps / 2);
    let bucket = (half * cutoff + 0.5) as u32;
    let rounded = 1.0 + 2.0 * (f64::from(bucket) / cutoff + 0.5);
    if rounded.is_finite() && rounded <= f64::from(MAX_TAPS) {
        Ok(rounded as u32)
    } else {
        Err(EffectError::InvalidSinc)
    }
}

#[allow(
    clippy::cast_precision_loss,
    reason = "SoX-ng's sinc FIR design evaluates bounded integer tap indexes as f64"
)]
fn make_low_pass(taps: usize, cutoff: f64, beta: f64) -> Result<Vec<f64>> {
    let m = taps.checked_sub(1).ok_or(EffectError::InvalidSinc)?;
    let mult = 1.0 / bessel_i_0(beta);
    let mult1 = 1.0 / (0.5 * m as f64);
    let mut coefficients = vec![0.0; taps];

    for index in 0..=m / 2 {
        let z = index as f64 - 0.5 * m as f64;
        let x = z * std::f64::consts::PI;
        let y = z * mult1;
        let sinc = if x == 0.0 {
            cutoff
        } else {
            (cutoff * x).sin() / x
        };
        let window = bessel_i_0(beta * (1.0 - y * y).sqrt()) * mult;
        let value = sinc * window;
        coefficients[index] = value;
        coefficients[m - index] = value;
    }

    Ok(coefficients)
}

fn invert_filter(coefficients: &mut [f64]) {
    for coefficient in coefficients.iter_mut() {
        *coefficient = -*coefficient;
    }
    let center = (coefficients.len() - 1) / 2;
    coefficients[center] += 1.0;
}

#[allow(
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    reason = "SoX-ng's Kaiser beta interpolation indexes a fixed coefficient table from log2 transition width"
)]
fn kaiser_beta(att: f64, tr_bw: f64) -> f64 {
    if att >= 60.0 {
        const COEFS: [[f64; 4]; 10] = [
            [-6.784_957e-10, 1.028_56e-5, 0.108_755_6, -0.897_836_5],
            [-6.897_885e-10, 1.027_433e-5, 0.108_76, -0.897_465_8],
            [-1.000_683e-9, 1.030_092e-5, 0.108_767_7, -0.897_789_8],
            [-3.654_474e-10, 1.040_631e-5, 0.108_708_5, -0.891_776_6],
            [8.106_988e-9, 6.983_091e-6, 0.109_138_7, -0.902_204_8],
            [9.519_571e-9, 7.272_678e-6, 0.109_006_8, -0.889_076_8],
            [-5.626_821e-9, 1.342_186e-5, 0.108_399_9, -0.856_545_2],
            [-9.965_946e-8, 5.073_548e-5, 0.104_096_7, -0.682_277_8],
            [1.604_808e-7, -5.856_462e-5, 0.118_599_8, -1.248_24],
            [-1.511_964e-7, 6.363_034e-5, 0.106_462_7, -0.807_666_5],
        ];
        let realm = (tr_bw / 0.0005).log2();
        let left = realm as usize;
        let right = left.saturating_add(1);
        let c0 = COEFS[left.min(COEFS.len() - 1)];
        let c1 = COEFS[right.min(COEFS.len() - 1)];
        let b0 = ((c0[0] * att + c0[1]) * att + c0[2]) * att + c0[3];
        let b1 = ((c1[0] * att + c1[1]) * att + c1[2]) * att + c1[3];
        return b0 + (b1 - b0) * (realm - realm.trunc());
    }
    if att > 50.0 {
        return 0.1102 * (att - 8.7);
    }
    if att > 20.96 {
        return 0.58417 * (att - 20.96).powf(0.4) + 0.07886 * (att - 20.96);
    }
    0.0
}

#[allow(
    clippy::float_cmp,
    reason = "the series intentionally stops when adding the next term no longer changes the f64 sum"
)]
fn bessel_i_0(x: f64) -> f64 {
    let mut term = 1.0;
    let mut sum = 1.0;
    let x2 = x / 2.0;
    for i in 1.. {
        let previous_sum = sum;
        let y = x2 / f64::from(i);
        term *= y * y;
        sum += term;
        if sum == previous_sum {
            return sum;
        }
    }
    unreachable!("Bessel I0 series converges before integer exhaustion")
}

#[cfg(test)]
mod tests {
    use super::{Sinc, SincBand, SincOptions};
    use crate::EffectError;
    use auralis_core::SampleRate;

    #[test]
    fn explicit_taps_are_forced_odd_during_design() {
        let sinc = Sinc::with_options(
            SincBand::LowPass {
                frequency_hz: 4_000.0,
                delete_at_nyquist: false,
            },
            SincOptions::with_taps(12).unwrap(),
        )
        .unwrap();

        let coefficients = sinc
            .coefficients_for_sample_rate(SampleRate::new(48_000).unwrap())
            .unwrap();

        assert_eq!(coefficients.len(), 13);
    }

    #[test]
    fn high_pass_inverts_low_pass_response() {
        let options = SincOptions::with_taps(11).unwrap();
        let low_pass = Sinc::with_options(
            SincBand::LowPass {
                frequency_hz: 4_000.0,
                delete_at_nyquist: false,
            },
            options,
        )
        .unwrap()
        .coefficients_for_sample_rate(SampleRate::new(48_000).unwrap())
        .unwrap();
        let high_pass = Sinc::with_options(
            SincBand::HighPass {
                frequency_hz: 4_000.0,
            },
            options,
        )
        .unwrap()
        .coefficients_for_sample_rate(SampleRate::new(48_000).unwrap())
        .unwrap();
        let center = low_pass.len() / 2;

        for (index, (&low, &high)) in low_pass
            .as_slice()
            .iter()
            .zip(high_pass.as_slice())
            .enumerate()
        {
            let expected_sum = if index == center { 1.0 } else { 0.0 };
            assert!((low + high - expected_sum).abs() <= 1.0e-12);
        }
    }

    #[test]
    fn rejects_invalid_frequency_and_options() {
        assert_eq!(Sinc::low_pass(0.0).unwrap_err(), EffectError::InvalidSinc);
        assert_eq!(
            SincOptions::with_taps(10).unwrap_err(),
            EffectError::InvalidSinc
        );
    }
}

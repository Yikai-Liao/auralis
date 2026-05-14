use auralis_core::AudioBuffer;
use auralis_dsp::{DftFir, FirCoefficients as DspFirCoefficients};
use rustfft::{FftPlanner, num_complex::Complex};

use crate::{EffectError, Result};

const DEFAULT_GAIN_DB: f32 = -10.0;
const DEFAULT_REFERENCE_DB: f32 = 65.0;
const DEFAULT_HALF_POINTS: u16 = 1023;
const MIN_GAIN_DB: f32 = -50.0;
const MAX_GAIN_DB: f32 = 15.0;
const MIN_REFERENCE_DB: f32 = 50.0;
const MAX_REFERENCE_DB: f32 = 75.0;
const MIN_HALF_POINTS: u16 = 127;
const MAX_HALF_POINTS: u16 = 2047;

const ISO_226_TABLE: [(f64, f64, f64, f64); 29] = [
    (20.0, 0.532, -31.6, 78.5),
    (25.0, 0.506, -27.2, 68.7),
    (31.5, 0.480, -23.0, 59.5),
    (40.0, 0.455, -19.1, 51.1),
    (50.0, 0.432, -15.9, 44.0),
    (63.0, 0.409, -13.0, 37.5),
    (80.0, 0.387, -10.3, 31.5),
    (100.0, 0.367, -8.1, 26.5),
    (125.0, 0.349, -6.2, 22.1),
    (160.0, 0.330, -4.5, 17.9),
    (200.0, 0.315, -3.1, 14.4),
    (250.0, 0.301, -2.0, 11.4),
    (315.0, 0.288, -1.1, 8.6),
    (400.0, 0.276, -0.4, 6.2),
    (500.0, 0.267, 0.0, 4.4),
    (630.0, 0.259, 0.3, 3.0),
    (800.0, 0.253, 0.5, 2.2),
    (1000.0, 0.250, 0.0, 2.4),
    (1250.0, 0.246, -2.7, 3.5),
    (1600.0, 0.244, -4.1, 1.7),
    (2000.0, 0.243, -1.0, -1.3),
    (2500.0, 0.243, 1.7, -4.2),
    (3150.0, 0.243, 2.5, -6.0),
    (4000.0, 0.242, 1.2, -5.4),
    (5000.0, 0.242, -2.1, -1.5),
    (6300.0, 0.245, -7.1, 6.0),
    (8000.0, 0.254, -11.2, 12.6),
    (10000.0, 0.271, -10.7, 13.9),
    (12500.0, 0.301, -3.1, 12.3),
];

/// SoX-ng-style ISO 226 loudness compensation.
///
/// `loudness [gain [reference [n]]]` applies a centered FIR filter whose
/// frequency response follows SoX-ng's ISO 226 equal-loudness compensation.
/// `gain_db` is usually negative, `reference_db` is the phon reference level,
/// and `half_points` is SoX-ng's half-length setting before it is expanded to
/// an odd tap count. A zero gain is an identity operation.
///
/// Processing is whole-buffer, deterministic, and scalar. SIMD is not used
/// because this feature is a generated FIR convolution with a filter-wide
/// dependency window rather than a data-parallel per-sample transform.
///
/// # Examples
///
/// ```
/// use auralis_core::{AudioBuffer, AudioSpec, ChannelCount, FrameCount, SampleFormat, SampleRate};
/// use auralis_effects::Loudness;
///
/// let spec = AudioSpec::new(
///     SampleRate::new(48_000)?,
///     ChannelCount::new(1)?,
///     SampleFormat::Float32,
/// );
/// let mut audio = AudioBuffer::from_planar_f32(spec, FrameCount::new(3), vec![0.25, 0.0, -0.25])?;
/// Loudness::identity().process_buffer(&mut audio)?;
///
/// assert_eq!(audio.as_planar_f32(), &[0.25, 0.0, -0.25]);
/// # Ok::<(), Box<dyn std::error::Error>>(())
/// ```
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Loudness {
    /// Input gain in decibels.
    pub gain_db: f32,

    /// ISO 226 reference level in decibels SPL.
    pub reference_db: f32,

    /// SoX-ng half-length setting before conversion to an odd FIR tap count.
    pub half_points: u16,
}

impl Loudness {
    /// Creates a loudness-compensation processor.
    ///
    /// # Errors
    ///
    /// Returns [`EffectError::InvalidLoudness`] when arguments are outside
    /// SoX-ng's supported ranges: gain `-50..=15`, reference `50..=75`, and
    /// half-points `127..=2047`.
    pub fn new(gain_db: f32, reference_db: f32, half_points: u16) -> Result<Self> {
        if !gain_db.is_finite()
            || !reference_db.is_finite()
            || !(MIN_GAIN_DB..=MAX_GAIN_DB).contains(&gain_db)
            || !(MIN_REFERENCE_DB..=MAX_REFERENCE_DB).contains(&reference_db)
            || !(MIN_HALF_POINTS..=MAX_HALF_POINTS).contains(&half_points)
        {
            return Err(EffectError::InvalidLoudness);
        }

        Ok(Self {
            gain_db,
            reference_db,
            half_points,
        })
    }

    /// Creates the SoX-ng default `loudness` processor.
    #[must_use]
    pub const fn default_settings() -> Self {
        Self {
            gain_db: DEFAULT_GAIN_DB,
            reference_db: DEFAULT_REFERENCE_DB,
            half_points: DEFAULT_HALF_POINTS,
        }
    }

    /// Creates an identity loudness processor.
    #[must_use]
    pub const fn identity() -> Self {
        Self {
            gain_db: 0.0,
            reference_db: DEFAULT_REFERENCE_DB,
            half_points: DEFAULT_HALF_POINTS,
        }
    }

    /// Returns the expanded odd FIR tap count.
    #[must_use]
    pub fn tap_count(self) -> usize {
        usize::from(self.half_points) * 2 + 1
    }

    /// Applies loudness compensation to all channels in a decoded buffer.
    ///
    /// # Errors
    ///
    /// Returns [`EffectError::InvalidLoudness`] if the configured filter cannot
    /// be generated for the buffer's sample rate.
    pub fn process_buffer(self, audio: &mut AudioBuffer) -> Result<()> {
        if self.gain_db.to_bits() == 0.0_f32.to_bits() {
            return Ok(());
        }

        let sample_rate = f64::from(audio.spec().sample_rate().as_u32());
        let taps = self.filter_taps(sample_rate)?;
        let tap_count = taps.len();
        let coefficients =
            DspFirCoefficients::new(taps).map_err(|_| EffectError::InvalidLoudness)?;
        let dft = if tap_count <= 511 {
            DftFir::with_dft_len(coefficients, 2048)
        } else {
            DftFir::new(coefficients)
        };
        let frames =
            usize::try_from(audio.frames().as_u64()).map_err(|_| EffectError::InvalidLoudness)?;
        let channels = audio.channels().as_usize();
        let samples = audio.as_planar_f32_mut();

        for channel in 0..channels {
            let start = channel * frames;
            let end = start + frames;
            let source = samples[start..end].to_vec();
            dft.process_into(&source, &mut samples[start..end]);
        }

        Ok(())
    }

    fn filter_taps(self, sample_rate: f64) -> Result<Vec<f64>> {
        if !sample_rate.is_finite() || sample_rate <= 0.0 {
            return Err(EffectError::InvalidLoudness);
        }

        Ok(make_filter(
            self.tap_count(),
            f64::from(self.reference_db),
            f64::from(self.gain_db),
            sample_rate,
        ))
    }
}

impl Default for Loudness {
    fn default() -> Self {
        Self::default_settings()
    }
}

#[allow(
    clippy::cast_precision_loss,
    reason = "filter generation follows SoX-ng's floating-point frequency-domain formulas"
)]
fn make_filter(tap_count: usize, reference_db: f64, gain_db: f64, sample_rate: f64) -> Vec<f64> {
    let response = LoudnessResponse::new(reference_db, gain_db);
    let work_len = work_len_for_rate(sample_rate);
    let mut spectrum = vec![Complex::new(0.0, 0.0); work_len];

    for i in 0..=work_len / 2 {
        let frequency = sample_rate * i as f64 / work_len as f64;
        let gain = 10.0_f64.powf(response.spl_at_hz(frequency) / 20.0);
        spectrum[i].re = gain;
        if i != 0 && i != work_len / 2 {
            spectrum[work_len - i].re = gain;
        }
    }

    let mut planner = FftPlanner::<f64>::new();
    planner.plan_fft_inverse(work_len).process(&mut spectrum);

    let half = tap_count / 2;
    let scale = 1.0 / work_len as f64;
    let mut taps = (0..tap_count)
        .map(|index| {
            let source = (work_len + index - half) % work_len;
            spectrum[source].re * scale
        })
        .collect::<Vec<_>>();
    apply_kaiser(
        &mut taps,
        kaiser_beta(40.0 + 2.0 / 3.0 * gain_db.abs(), 0.1),
    );
    taps
}

#[allow(
    clippy::cast_precision_loss,
    reason = "SoX-ng chooses the FFT work length by comparing it with the floating-point sample rate"
)]
fn work_len_for_rate(sample_rate: f64) -> usize {
    let mut work_len = 8192_usize;
    while (work_len as f64) < sample_rate / 2.0 {
        work_len <<= 1;
    }
    work_len
}

struct LoudnessResponse {
    log_frequencies: Vec<f64>,
    spl_delta: Vec<f64>,
    second_derivatives: Vec<f64>,
}

impl LoudnessResponse {
    fn new(reference_db: f64, gain_db: f64) -> Self {
        let mut log_frequencies = Vec::with_capacity(ISO_226_TABLE.len() + 2);
        let mut spl_delta = Vec::with_capacity(ISO_226_TABLE.len() + 2);

        log_frequencies.push(1.0_f64.ln());
        spl_delta.push(gain_db * 0.2);
        for &(frequency, af, lu, tf) in &ISO_226_TABLE {
            log_frequencies.push(frequency.ln());
            spl_delta.push(spl(reference_db + gain_db, af, lu, tf) - spl(reference_db, af, lu, tf));
        }
        log_frequencies.push(100_000.0_f64.ln());
        spl_delta.push(gain_db * 0.2);

        let second_derivatives = prepare_natural_spline(&log_frequencies, &spl_delta);
        Self {
            log_frequencies,
            spl_delta,
            second_derivatives,
        }
    }

    fn spl_at_hz(&self, frequency_hz: f64) -> f64 {
        if frequency_hz < 1.0 {
            return self.spl_delta[0];
        }
        spline3(
            &self.log_frequencies,
            &self.spl_delta,
            &self.second_derivatives,
            frequency_hz.ln(),
        )
    }
}

fn spl(phon: f64, af: f64, lu: f64, tf: f64) -> f64 {
    10.0 / af
        * (4.47e-3 * (10.0_f64.powf(0.025 * phon) - 1.15)
            + (0.4 * 10.0_f64.powf((tf + lu) / 10.0 - 9.0)).powf(af))
        .log10()
        - lu
        + 94.0
}

fn prepare_natural_spline(abscissas: &[f64], ordinates: &[f64]) -> Vec<f64> {
    debug_assert_eq!(abscissas.len(), ordinates.len());
    let point_count = abscissas.len();
    let mut second_derivatives = vec![0.0; point_count];
    let mut work = vec![0.0; point_count - 1];

    for index in 1..point_count - 1 {
        let sig = (abscissas[index] - abscissas[index - 1])
            / (abscissas[index + 1] - abscissas[index - 1]);
        let denominator = sig.mul_add(second_derivatives[index - 1], 2.0);
        second_derivatives[index] = (sig - 1.0) / denominator;
        work[index] = (ordinates[index + 1] - ordinates[index])
            / (abscissas[index + 1] - abscissas[index])
            - (ordinates[index] - ordinates[index - 1]) / (abscissas[index] - abscissas[index - 1]);
        work[index] = (6.0 * work[index] / (abscissas[index + 1] - abscissas[index - 1])
            - sig * work[index - 1])
            / denominator;
    }

    for index in (0..point_count - 1).rev() {
        second_derivatives[index] =
            second_derivatives[index].mul_add(second_derivatives[index + 1], work[index]);
    }

    second_derivatives
}

fn spline3(x: &[f64], y: &[f64], y_2d: &[f64], value: f64) -> f64 {
    let mut low = 0;
    let mut high = x.len() - 1;
    while high - low > 1 {
        let middle = (high + low) >> 1;
        if x[middle] > value {
            high = middle;
        } else {
            low = middle;
        }
    }

    let distance = x[high] - x[low];
    let a = (x[high] - value) / distance;
    let b = (value - x[low]) / distance;
    a * y[low]
        + b * y[high]
        + ((a * a * a - a) * y_2d[low] + (b * b * b - b) * y_2d[high]) * distance * distance / 6.0
}

#[allow(
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    reason = "SoX-ng's Kaiser beta interpolation indexes a small coefficient table from log2 transition width"
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
    clippy::cast_precision_loss,
    reason = "window generation follows SoX-ng's floating-point tap-index formula"
)]
fn apply_kaiser(taps: &mut [f64], beta: f64) {
    let denominator = bessel_i_0(beta);
    let m = taps.len() - 1;
    for (index, tap) in taps.iter_mut().enumerate() {
        let x = 2.0 * index as f64 / m as f64 - 1.0;
        *tap *= bessel_i_0(beta * (1.0 - x * x).sqrt()) / denominator;
    }
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
    use super::Loudness;
    use crate::EffectError;
    use auralis_core::{
        AudioBuffer, AudioSpec, ChannelCount, FrameCount, SampleFormat, SampleRate,
    };

    #[test]
    fn identity_gain_preserves_samples() {
        let mut audio = mono_audio_buffer(vec![0.25, 0.0, -0.25]);

        Loudness::identity().process_buffer(&mut audio).unwrap();

        assert_eq!(audio.as_planar_f32(), &[0.25, 0.0, -0.25]);
    }

    #[test]
    fn default_settings_match_sox_ng() {
        let loudness = Loudness::default();

        assert_eq!(loudness.gain_db.to_bits(), (-10.0_f32).to_bits());
        assert_eq!(loudness.reference_db.to_bits(), 65.0_f32.to_bits());
        assert_eq!(loudness.half_points, 1023);
        assert_eq!(loudness.tap_count(), 2047);
    }

    #[test]
    fn generated_filter_is_symmetric() {
        let taps = Loudness::new(-6.0, 65.0, 127)
            .unwrap()
            .filter_taps(48_000.0)
            .unwrap();

        for index in 0..taps.len() / 2 {
            let opposite = taps[taps.len() - index - 1];
            assert!((taps[index] - opposite).abs() <= 0.000_000_000_001);
        }
    }

    #[test]
    fn process_buffer_preserves_shape_and_finite_samples() {
        let mut audio = stereo_audio_buffer(vec![0.5, -0.25, 0.25, -0.5]);

        Loudness::new(-6.0, 65.0, 127)
            .unwrap()
            .process_buffer(&mut audio)
            .unwrap();

        assert_eq!(audio.channels().as_usize(), 2);
        assert_eq!(audio.frames().as_u64(), 2);
        assert!(
            audio
                .as_planar_f32()
                .iter()
                .all(|sample| sample.is_finite())
        );
    }

    #[test]
    fn invalid_values_are_rejected() {
        assert_eq!(
            Loudness::new(-51.0, 65.0, 127).unwrap_err(),
            EffectError::InvalidLoudness
        );
        assert_eq!(
            Loudness::new(-10.0, 49.0, 127).unwrap_err(),
            EffectError::InvalidLoudness
        );
        assert_eq!(
            Loudness::new(-10.0, 65.0, 126).unwrap_err(),
            EffectError::InvalidLoudness
        );
    }

    fn mono_audio_buffer(samples: Vec<f32>) -> AudioBuffer {
        audio_buffer(samples, 1)
    }

    fn stereo_audio_buffer(samples: Vec<f32>) -> AudioBuffer {
        audio_buffer(samples, 2)
    }

    fn audio_buffer(samples: Vec<f32>, channels: u16) -> AudioBuffer {
        let spec = AudioSpec::new(
            SampleRate::new(48_000).unwrap(),
            ChannelCount::new(channels).unwrap(),
            SampleFormat::Float32,
        );
        AudioBuffer::from_planar_f32(
            spec,
            FrameCount::new(u64::try_from(samples.len() / usize::from(channels)).unwrap()),
            samples,
        )
        .unwrap()
    }
}

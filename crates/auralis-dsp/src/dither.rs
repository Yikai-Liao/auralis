//! Deterministic dither and noise-shaping primitives.

use thiserror::Error;

const DEFAULT_PRECISION_BITS: u8 = 16;
const MIN_PRECISION_BITS: u8 = 2;
const MAX_PRECISION_BITS: u8 = 24;
const SOX_SAMPLE_SCALE: f64 = 2_147_483_648.0;
const SOX_SAMPLE_MIN: i64 = i32::MIN as i64;
const SOX_SAMPLE_MAX: i64 = i32::MAX as i64;

/// Crate-local result type for dither primitive constructors.
pub type DitherResult<T> = std::result::Result<T, DitherError>;

/// Errors produced by reusable dither DSP primitive constructors.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Error)]
#[non_exhaustive]
pub enum DitherError {
    /// The requested target precision was outside the supported range.
    #[error("dither precision must be in the implemented SoX-ng range 2..=24 bits")]
    InvalidPrecision,
}

/// Default deterministic seed used by command-style dither processing.
///
/// This is the first SoX-ng repeatable PRNG state after seeding with zero.
pub const DEFAULT_DITHER_SEED: u32 = 1_013_904_223;

/// Base dither distribution.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum DitherMode {
    /// Plain triangular probability density dither.
    Tpdf,
    /// SoX-ng `-S` sloped triangular dither without noise shaping.
    SlopedTpdf,
}

/// Noise-shaping filter family.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum DitherNoiseShape {
    /// SoX-ng's default `dither -s` Shibata-style error-feedback curve.
    Shibata,
}

/// SoX-ng-style deterministic dither configuration for normalized samples.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Dither {
    mode: DitherMode,
    noise_shape: Option<DitherNoiseShape>,
    precision_bits: u8,
    seed: u32,
}

impl Dither {
    /// Creates default TPDF dither for 16-bit output with a deterministic seed.
    #[must_use]
    pub const fn new() -> Self {
        Self {
            mode: DitherMode::Tpdf,
            noise_shape: None,
            precision_bits: DEFAULT_PRECISION_BITS,
            seed: DEFAULT_DITHER_SEED,
        }
    }

    /// Creates sloped TPDF dither for 16-bit output with a deterministic seed.
    #[must_use]
    pub const fn sloped_tpdf() -> Self {
        Self {
            mode: DitherMode::SlopedTpdf,
            noise_shape: None,
            precision_bits: DEFAULT_PRECISION_BITS,
            seed: DEFAULT_DITHER_SEED,
        }
    }

    /// Creates default Shibata noise-shaped TPDF dither for 16-bit output.
    #[must_use]
    pub const fn shibata() -> Self {
        Self {
            mode: DitherMode::Tpdf,
            noise_shape: Some(DitherNoiseShape::Shibata),
            precision_bits: DEFAULT_PRECISION_BITS,
            seed: DEFAULT_DITHER_SEED,
        }
    }

    /// Returns this dither configuration with an explicit target precision.
    ///
    /// # Errors
    ///
    /// Returns [`DitherError::InvalidPrecision`] when `precision_bits` is
    /// outside the implemented SoX-ng-compatible range.
    pub const fn with_precision(mut self, precision_bits: u8) -> DitherResult<Self> {
        if precision_bits < MIN_PRECISION_BITS || precision_bits > MAX_PRECISION_BITS {
            return Err(DitherError::InvalidPrecision);
        }
        self.precision_bits = precision_bits;
        Ok(self)
    }

    /// Returns this dither configuration with an explicit deterministic seed.
    #[must_use]
    pub const fn with_seed(mut self, seed: u32) -> Self {
        self.seed = seed;
        self
    }

    /// Returns this dither configuration with explicit noise shaping.
    #[must_use]
    pub const fn with_noise_shape(mut self, noise_shape: DitherNoiseShape) -> Self {
        self.noise_shape = Some(noise_shape);
        self.mode = DitherMode::Tpdf;
        self
    }

    /// Returns the configured dither mode.
    #[must_use]
    pub const fn mode(self) -> DitherMode {
        self.mode
    }

    /// Returns the configured noise-shaping filter, if any.
    #[must_use]
    pub const fn noise_shape(self) -> Option<DitherNoiseShape> {
        self.noise_shape
    }

    /// Returns the target precision in bits.
    #[must_use]
    pub const fn precision_bits(self) -> u8 {
        self.precision_bits
    }

    /// Returns the deterministic PRNG seed.
    #[must_use]
    pub const fn seed(self) -> u32 {
        self.seed
    }

    /// Applies dither to a planar sample slice in place.
    pub fn process_samples(self, samples: &mut [f32]) {
        let mut state = DitherState::new(self);
        state.process_samples(samples);
    }
}

impl Default for Dither {
    fn default() -> Self {
        Self::new()
    }
}

/// Stateful deterministic dither processor for chunked callers.
#[derive(Debug, Clone)]
pub struct DitherState {
    dither: Dither,
    random: u32,
    previous_random: i32,
    previous_errors: [f64; SHIBATA_48KHZ.len()],
    shape_position: usize,
}

impl DitherState {
    /// Creates a stateful dither processor from a validated configuration.
    #[must_use]
    pub const fn new(dither: Dither) -> Self {
        Self {
            random: dither.seed,
            dither,
            previous_random: 0,
            previous_errors: [0.0; SHIBATA_48KHZ.len()],
            shape_position: 0,
        }
    }

    /// Applies dither to the next chunk of samples.
    pub fn process_samples(&mut self, samples: &mut [f32]) {
        if self.dither.noise_shape.is_some() {
            for sample in samples {
                *sample = self.process_noise_shaped_sample(*sample);
            }
        } else {
            self.process_unshaped_samples(samples);
        }
    }

    fn process_unshaped_samples(&mut self, samples: &mut [f32]) {
        let params = self.unshaped_params();
        for sample in samples {
            *sample = self.process_unshaped_sample(*sample, params);
        }
    }

    fn unshaped_params(&self) -> UnshapedDitherParams {
        let precision = self.dither.precision_bits;
        UnshapedDitherParams {
            precision,
            denominator: f64::from(1_u32 << u32::from(32 - precision)),
            minimum: -(1_i64 << u32::from(precision - 1)),
            maximum: (1_i64 << u32::from(precision - 1)) - 1,
            output_shift: u32::from(32 - precision),
        }
    }

    #[allow(
        clippy::cast_possible_truncation,
        clippy::cast_precision_loss,
        reason = "SoX-ng-compatible quantization maps between integer sample units and normalized f32"
    )]
    fn process_unshaped_sample(&mut self, sample: f32, params: UnshapedDitherParams) -> f32 {
        let random = self.next_random() >> u32::from(params.precision);
        let second = match self.dither.mode {
            DitherMode::Tpdf => self.next_random() >> u32::from(params.precision),
            DitherMode::SlopedTpdf => -self.previous_random,
        };
        self.previous_random = random;

        let internal = normalized_to_sox_sample(sample);
        let scaled =
            (internal as f64 + f64::from(random) + f64::from(second)) / params.denominator;
        let quantized = round_half_away_from_zero(scaled);
        let clamped = quantized.clamp(params.minimum, params.maximum);
        let output = clamped << params.output_shift;

        (output as f64 / SOX_SAMPLE_SCALE) as f32
    }

    #[allow(
        clippy::cast_possible_truncation,
        clippy::cast_precision_loss,
        reason = "SoX-ng-compatible noise-shaped quantization maps between integer sample units and normalized f32"
    )]
    fn process_noise_shaped_sample(&mut self, sample: f32) -> f32 {
        let precision = self.dither.precision_bits;
        let random = f64::from(self.next_random() >> u32::from(precision))
            + f64::from(self.next_random() >> u32::from(precision));
        let denominator = f64::from(1_u32 << u32::from(32 - precision));
        let shaped = self.shaped_internal_sample(sample);
        let scaled = (shaped + random) / denominator;
        let quantized = round_half_away_from_zero(scaled);
        let minimum = -(1_i64 << u32::from(precision - 1));
        let maximum = (1_i64 << u32::from(precision - 1)) - 1;
        let clamped = quantized.clamp(minimum, maximum);

        self.shape_position = self
            .shape_position
            .checked_sub(1)
            .unwrap_or(SHIBATA_48KHZ.len() - 1);
        self.previous_errors[self.shape_position] = (clamped as f64 * denominator) - shaped;

        let output = clamped << u32::from(32 - precision);
        (output as f64 / SOX_SAMPLE_SCALE) as f32
    }

    #[allow(
        clippy::cast_precision_loss,
        reason = "noise-shaping feedback works in SoX integer sample units before f64 coefficient convolution"
    )]
    fn shaped_internal_sample(&self, sample: f32) -> f64 {
        let mut shaped = normalized_to_sox_sample(sample) as f64;
        for (offset, coefficient) in SHIBATA_48KHZ.iter().enumerate() {
            let index = (self.shape_position + offset) % SHIBATA_48KHZ.len();
            shaped -= coefficient * self.previous_errors[index];
        }
        shaped
    }

    fn next_random(&mut self) -> i32 {
        self.random = self
            .random
            .wrapping_mul(1_664_525)
            .wrapping_add(1_013_904_223);
        i32::from_ne_bytes(self.random.to_ne_bytes())
    }
}

#[derive(Debug, Clone, Copy)]
struct UnshapedDitherParams {
    precision: u8,
    denominator: f64,
    minimum: i64,
    maximum: i64,
    output_shift: u32,
}

const SHIBATA_48KHZ: [f64; 16] = [
    2.872_072_935_104_37,
    -5.041_323_184_967_041,
    6.244_299_411_773_682,
    -5.848_398_685_455_322,
    3.706_754_207_611_084,
    -1.049_511_909_484_863,
    -1.183_023_691_177_368,
    2.112_679_243_087_769,
    -1.909_453_153_610_229,
    0.999_130_845_069_885,
    -0.170_908_063_650_131,
    -0.326_156_020_164_49,
    0.391_276_448_965_073,
    -0.268_764_615_058_899,
    0.097_676_105_797_29,
    -0.023_473_845_794_797,
];

#[allow(
    clippy::cast_possible_truncation,
    reason = "SoX-ng-compatible quantization intentionally rounds finite normalized samples into i32 sample units"
)]
fn normalized_to_sox_sample(sample: f32) -> i64 {
    if !sample.is_finite() {
        return 0;
    }

    let scaled = f64::from(sample).clamp(-1.0, 1.0) * SOX_SAMPLE_SCALE;
    round_half_away_from_zero(scaled).clamp(SOX_SAMPLE_MIN, SOX_SAMPLE_MAX)
}

#[allow(
    clippy::cast_possible_truncation,
    reason = "rounding is bounded by callers before conversion to integer sample units"
)]
fn round_half_away_from_zero(value: f64) -> i64 {
    if value < 0.0 {
        (value - 0.5) as i64
    } else {
        (value + 0.5) as i64
    }
}

#[cfg(test)]
mod tests {
    use super::{Dither, DitherError, DitherMode, DitherNoiseShape, DitherState};

    #[test]
    fn rejects_invalid_precision() {
        assert_eq!(
            Dither::new().with_precision(1).unwrap_err(),
            DitherError::InvalidPrecision
        );
        assert_eq!(
            Dither::new().with_precision(25).unwrap_err(),
            DitherError::InvalidPrecision
        );
        assert_eq!(Dither::new().with_precision(8).unwrap().precision_bits(), 8);
    }

    #[test]
    fn tpdf_dither_is_deterministic_and_quantized() {
        let mut first = [0.0, 0.0, 0.0, 0.0];
        let mut second = first;
        let dither = Dither::new().with_precision(8).unwrap().with_seed(0);

        dither.process_samples(&mut first);
        dither.process_samples(&mut second);

        assert_sample_bits_eq(&first, &second);
        assert!(
            first
                .iter()
                .any(|sample| sample.to_bits() != 0.0_f32.to_bits())
        );
        assert!(first.iter().all(|sample| is_quantized_to_8_bits(*sample)));
    }

    #[test]
    fn sloped_tpdf_uses_previous_random_state() {
        let mut plain = [0.0; 8];
        let mut sloped = plain;

        Dither::new()
            .with_precision(8)
            .unwrap()
            .with_seed(0)
            .process_samples(&mut plain);
        Dither::sloped_tpdf()
            .with_precision(8)
            .unwrap()
            .with_seed(0)
            .process_samples(&mut sloped);

        assert_ne!(
            plain.map(f32::to_bits),
            sloped.map(f32::to_bits),
            "sloped TPDF should produce a different deterministic sequence"
        );
        assert!(sloped.iter().all(|sample| is_quantized_to_8_bits(*sample)));
    }

    #[test]
    fn chunked_processing_matches_whole_slice_processing() {
        let dither = Dither::shibata().with_precision(8).unwrap().with_seed(123);
        let mut whole = [0.0, 0.001, -0.001, 0.25, -0.25, 0.0];
        let mut chunked = whole;
        let mut state = DitherState::new(dither);

        dither.process_samples(&mut whole);
        state.process_samples(&mut chunked[..2]);
        state.process_samples(&mut chunked[2..5]);
        state.process_samples(&mut chunked[5..]);

        assert_sample_bits_eq(&chunked, &whole);
    }

    #[test]
    fn exposes_mode_and_seed_policy() {
        let dither = Dither::sloped_tpdf()
            .with_noise_shape(DitherNoiseShape::Shibata)
            .with_seed(42);

        assert_eq!(dither.mode(), DitherMode::Tpdf);
        assert_eq!(dither.noise_shape(), Some(DitherNoiseShape::Shibata));
        assert_eq!(dither.seed(), 42);
    }

    #[test]
    fn shibata_noise_shaping_feeds_back_quantization_error() {
        let mut plain = [0.0; 8];
        let mut shaped = plain;

        Dither::new()
            .with_precision(8)
            .unwrap()
            .with_seed(0)
            .process_samples(&mut plain);
        Dither::shibata()
            .with_precision(8)
            .unwrap()
            .with_seed(0)
            .process_samples(&mut shaped);

        assert_ne!(plain.map(f32::to_bits), shaped.map(f32::to_bits));
        assert!(shaped.iter().all(|sample| is_quantized_to_8_bits(*sample)));
    }

    fn is_quantized_to_8_bits(sample: f32) -> bool {
        let scaled = sample * 128.0;
        (scaled - scaled.round()).abs() <= f32::EPSILON
    }

    fn assert_sample_bits_eq(actual: &[f32], expected: &[f32]) {
        assert_eq!(actual.len(), expected.len());
        for (index, (&actual, &expected)) in actual.iter().zip(expected).enumerate() {
            assert_eq!(
                actual.to_bits(),
                expected.to_bits(),
                "sample {index} differed: {actual} != {expected}"
            );
        }
    }
}

//! Deterministic TPDF dither primitives.

use auralis_core::AudioBuffer;

use crate::{EffectError, Result};

const DEFAULT_PRECISION_BITS: u8 = 16;
const MIN_PRECISION_BITS: u8 = 2;
const MAX_PRECISION_BITS: u8 = 24;
const SOX_SAMPLE_SCALE: f64 = 2_147_483_648.0;
const SOX_SAMPLE_MIN: i64 = i32::MIN as i64;
const SOX_SAMPLE_MAX: i64 = i32::MAX as i64;

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

/// SoX-ng-style deterministic dither configuration.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Dither {
    mode: DitherMode,
    precision_bits: u8,
    seed: u32,
}

impl Dither {
    /// Creates default TPDF dither for 16-bit output with a deterministic seed.
    #[must_use]
    pub const fn new() -> Self {
        Self {
            mode: DitherMode::Tpdf,
            precision_bits: DEFAULT_PRECISION_BITS,
            seed: DEFAULT_DITHER_SEED,
        }
    }

    /// Creates sloped TPDF dither for 16-bit output with a deterministic seed.
    #[must_use]
    pub const fn sloped_tpdf() -> Self {
        Self {
            mode: DitherMode::SlopedTpdf,
            precision_bits: DEFAULT_PRECISION_BITS,
            seed: DEFAULT_DITHER_SEED,
        }
    }

    /// Returns this dither configuration with an explicit target precision.
    ///
    /// # Errors
    ///
    /// Returns [`EffectError::InvalidDither`] when `precision_bits` is outside
    /// the implemented SoX-ng-compatible range.
    pub const fn with_precision(mut self, precision_bits: u8) -> Result<Self> {
        if precision_bits < MIN_PRECISION_BITS || precision_bits > MAX_PRECISION_BITS {
            return Err(EffectError::InvalidDither);
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

    /// Returns the configured dither mode.
    #[must_use]
    pub const fn mode(self) -> DitherMode {
        self.mode
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

    /// Applies dither to a decoded planar audio buffer in place.
    pub fn process_buffer(self, audio: &mut AudioBuffer) {
        self.process_samples(audio.as_planar_f32_mut());
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
}

impl DitherState {
    /// Creates a stateful dither processor from a validated configuration.
    #[must_use]
    pub const fn new(dither: Dither) -> Self {
        Self {
            random: dither.seed,
            dither,
            previous_random: 0,
        }
    }

    /// Applies dither to the next chunk of samples.
    pub fn process_samples(&mut self, samples: &mut [f32]) {
        for sample in samples {
            *sample = self.process_sample(*sample);
        }
    }

    #[allow(
        clippy::cast_possible_truncation,
        clippy::cast_precision_loss,
        reason = "SoX-ng-compatible quantization maps between integer sample units and normalized f32"
    )]
    fn process_sample(&mut self, sample: f32) -> f32 {
        let precision = self.dither.precision_bits;
        let random = self.next_random() >> u32::from(precision);
        let second = match self.dither.mode {
            DitherMode::Tpdf => self.next_random() >> u32::from(precision),
            DitherMode::SlopedTpdf => -self.previous_random,
        };
        self.previous_random = random;

        let denominator = f64::from(1_u32 << u32::from(32 - precision));
        let internal = normalized_to_sox_sample(sample);
        let scaled = (internal as f64 + f64::from(random) + f64::from(second)) / denominator;
        let quantized = round_half_away_from_zero(scaled);
        let minimum = -(1_i64 << u32::from(precision - 1));
        let maximum = (1_i64 << u32::from(precision - 1)) - 1;
        let clamped = quantized.clamp(minimum, maximum);
        let output = clamped << u32::from(32 - precision);

        (output as f64 / SOX_SAMPLE_SCALE) as f32
    }

    fn next_random(&mut self) -> i32 {
        self.random = self
            .random
            .wrapping_mul(1_664_525)
            .wrapping_add(1_013_904_223);
        i32::from_ne_bytes(self.random.to_ne_bytes())
    }
}

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
    use super::{Dither, DitherMode, DitherState};

    #[test]
    fn rejects_invalid_precision() {
        assert!(Dither::new().with_precision(1).is_err());
        assert!(Dither::new().with_precision(25).is_err());
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
        let dither = Dither::sloped_tpdf()
            .with_precision(8)
            .unwrap()
            .with_seed(123);
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
        let dither = Dither::sloped_tpdf().with_seed(42);

        assert_eq!(dither.mode(), DitherMode::SlopedTpdf);
        assert_eq!(dither.seed(), 42);
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

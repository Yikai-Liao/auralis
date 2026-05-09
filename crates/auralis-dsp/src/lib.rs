//! DSP kernels for Auralis.
//!
//! The kernels in this crate operate on planar `f32` sample slices and avoid
//! format-specific assumptions. They are deterministic scalar references for
//! later effect processors and optimized backends.
//!
//! # Examples
//!
//! ```
//! use auralis_core::Decibels;
//! use auralis_dsp::gain_in_place;
//!
//! let mut samples = [0.25, -0.5, 1.0];
//! gain_in_place(&mut samples, Decibels::new(6.0)?);
//!
//! assert!(samples[0] > 0.49 && samples[0] < 0.51);
//! # Ok::<(), auralis_core::AuralisError>(())
//! ```

use auralis_core::Decibels;

/// Applies constant gain to each sample in place.
///
/// `db` is measured in decibels, where `0 dB` is identity and the linear
/// multiplier is `10^(db / 20)`. The kernel does not clip or normalize; full
/// scale inputs may exceed `[-1.0, 1.0]` after positive gain, and very large
/// gains may produce signed infinity. Finite input samples never become `NaN`.
///
/// The operation is deterministic and streaming-safe because each sample is
/// transformed independently. It allocates no memory and runs in `O(n)` time.
#[inline]
pub fn gain_in_place(samples: &mut [f32], db: Decibels) {
    let multiplier = linear_gain(db);

    if multiplier.is_infinite() {
        for sample in samples {
            if *sample != 0.0 {
                *sample = sample.signum() * f32::INFINITY;
            }
        }
        return;
    }

    for sample in samples {
        *sample *= multiplier;
    }
}

/// Returns the linear amplitude multiplier for a decibel value.
///
/// This helper is exposed so effect processors and tests can use the same
/// reference conversion as [`gain_in_place`].
#[must_use]
#[inline]
#[allow(
    clippy::cast_possible_truncation,
    reason = "the public DSP sample format is f32, so the f64 unit value is intentionally rounded once"
)]
pub fn linear_gain(db: Decibels) -> f32 {
    let multiplier = 10.0_f64.powf(db.as_f64() / 20.0);

    if multiplier > f64::from(f32::MAX) {
        f32::INFINITY
    } else {
        multiplier as f32
    }
}

#[cfg(test)]
mod tests {
    use super::{gain_in_place, linear_gain};
    use auralis_core::Decibels;

    #[test]
    fn zero_db_is_identity() {
        let mut samples = [-1.0, -0.25, 0.0, 0.25, 1.0];

        gain_in_place(&mut samples, db(0.0));

        assert_samples_close(&samples, &[-1.0, -0.25, 0.0, 0.25, 1.0]);
    }

    #[test]
    fn minus_six_db_matches_reference_multiplier() {
        let mut samples = [-1.0, -0.5, 0.25, 1.0];
        let expected_multiplier = 10.0_f32.powf(-6.0 / 20.0);

        gain_in_place(&mut samples, db(-6.0));

        assert_samples_close(
            &samples,
            &[
                -expected_multiplier,
                -0.5 * expected_multiplier,
                0.25 * expected_multiplier,
                expected_multiplier,
            ],
        );
    }

    #[test]
    fn plus_six_db_matches_reference_multiplier() {
        let mut samples = [-1.0, -0.5, 0.25, 1.0];
        let expected_multiplier = 10.0_f32.powf(6.0 / 20.0);

        gain_in_place(&mut samples, db(6.0));

        assert_samples_close(
            &samples,
            &[
                -expected_multiplier,
                -0.5 * expected_multiplier,
                0.25 * expected_multiplier,
                expected_multiplier,
            ],
        );
    }

    #[test]
    fn empty_slice_is_accepted() {
        let mut samples = [];

        gain_in_place(&mut samples, db(-12.0));

        assert!(samples.is_empty());
    }

    #[test]
    fn one_sample_is_scaled() {
        let mut samples = [0.5];

        gain_in_place(&mut samples, db(6.0));

        assert_close(samples[0], 0.5 * 10.0_f32.powf(6.0 / 20.0));
    }

    #[test]
    fn odd_lengths_are_scaled() {
        let mut samples = [-0.75, -0.25, 0.0, 0.25, 0.75];
        let expected_multiplier = 10.0_f32.powf(-6.0 / 20.0);

        gain_in_place(&mut samples, db(-6.0));

        assert_samples_close(
            &samples,
            &[
                -0.75 * expected_multiplier,
                -0.25 * expected_multiplier,
                0.0,
                0.25 * expected_multiplier,
                0.75 * expected_multiplier,
            ],
        );
    }

    #[test]
    fn full_scale_input_is_not_clipped() {
        let mut samples = [-1.0, 1.0];
        let expected_multiplier = 10.0_f32.powf(6.0 / 20.0);

        gain_in_place(&mut samples, db(6.0));

        assert_samples_close(&samples, &[-expected_multiplier, expected_multiplier]);
        assert!(samples[1] > 1.0);
    }

    #[test]
    fn finite_inputs_do_not_produce_nan() {
        let mut samples = [-1.0, -0.0, 0.0, 1.0, f32::MAX];

        gain_in_place(&mut samples, db(10_000.0));

        assert!(samples.iter().all(|sample| !sample.is_nan()));
        assert_eq!(samples[1].to_bits(), (-0.0_f32).to_bits());
        assert_eq!(samples[2].to_bits(), 0.0_f32.to_bits());
    }

    #[test]
    fn linear_gain_matches_decibel_formula() {
        assert_close(linear_gain(db(-6.0)), 10.0_f32.powf(-6.0 / 20.0));
        assert_close(linear_gain(db(6.0)), 10.0_f32.powf(6.0 / 20.0));
    }

    fn db(value: f64) -> Decibels {
        Decibels::new(value).unwrap()
    }

    fn assert_samples_close(actual: &[f32], expected: &[f32]) {
        assert_eq!(actual.len(), expected.len());

        for (actual, expected) in actual.iter().zip(expected) {
            assert_close(*actual, *expected);
        }
    }

    fn assert_close(actual: f32, expected: f32) {
        let tolerance = 1.0e-6;
        let difference = (actual - expected).abs();

        assert!(
            difference <= tolerance,
            "expected {actual} to be within {tolerance} of {expected}, difference was {difference}"
        );
    }
}

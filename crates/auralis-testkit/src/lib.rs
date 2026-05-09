//! Test utilities for Auralis.
//!
//! The metrics in this crate are small, deterministic helpers for comparing
//! decoded sample streams in Rust integration tests and golden tests. Error
//! metrics accept unequal slice lengths by treating missing samples as silence;
//! callers that need to fail directly on length changes should assert lengths
//! separately.

pub mod golden;

/// Returns the largest absolute sample error between `reference` and `actual`.
///
/// Inputs are linear full-scale samples. If the slices have different lengths,
/// missing samples on either side are compared as `0.0`, so a truncated tail is
/// still counted as error. Empty inputs return `0.0`.
///
/// NaN behavior is explicit: if any compared sample is NaN, the returned value
/// is NaN. Infinite samples follow normal IEEE arithmetic.
///
/// # Examples
///
/// ```
/// let reference = [0.0, 0.5, -0.5];
/// let actual = [0.0, 0.25, -0.75];
///
/// assert_eq!(auralis_testkit::max_abs_error(&reference, &actual), 0.25);
/// ```
#[must_use]
pub fn max_abs_error(reference: &[f32], actual: &[f32]) -> f64 {
    let mut max = 0.0_f64;

    for (reference_sample, actual_sample) in paired_samples(reference, actual) {
        let error = f64::from(reference_sample) - f64::from(actual_sample);
        if error.is_nan() {
            return f64::NAN;
        }
        max = max.max(error.abs());
    }

    max
}

/// Returns root-mean-square sample error between `reference` and `actual`.
///
/// Inputs are linear full-scale samples. If the slices have different lengths,
/// missing samples on either side are compared as `0.0`. Empty inputs return
/// `0.0`.
///
/// If any compared sample is NaN, the returned value is NaN. Infinite samples
/// follow normal IEEE arithmetic.
///
/// # Examples
///
/// ```
/// let reference = [1.0, -1.0];
/// let actual = [0.0, 0.0];
///
/// assert_eq!(auralis_testkit::rms_error(&reference, &actual), 1.0);
/// ```
#[must_use]
pub fn rms_error(reference: &[f32], actual: &[f32]) -> f64 {
    let len = reference.len().max(actual.len());
    if len == 0 {
        return 0.0;
    }

    let mut sum_squares = 0.0_f64;
    for (reference_sample, actual_sample) in paired_samples(reference, actual) {
        let error = f64::from(reference_sample) - f64::from(actual_sample);
        if error.is_nan() {
            return f64::NAN;
        }
        sum_squares += error * error;
    }

    #[expect(
        clippy::cast_precision_loss,
        reason = "slice lengths are small test metrics and f64 division is the desired domain"
    )]
    (sum_squares / len as f64).sqrt()
}

/// Returns signal-to-noise ratio in decibels.
///
/// SNR is computed as `10 * log10(reference_power / error_power)` over linear
/// full-scale samples. If the slices have different lengths, missing samples on
/// either side are compared as `0.0`.
///
/// Zero-reference behavior is documented for deterministic assertions:
///
/// - exact silence compared with exact silence returns positive infinity.
/// - a zero-power reference with non-zero error returns negative infinity.
/// - non-zero reference with zero error returns positive infinity.
///
/// If any compared sample is NaN, the returned value is NaN. Infinite samples
/// follow normal IEEE arithmetic.
///
/// # Examples
///
/// ```
/// let reference = [1.0, -1.0];
/// let actual = [0.5, -0.5];
///
/// assert_eq!(auralis_testkit::snr_db(&reference, &actual), 6.020_599_913_279_624);
/// ```
#[must_use]
pub fn snr_db(reference: &[f32], actual: &[f32]) -> f64 {
    let mut reference_power = 0.0_f64;
    let mut error_power = 0.0_f64;

    for (reference_sample, actual_sample) in paired_samples(reference, actual) {
        let reference_sample = f64::from(reference_sample);
        let actual_sample = f64::from(actual_sample);
        let error = reference_sample - actual_sample;
        if reference_sample.is_nan() || actual_sample.is_nan() || error.is_nan() {
            return f64::NAN;
        }
        reference_power += reference_sample * reference_sample;
        error_power += error * error;
    }

    match (reference_power == 0.0, error_power == 0.0) {
        (_, true) => f64::INFINITY,
        (true, false) => f64::NEG_INFINITY,
        (false, false) => 10.0 * (reference_power / error_power).log10(),
    }
}

/// Returns the largest absolute sample value.
///
/// Inputs are linear full-scale samples. Empty inputs return `0.0`. If any
/// sample is NaN, the returned value is NaN. Infinite samples follow normal IEEE
/// arithmetic.
///
/// # Examples
///
/// ```
/// let samples = [-0.25, 0.75, -0.5];
///
/// assert_eq!(auralis_testkit::peak(&samples), 0.75);
/// ```
#[must_use]
pub fn peak(samples: &[f32]) -> f64 {
    let mut peak = 0.0_f64;

    for &sample in samples {
        let sample = f64::from(sample);
        if sample.is_nan() {
            return f64::NAN;
        }
        peak = peak.max(sample.abs());
    }

    peak
}

/// Returns the arithmetic mean sample value.
///
/// Inputs are linear full-scale samples. Empty inputs return `0.0`, matching
/// silence. If any sample is NaN, the returned value is NaN. Infinite samples
/// follow normal IEEE arithmetic.
///
/// # Examples
///
/// ```
/// let samples = [-0.5, 0.25, 0.75];
///
/// assert_eq!(auralis_testkit::dc_offset(&samples), 1.0 / 6.0);
/// ```
#[must_use]
pub fn dc_offset(samples: &[f32]) -> f64 {
    if samples.is_empty() {
        return 0.0;
    }

    let mut sum = 0.0_f64;
    for &sample in samples {
        let sample = f64::from(sample);
        if sample.is_nan() {
            return f64::NAN;
        }
        sum += sample;
    }

    #[expect(
        clippy::cast_precision_loss,
        reason = "slice lengths are small test metrics and f64 division is the desired domain"
    )]
    {
        sum / samples.len() as f64
    }
}

fn paired_samples<'a>(
    reference: &'a [f32],
    actual: &'a [f32],
) -> impl Iterator<Item = (f32, f32)> + 'a {
    let len = reference.len().max(actual.len());
    (0..len).map(|index| {
        (
            reference.get(index).copied().unwrap_or(0.0),
            actual.get(index).copied().unwrap_or(0.0),
        )
    })
}

#[cfg(test)]
mod tests {
    use super::{dc_offset, max_abs_error, peak, rms_error, snr_db};

    fn assert_metric_eq(actual: f64, expected: f64) {
        assert!(
            (actual - expected).abs() <= f64::EPSILON,
            "expected {expected}, got {actual}"
        );
    }

    #[test]
    fn crate_is_linkable() {
        assert_eq!(env!("CARGO_PKG_NAME"), "auralis-testkit");
    }

    #[test]
    fn max_abs_error_uses_known_vector() {
        let reference = [0.0, 0.5, -0.5, 1.0];
        let actual = [0.0, 0.25, -0.75, 0.25];

        assert_metric_eq(max_abs_error(&reference, &actual), 0.75);
    }

    #[test]
    fn max_abs_error_counts_unequal_lengths_against_silence() {
        assert_metric_eq(max_abs_error(&[0.25, -0.75], &[0.25]), 0.75);
        assert_metric_eq(max_abs_error(&[0.25], &[0.25, -0.5]), 0.5);
    }

    #[test]
    fn rms_error_uses_known_vector() {
        let reference = [1.0, -1.0, 0.0, 0.0];
        let actual = [0.0, 0.0, 0.0, 0.0];

        assert_metric_eq(
            rms_error(&reference, &actual),
            std::f64::consts::FRAC_1_SQRT_2,
        );
    }

    #[test]
    fn snr_db_uses_known_vector() {
        let reference = [1.0, -1.0];
        let actual = [0.5, -0.5];

        assert_metric_eq(snr_db(&reference, &actual), 6.020_599_913_279_624);
    }

    #[test]
    fn snr_db_documents_zero_reference_behavior() {
        assert!(snr_db(&[], &[]).is_infinite() && snr_db(&[], &[]).is_sign_positive());
        assert!(
            snr_db(&[0.0, 0.0], &[0.0, 0.0]).is_infinite()
                && snr_db(&[0.0, 0.0], &[0.0, 0.0]).is_sign_positive()
        );
        assert!(
            snr_db(&[0.0, 0.0], &[0.5, 0.0]).is_infinite()
                && snr_db(&[0.0, 0.0], &[0.5, 0.0]).is_sign_negative()
        );
        assert!(
            snr_db(&[0.25], &[0.25]).is_infinite() && snr_db(&[0.25], &[0.25]).is_sign_positive()
        );
    }

    #[test]
    fn peak_uses_known_vector() {
        assert_metric_eq(peak(&[-0.25, 0.75, -0.5]), 0.75);
    }

    #[test]
    fn dc_offset_uses_known_vector() {
        assert_metric_eq(dc_offset(&[-0.5, 0.25, 0.75]), 1.0 / 6.0);
    }

    #[test]
    fn empty_inputs_match_silence() {
        assert_metric_eq(max_abs_error(&[], &[]), 0.0);
        assert_metric_eq(rms_error(&[], &[]), 0.0);
        assert_metric_eq(peak(&[]), 0.0);
        assert_metric_eq(dc_offset(&[]), 0.0);
    }

    #[test]
    fn nan_inputs_propagate_to_metrics() {
        assert!(max_abs_error(&[f32::NAN], &[0.0]).is_nan());
        assert!(rms_error(&[0.0], &[f32::NAN]).is_nan());
        assert!(snr_db(&[f32::NAN], &[0.0]).is_nan());
        assert!(peak(&[0.0, f32::NAN]).is_nan());
        assert!(dc_offset(&[0.0, f32::NAN]).is_nan());
    }
}

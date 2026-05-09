//! Backend differential-test helpers.
//!
//! These helpers let kernel tests run the same case under forced scalar and
//! requested SIMD backend selection, then compare the outputs with exact or
//! tolerance-based checks. The comparison report keeps backend names, the case
//! identifier, the first failing index, and numeric error metrics together so
//! failure messages stay deterministic.
//!
//! # Examples
//!
//! ```
//! use auralis_testkit::backend_conformance::{
//!     Tolerance, run_scalar_and_simd,
//! };
//!
//! let runs = run_scalar_and_simd("identity", |_selection| vec![0.0_f32, 0.5, -0.5]);
//! let comparison = runs.compare_with_tolerance(Tolerance::new(0.0, 0.0, 90.0));
//!
//! comparison.require_passed()?;
//! # Ok::<(), auralis_testkit::backend_conformance::Failure>(())
//! ```

use std::{error::Error, fmt};

use auralis_simd::{BackendKind, BackendSelection, select_backend};

/// Output from one backend run for a single conformance case.
#[derive(Debug, Clone, PartialEq)]
pub struct Run<T> {
    case_id: String,
    selection: BackendSelection,
    output: Vec<T>,
}

impl<T> Run<T> {
    /// Creates a backend run record.
    #[must_use]
    pub fn new(case_id: &str, selection: BackendSelection, output: Vec<T>) -> Self {
        Self {
            case_id: case_id.to_owned(),
            selection,
            output,
        }
    }

    /// Returns the stable conformance case identifier.
    #[must_use]
    pub fn case_id(&self) -> &str {
        &self.case_id
    }

    /// Returns the backend selection used for this run.
    #[must_use]
    pub const fn selection(&self) -> BackendSelection {
        self.selection
    }

    /// Returns the backend kind requested by this run.
    #[must_use]
    pub const fn requested_kind(&self) -> BackendKind {
        self.selection.requested_kind()
    }

    /// Returns the selected backend name.
    #[must_use]
    pub const fn selected_name(&self) -> &'static str {
        self.selection.selected_name()
    }

    /// Returns the output samples produced by the run.
    #[must_use]
    pub fn output(&self) -> &[T] {
        &self.output
    }

    /// Consumes the run and returns the owned output samples.
    #[must_use]
    pub fn into_output(self) -> Vec<T> {
        self.output
    }
}

/// Output from running one case under forced scalar and requested SIMD.
#[derive(Debug, Clone, PartialEq)]
pub struct RunPair<T> {
    case_id: String,
    scalar: Run<T>,
    simd: Run<T>,
}

impl<T> RunPair<T> {
    /// Creates paired scalar and SIMD run records.
    #[must_use]
    pub fn new(case_id: &str, scalar: Run<T>, simd: Run<T>) -> Self {
        Self {
            case_id: case_id.to_owned(),
            scalar,
            simd,
        }
    }

    /// Returns the stable conformance case identifier.
    #[must_use]
    pub fn case_id(&self) -> &str {
        &self.case_id
    }

    /// Returns the forced scalar run.
    #[must_use]
    pub const fn scalar(&self) -> &Run<T> {
        &self.scalar
    }

    /// Returns the requested SIMD run.
    ///
    /// The run records a SIMD request even when the active build falls back to
    /// scalar because the `simd` feature is disabled or the target is unsupported.
    #[must_use]
    pub const fn simd(&self) -> &Run<T> {
        &self.simd
    }
}

impl<T> RunPair<T>
where
    T: Copy + Into<f64>,
{
    /// Compares scalar and SIMD outputs with exact value and length matching.
    #[must_use]
    pub fn compare_exact(&self) -> Comparison {
        compare_runs_exact(&self.scalar, &self.simd)
    }

    /// Compares scalar and SIMD outputs with metric thresholds.
    #[must_use]
    pub fn compare_with_tolerance(&self, tolerance: Tolerance) -> Comparison {
        compare_runs_with_tolerance(&self.scalar, &self.simd, tolerance)
    }
}

/// Runs one case with forced scalar and requested SIMD backend selection.
///
/// The closure receives the deterministic [`BackendSelection`] for each run.
/// A SIMD request may still select scalar with a fallback reason when SIMD is
/// unavailable in the active build.
#[must_use]
pub fn run_scalar_and_simd<T, F>(case_id: &str, mut run: F) -> RunPair<T>
where
    F: FnMut(BackendSelection) -> Vec<T>,
{
    let scalar_selection = select_backend(BackendKind::Scalar);
    let scalar_output = run(scalar_selection);
    let simd_selection = select_backend(BackendKind::Simd);
    let simd_output = run(simd_selection);

    RunPair::new(
        case_id,
        Run::new(case_id, scalar_selection, scalar_output),
        Run::new(case_id, simd_selection, simd_output),
    )
}

/// Runs one fallible case with forced scalar and requested SIMD backend selection.
///
/// The closure receives the deterministic [`BackendSelection`] for each run.
///
/// # Errors
///
/// Returns the first error produced by the run closure.
pub fn try_run_scalar_and_simd<T, E, F>(case_id: &str, mut run: F) -> Result<RunPair<T>, E>
where
    F: FnMut(BackendSelection) -> Result<Vec<T>, E>,
{
    let scalar_selection = select_backend(BackendKind::Scalar);
    let scalar_output = run(scalar_selection)?;
    let simd_selection = select_backend(BackendKind::Simd);
    let simd_output = run(simd_selection)?;

    Ok(RunPair::new(
        case_id,
        Run::new(case_id, scalar_selection, scalar_output),
        Run::new(case_id, simd_selection, simd_output),
    ))
}

/// Metric thresholds for tolerance-based backend comparisons.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Tolerance {
    /// Maximum permitted absolute sample error.
    pub max_abs: f64,

    /// Maximum permitted root-mean-square sample error.
    pub rms: f64,

    /// Minimum permitted signal-to-noise ratio in decibels.
    pub snr_db: f64,
}

impl Tolerance {
    /// Creates metric thresholds for a tolerance-based comparison.
    #[must_use]
    pub const fn new(max_abs: f64, rms: f64, snr_db: f64) -> Self {
        Self {
            max_abs,
            rms,
            snr_db,
        }
    }
}

/// Error metrics computed between reference and actual backend outputs.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ErrorMetrics {
    /// Largest absolute sample error.
    pub max_abs: f64,

    /// Root-mean-square sample error.
    pub rms: f64,

    /// Signal-to-noise ratio in decibels.
    pub snr_db: f64,
}

/// Deterministic result of comparing two backend outputs.
#[derive(Debug, Clone, PartialEq)]
pub struct Comparison {
    case_id: String,
    reference_backend: String,
    actual_backend: String,
    reference_len: usize,
    actual_len: usize,
    first_failing_index: Option<usize>,
    metrics: ErrorMetrics,
    tolerance: Option<Tolerance>,
}

impl Comparison {
    /// Returns the stable conformance case identifier.
    #[must_use]
    pub fn case_id(&self) -> &str {
        &self.case_id
    }

    /// Returns the reference backend label.
    #[must_use]
    pub fn reference_backend(&self) -> &str {
        &self.reference_backend
    }

    /// Returns the actual backend label.
    #[must_use]
    pub fn actual_backend(&self) -> &str {
        &self.actual_backend
    }

    /// Returns the reference output length.
    #[must_use]
    pub const fn reference_len(&self) -> usize {
        self.reference_len
    }

    /// Returns the actual output length.
    #[must_use]
    pub const fn actual_len(&self) -> usize {
        self.actual_len
    }

    /// Returns the first index that failed the exact or tolerance check.
    #[must_use]
    pub const fn first_failing_index(&self) -> Option<usize> {
        self.first_failing_index
    }

    /// Returns computed error metrics.
    #[must_use]
    pub const fn metrics(&self) -> ErrorMetrics {
        self.metrics
    }

    /// Returns tolerance thresholds for tolerance-based comparisons.
    #[must_use]
    pub const fn tolerance(&self) -> Option<Tolerance> {
        self.tolerance
    }

    /// Returns whether the comparison passed.
    #[must_use]
    pub fn passed(&self) -> bool {
        if self.first_failing_index.is_some() {
            return false;
        }

        self.tolerance.is_none_or(|tolerance| {
            self.metrics.max_abs <= tolerance.max_abs
                && self.metrics.rms <= tolerance.rms
                && self.metrics.snr_db >= tolerance.snr_db
        })
    }

    /// Converts a comparison into a test-friendly pass/fail result.
    ///
    /// # Errors
    ///
    /// Returns [`Failure`] with the full comparison report when the comparison
    /// did not pass.
    pub fn require_passed(&self) -> Result<(), Failure> {
        if self.passed() {
            Ok(())
        } else {
            Err(Failure {
                comparison: Box::new(self.clone()),
            })
        }
    }
}

/// Failed backend conformance comparison.
#[derive(Debug, Clone, PartialEq)]
pub struct Failure {
    comparison: Box<Comparison>,
}

impl Failure {
    /// Returns the failed comparison report.
    #[must_use]
    pub const fn comparison(&self) -> &Comparison {
        &self.comparison
    }
}

impl fmt::Display for Failure {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        let comparison = &self.comparison;
        let first_index = comparison
            .first_failing_index()
            .map_or_else(|| "none".to_owned(), |index| index.to_string());
        let metrics = comparison.metrics();

        write!(
            formatter,
            "backend conformance case `{}` failed comparing `{}` to `{}`; \
             first failing index: {}; max_abs={:.6e}; rms={:.6e}; snr_db={:.6e}",
            comparison.case_id(),
            comparison.reference_backend(),
            comparison.actual_backend(),
            first_index,
            metrics.max_abs,
            metrics.rms,
            metrics.snr_db
        )
    }
}

impl Error for Failure {}

/// Compares two backend runs with exact value and length matching.
#[must_use]
pub fn compare_runs_exact<T>(reference: &Run<T>, actual: &Run<T>) -> Comparison
where
    T: Copy + Into<f64>,
{
    compare_outputs(
        reference.case_id(),
        &selection_label(reference.selection()),
        &selection_label(actual.selection()),
        reference.output(),
        actual.output(),
        None,
    )
}

/// Compares two backend runs with metric thresholds.
#[must_use]
pub fn compare_runs_with_tolerance<T>(
    reference: &Run<T>,
    actual: &Run<T>,
    tolerance: Tolerance,
) -> Comparison
where
    T: Copy + Into<f64>,
{
    compare_outputs(
        reference.case_id(),
        &selection_label(reference.selection()),
        &selection_label(actual.selection()),
        reference.output(),
        actual.output(),
        Some(tolerance),
    )
}

/// Compares two output slices with exact value and length matching.
#[must_use]
pub fn compare_exact<T>(
    case_id: &str,
    reference_backend: &str,
    actual_backend: &str,
    reference: &[T],
    actual: &[T],
) -> Comparison
where
    T: Copy + Into<f64>,
{
    compare_outputs(
        case_id,
        reference_backend,
        actual_backend,
        reference,
        actual,
        None,
    )
}

/// Compares two output slices with metric thresholds.
#[must_use]
pub fn compare_with_tolerance<T>(
    case_id: &str,
    reference_backend: &str,
    actual_backend: &str,
    reference: &[T],
    actual: &[T],
    tolerance: Tolerance,
) -> Comparison
where
    T: Copy + Into<f64>,
{
    compare_outputs(
        case_id,
        reference_backend,
        actual_backend,
        reference,
        actual,
        Some(tolerance),
    )
}

/// Returns a deterministic label for backend comparison reports.
#[must_use]
pub fn selection_label(selection: BackendSelection) -> String {
    match selection.fallback_reason() {
        Some(reason) => format!(
            "{} (requested {}, fallback: {})",
            selection.selected_name(),
            selection.requested_kind(),
            reason
        ),
        None => selection.selected_name().to_owned(),
    }
}

fn compare_outputs<T>(
    case_id: &str,
    reference_backend: &str,
    actual_backend: &str,
    reference: &[T],
    actual: &[T],
    tolerance: Option<Tolerance>,
) -> Comparison
where
    T: Copy + Into<f64>,
{
    let metrics = error_metrics(reference, actual);
    let first_failing_index = tolerance.map_or_else(
        || first_exact_failure(reference, actual),
        |thresholds| first_tolerance_failure(reference, actual, thresholds),
    );

    Comparison {
        case_id: case_id.to_owned(),
        reference_backend: reference_backend.to_owned(),
        actual_backend: actual_backend.to_owned(),
        reference_len: reference.len(),
        actual_len: actual.len(),
        first_failing_index,
        metrics,
        tolerance,
    }
}

fn first_exact_failure<T>(reference: &[T], actual: &[T]) -> Option<usize>
where
    T: Copy + Into<f64>,
{
    for index in 0..reference.len().max(actual.len()) {
        let Some(reference_value) = reference.get(index).copied() else {
            return Some(index);
        };
        let Some(actual_value) = actual.get(index).copied() else {
            return Some(index);
        };

        #[expect(
            clippy::float_cmp,
            reason = "exact conformance comparisons intentionally require equal numeric values"
        )]
        if reference_value.into() != actual_value.into() {
            return Some(index);
        }
    }

    None
}

fn first_tolerance_failure<T>(reference: &[T], actual: &[T], tolerance: Tolerance) -> Option<usize>
where
    T: Copy + Into<f64>,
{
    for index in 0..reference.len().max(actual.len()) {
        let Some(reference_value) = reference.get(index).copied() else {
            return Some(index);
        };
        let Some(actual_value) = actual.get(index).copied() else {
            return Some(index);
        };

        let error = (reference_value.into() - actual_value.into()).abs();
        if error.is_nan() || error > tolerance.max_abs {
            return Some(index);
        }
    }

    None
}

fn error_metrics<T>(reference: &[T], actual: &[T]) -> ErrorMetrics
where
    T: Copy + Into<f64>,
{
    let len = reference.len().max(actual.len());
    if len == 0 {
        return ErrorMetrics {
            max_abs: 0.0,
            rms: 0.0,
            snr_db: f64::INFINITY,
        };
    }

    let mut max_abs = 0.0_f64;
    let mut sum_squares = 0.0_f64;
    let mut reference_power = 0.0_f64;

    for index in 0..len {
        let reference_value = reference.get(index).copied().map_or(0.0, Into::into);
        let actual_value = actual.get(index).copied().map_or(0.0, Into::into);
        let error = reference_value - actual_value;

        if reference_value.is_nan() || actual_value.is_nan() || error.is_nan() {
            return ErrorMetrics {
                max_abs: f64::NAN,
                rms: f64::NAN,
                snr_db: f64::NAN,
            };
        }

        max_abs = max_abs.max(error.abs());
        sum_squares += error * error;
        reference_power += reference_value * reference_value;
    }

    #[expect(
        clippy::cast_precision_loss,
        reason = "test output lengths are small and f64 division is the metric domain"
    )]
    let rms = (sum_squares / len as f64).sqrt();
    let snr_db = match (reference_power == 0.0, sum_squares == 0.0) {
        (_, true) => f64::INFINITY,
        (true, false) => f64::NEG_INFINITY,
        (false, false) => 10.0 * (reference_power / sum_squares).log10(),
    };

    ErrorMetrics {
        max_abs,
        rms,
        snr_db,
    }
}

#[cfg(test)]
mod tests {
    use super::{
        Tolerance, compare_exact, compare_with_tolerance, run_scalar_and_simd,
        try_run_scalar_and_simd,
    };
    use auralis_simd::BackendKind;

    fn assert_float_eq(actual: f64, expected: f64) {
        assert!(
            (actual - expected).abs() <= f64::EPSILON,
            "expected {expected}, got {actual}"
        );
    }

    #[test]
    fn exact_comparison_accepts_matching_outputs() {
        let comparison = compare_exact("identity", "scalar", "simd", &[0_i16, 1, -2], &[0, 1, -2]);

        assert!(comparison.passed());
        assert_eq!(comparison.case_id(), "identity");
        assert_eq!(comparison.first_failing_index(), None);
        assert_float_eq(comparison.metrics().max_abs, 0.0);
        assert!(comparison.metrics().snr_db.is_infinite());
    }

    #[test]
    fn exact_comparison_reports_first_mismatch() {
        let comparison = compare_exact(
            "first_bad",
            "scalar",
            "simd",
            &[0.0_f32, 0.25, 0.5],
            &[0.0, 0.5, 0.5],
        );
        let failure = comparison
            .require_passed()
            .expect_err("comparison should fail");
        let message = failure.to_string();

        assert!(!comparison.passed());
        assert_eq!(comparison.first_failing_index(), Some(1));
        assert!(message.contains("first_bad"));
        assert!(message.contains("scalar"));
        assert!(message.contains("simd"));
        assert!(message.contains("first failing index: 1"));
        assert!(message.contains("max_abs="));
    }

    #[test]
    fn length_mismatch_reports_shorter_length_index() {
        let comparison = compare_exact("length", "scalar", "simd", &[0.0_f32], &[0.0, 0.0]);

        assert!(!comparison.passed());
        assert_eq!(comparison.first_failing_index(), Some(1));
        assert_eq!(comparison.reference_len(), 1);
        assert_eq!(comparison.actual_len(), 2);
    }

    #[test]
    fn tolerance_comparison_accepts_small_error() {
        let comparison = compare_with_tolerance(
            "within",
            "scalar",
            "simd",
            &[1.0_f32, -1.0],
            &[0.999, -1.001],
            Tolerance::new(0.002, 0.002, 50.0),
        );

        assert!(comparison.passed());
        assert_eq!(comparison.first_failing_index(), None);
        assert_float_eq(comparison.metrics().max_abs, 0.001_000_046_730_041_504);
    }

    #[test]
    fn tolerance_comparison_reports_threshold_failure() {
        let comparison = compare_with_tolerance(
            "outside",
            "scalar",
            "simd",
            &[1.0_f32, -1.0],
            &[1.25, -1.0],
            Tolerance::new(0.01, 0.01, 60.0),
        );

        assert!(!comparison.passed());
        assert_eq!(comparison.first_failing_index(), Some(0));
        assert!(comparison.metrics().rms > 0.1);
    }

    #[test]
    fn scalar_and_simd_runs_use_forced_backend_requests() {
        let runs = run_scalar_and_simd("forced", |_selection| vec![1.0_f32, 2.0]);

        assert_eq!(runs.case_id(), "forced");
        assert_eq!(runs.scalar().requested_kind(), BackendKind::Scalar);
        assert_eq!(runs.simd().requested_kind(), BackendKind::Simd);
        assert_eq!(runs.scalar().selected_name(), "scalar");
        assert!(runs.compare_exact().passed());
    }

    #[test]
    fn fallible_backend_runs_return_first_error() {
        let error = try_run_scalar_and_simd("fallible", |selection| {
            if selection.requested_kind() == BackendKind::Simd {
                Err("simd failed")
            } else {
                Ok(vec![0.0_f32])
            }
        })
        .expect_err("SIMD request should return the closure error");

        assert_eq!(error, "simd failed");
    }
}

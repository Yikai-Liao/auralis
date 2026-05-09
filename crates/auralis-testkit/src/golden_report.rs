//! Shared JSON schema for SoX-ng golden-test failure artifacts.
//!
//! Golden runners in Rust and Python use this shape when a comparison drifts
//! outside its manifest tolerance. The schema is intentionally explicit about
//! commands, versions, corpus IDs, output metadata, measured metrics, and the
//! individual threshold failures so artifacts can be compared across runners.

use std::collections::BTreeMap;

use serde::Serialize;

/// Stable schema marker written to every golden failure report.
pub const GOLDEN_FAILURE_REPORT_SCHEMA: &str = "auralis.golden.failure.v1";

/// A complete golden-test failure artifact.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct GoldenFailureReport {
    /// Stable report schema identifier.
    pub schema: &'static str,

    /// Manifest case identifier.
    pub case_id: String,

    /// Backend used by the Auralis command under test.
    pub backend: String,

    /// Auralis crate version or commit identifier.
    pub auralis_version: String,

    /// SoX-ng version string used for the reference command.
    pub sox_ng_version: String,

    /// Input fixture paths as recorded by the manifest.
    pub inputs: Vec<String>,

    /// Stable L0 corpus IDs used to generate the input fixtures.
    pub corpus_ids: Vec<String>,

    /// Auralis command vector that produced the actual output.
    pub auralis_command: Vec<String>,

    /// SoX-ng command vector that produced the expected output.
    pub sox_ng_command: Vec<String>,

    /// Manifest metric thresholds.
    pub thresholds: GoldenThresholds,

    /// Output metadata keyed by runner name, normally `auralis` and `sox_ng`.
    pub outputs: BTreeMap<String, GoldenOutputMetadata>,

    /// Measured comparison metrics keyed by metric name.
    pub metrics: BTreeMap<String, GoldenJsonNumber>,

    /// Threshold failures that caused the report to be written.
    pub failures: Vec<GoldenMetricFailure>,
}

impl GoldenFailureReport {
    /// Creates a report with the stable schema marker.
    #[must_use]
    pub fn new(case_id: impl Into<String>) -> Self {
        Self {
            schema: GOLDEN_FAILURE_REPORT_SCHEMA,
            case_id: case_id.into(),
            backend: "scalar".to_owned(),
            auralis_version: env!("CARGO_PKG_VERSION").to_owned(),
            sox_ng_version: "unknown".to_owned(),
            inputs: Vec::new(),
            corpus_ids: Vec::new(),
            auralis_command: Vec::new(),
            sox_ng_command: Vec::new(),
            thresholds: GoldenThresholds::default(),
            outputs: BTreeMap::new(),
            metrics: BTreeMap::new(),
            failures: Vec::new(),
        }
    }
}

/// Golden metric thresholds from the manifest.
#[derive(Debug, Default, Clone, Copy, PartialEq, Serialize)]
pub struct GoldenThresholds {
    /// Maximum permitted absolute sample error.
    pub max_abs: f64,

    /// Maximum permitted root-mean-square sample error.
    pub rms: f64,

    /// Minimum permitted signal-to-noise ratio in decibels.
    pub snr_db: f64,
}

/// Output stream metadata captured after decoding a golden runner output.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub struct GoldenOutputMetadata {
    /// Decoded sample rate in hertz.
    pub sample_rate: u32,

    /// Decoded channel count.
    pub channel_count: u16,

    /// Decoded frame count.
    pub frame_count: usize,
}

/// JSON-safe metric value.
#[derive(Debug, Clone, Copy, PartialEq, Serialize)]
#[serde(untagged)]
pub enum GoldenJsonNumber {
    /// Finite JSON numeric value.
    Number(f64),

    /// String fallback for non-finite values such as `inf`.
    NonFinite(&'static str),
}

impl GoldenJsonNumber {
    /// Converts an f64 into a JSON-safe value.
    #[must_use]
    pub fn from_f64(value: f64) -> Self {
        if value.is_finite() {
            Self::Number(value)
        } else if value.is_sign_positive() {
            Self::NonFinite("inf")
        } else {
            Self::NonFinite("-inf")
        }
    }
}

/// A metric threshold failure inside a golden failure artifact.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct GoldenMetricFailure {
    /// Metric that failed, such as `max_abs` or `snr_db`.
    pub metric: String,

    /// Expected threshold value from the manifest.
    pub expected: GoldenJsonNumber,

    /// Actual measured metric value.
    pub actual: GoldenJsonNumber,

    /// Comparison operator that defines pass/fail behavior.
    pub comparison: GoldenMetricComparison,

    /// First offending flattened sample index, when sample-local.
    pub first_offending_index: Option<usize>,
}

/// Threshold comparison direction for a failing metric.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub enum GoldenMetricComparison {
    /// Metric should have been less than or equal to the expected value.
    #[serde(rename = "<=")]
    LessThanOrEqual,

    /// Metric should have been greater than or equal to the expected value.
    #[serde(rename = ">=")]
    GreaterThanOrEqual,
}

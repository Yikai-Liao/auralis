//! Golden failure report schema coverage tests.

use std::collections::BTreeMap;

use auralis_testkit::golden_report::{
    GOLDEN_FAILURE_REPORT_SCHEMA, GoldenFailureReport, GoldenJsonNumber, GoldenMetricComparison,
    GoldenMetricFailure, GoldenOutputMetadata, GoldenThresholds,
};
use serde_json::json;

#[test]
fn golden_failure_report_serializes_stable_schema() {
    let mut report = GoldenFailureReport::new("gain_minus_3_mono");
    report.backend = "scalar".to_owned();
    report.sox_ng_version = "sox_ng 14.4.3".to_owned();
    report.inputs = vec!["sine_48k_mono.wav".to_owned()];
    report.corpus_ids = vec!["l0/sine_mono_32".to_owned()];
    report.auralis_command = vec![
        "auralis".to_owned(),
        "render".to_owned(),
        "input.wav".to_owned(),
        "-o".to_owned(),
        "output.wav".to_owned(),
    ];
    report.sox_ng_command = vec!["sox_ng".to_owned(), "-R".to_owned()];
    report.thresholds = GoldenThresholds {
        max_abs: 0.0001,
        rms: 0.000_001,
        snr_db: 90.0,
    };
    report.outputs = BTreeMap::from([
        (
            "auralis".to_owned(),
            GoldenOutputMetadata {
                sample_rate: 48_000,
                channel_count: 1,
                frame_count: 32,
            },
        ),
        (
            "sox_ng".to_owned(),
            GoldenOutputMetadata {
                sample_rate: 48_000,
                channel_count: 1,
                frame_count: 32,
            },
        ),
    ]);
    report.metrics = BTreeMap::from([("max_abs".to_owned(), GoldenJsonNumber::from_f64(0.0002))]);
    report.failures = vec![GoldenMetricFailure {
        metric: "max_abs".to_owned(),
        expected: GoldenJsonNumber::from_f64(0.0001),
        actual: GoldenJsonNumber::from_f64(0.0002),
        comparison: GoldenMetricComparison::LessThanOrEqual,
        first_offending_index: Some(7),
    }];

    let value = serde_json::to_value(&report).unwrap();

    assert_eq!(value["schema"], GOLDEN_FAILURE_REPORT_SCHEMA);
    assert_eq!(value["case_id"], "gain_minus_3_mono");
    assert_eq!(value["auralis_version"], env!("CARGO_PKG_VERSION"));
    assert_eq!(value["outputs"]["auralis"]["sample_rate"], 48_000);
    assert_eq!(value["outputs"]["auralis"]["channel_count"], 1);
    assert_eq!(value["outputs"]["auralis"]["frame_count"], 32);
    assert_eq!(
        value["failures"][0],
        json!({
            "metric": "max_abs",
            "expected": 0.0001,
            "actual": 0.0002,
            "comparison": "<=",
            "first_offending_index": 7
        })
    );
}

#[test]
fn golden_json_number_serializes_non_finite_values_as_strings() {
    assert_eq!(
        serde_json::to_value(GoldenJsonNumber::from_f64(f64::INFINITY)).unwrap(),
        json!("inf")
    );
    assert_eq!(
        serde_json::to_value(GoldenJsonNumber::from_f64(f64::NEG_INFINITY)).unwrap(),
        json!("-inf")
    );
}

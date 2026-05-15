//! Integration tests for golden manifest parsing and command rendering.

use std::path::PathBuf;

use auralis_testkit::golden::{
    quote_command_arg, render_command_line, GoldenCommand, GoldenManifest, GoldenManifestError,
    GoldenMetric,
};

const VALID_MANIFEST: &str = r#"
    [id.fade_out_stereo]
    input = "stereo/step.wav"
    corpus_id = "l0/step_mono_16"
    auralis = ["--fade-out-frame", "4"]
    sox_ng = ["fade", "0", "0", "4s"]
    max_abs = 0.0001
    rms = 0.000001
    snr_db = 90.0

    [id.gain_minus_3_mono]
    input = "mono/sine.wav"
    corpus_id = "l0/sine_mono_32"
    auralis = ["gain", "-3"]
    sox_ng = ["gain", "-3"]
    max_abs = 0.0001
    rms = 0.000001
    snr_db = 90.0
"#;

#[test]
fn manifest_parse_keeps_cases_in_deterministic_id_order() {
    let manifest = GoldenManifest::parse_toml(VALID_MANIFEST).unwrap();

    let ids = manifest.iter().map(|(id, _)| id).collect::<Vec<_>>();

    assert_eq!(ids, ["fade_out_stereo", "gain_minus_3_mono"]);
    assert_eq!(manifest.cases().len(), 2);
}

#[test]
fn manifest_parse_preserves_case_fields() {
    let manifest = GoldenManifest::parse_toml(VALID_MANIFEST).unwrap();
    let case = manifest.get("gain_minus_3_mono").unwrap();

    assert_eq!(case.input().to_string_lossy(), "mono/sine.wav");
    assert_eq!(case.inputs(), [PathBuf::from("mono/sine.wav")]);
    assert_eq!(case.corpus_id(), Some("l0/sine_mono_32"));
    assert_eq!(case.corpus_ids(), ["l0/sine_mono_32"]);
    assert_eq!(case.combine_method(), None);
    assert_eq!(case.auralis_args(), ["gain", "-3"]);
    assert_eq!(case.sox_ng_args(), ["gain", "-3"]);
    assert_float_eq(case.tolerance().max_abs, 0.000_1);
    assert_float_eq(case.tolerance().rms, 0.000_001);
    assert_float_eq(case.tolerance().snr_db, 90.0);
}

#[test]
fn invalid_manifest_is_rejected_for_missing_fields() {
    let error = GoldenManifest::parse_toml(
        r#"
        [id.missing_sox_command]
        input = "mono/sine.wav"
        auralis = ["gain", "-3"]
        max_abs = 0.0001
        rms = 0.000001
        snr_db = 90.0
        "#,
    )
    .unwrap_err();

    assert!(matches!(error, GoldenManifestError::Toml(_)));
}

#[test]
fn invalid_manifest_is_rejected_for_bad_case_id() {
    let error = GoldenManifest::parse_toml(
        r#"
        [id."gain minus 3"]
        input = "mono/sine.wav"
        auralis = ["gain", "-3"]
        sox_ng = ["gain", "-3"]
        max_abs = 0.0001
        rms = 0.000001
        snr_db = 90.0
        "#,
    )
    .unwrap_err();

    assert!(matches!(
        error,
        GoldenManifestError::InvalidCaseId { id } if id == "gain minus 3"
    ));
}

#[test]
fn invalid_manifest_is_rejected_for_empty_argument() {
    let error = GoldenManifest::parse_toml(
        r#"
        [id.gain_minus_3]
        input = "mono/sine.wav"
        auralis = ["--gain-db", ""]
        sox_ng = ["gain", "-3"]
        max_abs = 0.0001
        rms = 0.000001
        snr_db = 90.0
        "#,
    )
    .unwrap_err();

    assert!(matches!(
        error,
        GoldenManifestError::EmptyArgument {
            id,
            command: GoldenCommand::Auralis,
        } if id == "gain_minus_3"
    ));
}

#[test]
fn manifest_parse_preserves_multi_input_cases() {
    let manifest = GoldenManifest::parse_toml(
        r#"
        [id.concat_then_gain]
        inputs = ["combine/first.wav", "combine/second.wav"]
        corpus_ids = ["combine/mono_short", "combine/mono_long"]
        auralis = ["gain", "-3"]
        sox_ng = ["gain", "-3"]
        max_abs = 0.0001
        rms = 0.000001
        snr_db = 90.0
        "#,
    )
    .unwrap();
    let case = manifest.get("concat_then_gain").unwrap();

    assert_eq!(case.input().to_string_lossy(), "combine/first.wav");
    assert_eq!(
        case.inputs(),
        [
            PathBuf::from("combine/first.wav"),
            PathBuf::from("combine/second.wav")
        ]
    );
    assert_eq!(case.corpus_id(), Some("combine/mono_short"));
    assert_eq!(
        case.corpus_ids(),
        ["combine/mono_short", "combine/mono_long"]
    );
    assert_eq!(
        case.render_auralis_command_line_with_inputs(
            "auralis",
            ["first file.wav", "second file.wav"],
            "out.wav",
        ),
        "auralis render \"first file.wav\" -o out.wav --combine concatenate --input \"second file.wav\" --fx \"gain -3\""
    );
    assert_eq!(
        case.render_sox_ng_command_line_with_inputs(
            "sox_ng",
            ["first.wav", "second.wav"],
            "out.wav",
        ),
        "sox_ng -R -D --combine concatenate first.wav second.wav out.wav gain -3"
    );
}

#[test]
fn manifest_parse_preserves_explicit_combine_method() {
    let manifest = GoldenManifest::parse_toml(
        r#"
        [id.sequence_then_reverse]
        inputs = ["combine/first.wav", "combine/second.wav"]
        combine = "sequence"
        auralis = ["reverse"]
        sox_ng = ["reverse"]
        max_abs = 0.0
        rms = 0.0
        snr_db = 120.0
        "#,
    )
    .unwrap();
    let case = manifest.get("sequence_then_reverse").unwrap();

    assert_eq!(case.combine_method(), Some("sequence"));
    assert_eq!(
        case.render_auralis_command_line_with_inputs(
            "auralis",
            ["first.wav", "second.wav"],
            "out.wav",
        ),
        "auralis render first.wav -o out.wav --combine sequence --input second.wav --fx reverse"
    );
    assert_eq!(
        case.render_sox_ng_command_line_with_inputs(
            "sox_ng",
            ["first.wav", "second.wav"],
            "out.wav",
        ),
        "sox_ng -R -D --combine sequence first.wav second.wav out.wav reverse"
    );
}

#[test]
fn invalid_manifest_is_rejected_for_unknown_corpus_id() {
    let error = GoldenManifest::parse_toml(
        r#"
        [id.unknown_corpus]
        input = "mono/sine.wav"
        corpus_id = "missing/case"
        auralis = ["gain", "-3"]
        sox_ng = ["gain", "-3"]
        max_abs = 0.0001
        rms = 0.000001
        snr_db = 90.0
        "#,
    )
    .unwrap_err();

    assert!(matches!(
        error,
        GoldenManifestError::InvalidCorpusId { id, corpus_id }
            if id == "unknown_corpus" && corpus_id == "missing/case"
    ));
}

#[test]
fn invalid_manifest_is_rejected_for_mismatched_corpus_ids() {
    let error = GoldenManifest::parse_toml(
        r#"
        [id.bad_corpus_count]
        inputs = ["combine/first.wav", "combine/second.wav"]
        corpus_ids = ["combine/mono_short"]
        auralis = []
        sox_ng = []
        max_abs = 0.0
        rms = 0.0
        snr_db = 120.0
        "#,
    )
    .unwrap_err();

    assert!(matches!(
        error,
        GoldenManifestError::CorpusIdInputCountMismatch {
            id,
            inputs: 2,
            corpus_ids: 1,
        } if id == "bad_corpus_count"
    ));
}

#[test]
fn manifest_parse_records_output_channel_auto_conversion() {
    let manifest = GoldenManifest::parse_toml(
        r#"
        [id.auto_channels_stereo_to_mono]
        input = "auto/stereo.wav"
        output_channels = 1
        sox_ng_auto_channels = true
        auralis = []
        sox_ng = []
        max_abs = 0.0
        rms = 0.0
        snr_db = 120.0
        "#,
    )
    .unwrap();
    let case = manifest.get("auto_channels_stereo_to_mono").unwrap();

    assert_eq!(case.output_channels(), Some(1));
    assert!(case.sox_ng_auto_channels_inserted());
    assert_eq!(
        case.render_auralis_command_line("auralis", "stereo.wav", "mono.wav"),
        "auralis render stereo.wav -o mono.wav --channels 1"
    );
    assert_eq!(
        case.render_sox_ng_command_line("sox_ng", "stereo.wav", "mono.wav"),
        "sox_ng -R -D stereo.wav --channels 1 mono.wav"
    );
}

#[test]
fn manifest_parse_records_output_sample_rate_auto_conversion() {
    let manifest = GoldenManifest::parse_toml(
        r#"
        [id.auto_rate_downsample]
        input = "auto/rate_48k.wav"
        output_sample_rate = 24000
        sox_ng_auto_rate = true
        auralis = []
        sox_ng = []
        max_abs = 0.0
        rms = 0.0
        snr_db = 120.0
        "#,
    )
    .unwrap();
    let case = manifest.get("auto_rate_downsample").unwrap();

    assert_eq!(case.output_sample_rate(), Some(24_000));
    assert!(case.sox_ng_auto_rate_inserted());
    assert_eq!(
        case.render_auralis_command_line("auralis", "in.wav", "out.wav"),
        "auralis render in.wav -o out.wav --rate 24000"
    );
    assert_eq!(
        case.render_sox_ng_command_line("sox_ng", "in.wav", "out.wav"),
        "sox_ng -R -D in.wav --rate 24000 out.wav"
    );
}

#[test]
fn invalid_manifest_is_rejected_for_missing_or_ambiguous_input_fields() {
    let missing = GoldenManifest::parse_toml(
        r#"
        [id.missing_input]
        auralis = ["gain", "-3"]
        sox_ng = ["gain", "-3"]
        max_abs = 0.0001
        rms = 0.000001
        snr_db = 90.0
        "#,
    )
    .unwrap_err();
    let ambiguous = GoldenManifest::parse_toml(
        r#"
        [id.ambiguous_input]
        input = "mono/sine.wav"
        inputs = ["mono/sine.wav", "mono/other.wav"]
        auralis = ["gain", "-3"]
        sox_ng = ["gain", "-3"]
        max_abs = 0.0001
        rms = 0.000001
        snr_db = 90.0
        "#,
    )
    .unwrap_err();

    assert!(matches!(
        missing,
        GoldenManifestError::MissingInput { id } if id == "missing_input"
    ));
    assert!(matches!(
        ambiguous,
        GoldenManifestError::AmbiguousInput { id } if id == "ambiguous_input"
    ));
}

#[test]
fn invalid_manifest_is_rejected_for_unknown_combine_method() {
    let error = GoldenManifest::parse_toml(
        r#"
        [id.unknown_combine]
        inputs = ["first.wav", "second.wav"]
        combine = "overlay"
        auralis = ["gain", "-3"]
        sox_ng = ["gain", "-3"]
        max_abs = 0.0001
        rms = 0.000001
        snr_db = 90.0
        "#,
    )
    .unwrap_err();

    assert!(matches!(
        error,
        GoldenManifestError::InvalidCombineMethod { id, combine }
            if id == "unknown_combine" && combine == "overlay"
    ));
}

#[test]
fn invalid_manifest_is_rejected_for_auto_channels_without_output_channels() {
    let error = GoldenManifest::parse_toml(
        r#"
        [id.missing_output_channels]
        input = "auto/stereo.wav"
        sox_ng_auto_channels = true
        auralis = []
        sox_ng = []
        max_abs = 0.0
        rms = 0.0
        snr_db = 120.0
        "#,
    )
    .unwrap_err();

    assert!(matches!(
        error,
        GoldenManifestError::AutoChannelsWithoutOutputChannels { id }
            if id == "missing_output_channels"
    ));
}

#[test]
fn invalid_manifest_is_rejected_for_auto_rate_without_output_sample_rate() {
    let error = GoldenManifest::parse_toml(
        r#"
        [id.missing_output_sample_rate]
        input = "auto/rate_48k.wav"
        sox_ng_auto_rate = true
        auralis = []
        sox_ng = []
        max_abs = 0.0
        rms = 0.0
        snr_db = 120.0
        "#,
    )
    .unwrap_err();

    assert!(matches!(
        error,
        GoldenManifestError::AutoRateWithoutOutputSampleRate { id }
            if id == "missing_output_sample_rate"
    ));
}

#[test]
fn invalid_manifest_is_rejected_for_negative_tolerance() {
    let error = GoldenManifest::parse_toml(
        r#"
        [id.gain_minus_3]
        input = "mono/sine.wav"
        auralis = ["gain", "-3"]
        sox_ng = ["gain", "-3"]
        max_abs = -0.0001
        rms = 0.000001
        snr_db = 90.0
        "#,
    )
    .unwrap_err();

    assert!(matches!(
        error,
        GoldenManifestError::InvalidTolerance {
            id,
            metric: GoldenMetric::MaxAbs,
            ..
        } if id == "gain_minus_3"
    ));
}

#[test]
fn command_rendering_is_deterministic() {
    let manifest = GoldenManifest::parse_toml(VALID_MANIFEST).unwrap();
    let case = manifest.get("gain_minus_3_mono").unwrap();

    assert_eq!(
        case.render_auralis_command("auralis", "/tmp/in.wav", "/tmp/out.wav"),
        [
            "auralis",
            "render",
            "/tmp/in.wav",
            "-o",
            "/tmp/out.wav",
            "--fx",
            "gain -3"
        ],
    );
    assert_eq!(
        case.render_sox_ng_command("sox_ng", "/tmp/in.wav", "/tmp/out.wav"),
        [
            "sox_ng",
            "-R",
            "-D",
            "/tmp/in.wav",
            "/tmp/out.wav",
            "gain",
            "-3"
        ],
    );
}

#[test]
fn command_line_rendering_quotes_and_escapes_deterministically() {
    let command = [
        "auralis",
        "render",
        "input file.wav",
        "-o",
        "quote\"and\\slash",
        "line\nbreak",
        "tab\tvalue",
        "",
    ];

    assert_eq!(
        render_command_line(command),
        "auralis render \"input file.wav\" -o \"quote\\\"and\\\\slash\" \"line\\nbreak\" \"tab\\tvalue\" \"\""
    );
    assert_eq!(quote_command_arg("safe/path-1.wav"), "safe/path-1.wav");
    assert_eq!(quote_command_arg("needs space"), "\"needs space\"");
}

#[test]
fn golden_manifest_command_line_rendering_is_stable_across_runs() {
    let manifest = GoldenManifest::parse_toml(
        r#"
        [id.quoted_paths]
        input = "fixtures/input file.wav"
        auralis = ["gain", "-3"]
        sox_ng = ["gain", "-3"]
        max_abs = 0.0001
        rms = 0.000001
        snr_db = 90.0
        "#,
    )
    .unwrap();
    let case = manifest.get("quoted_paths").unwrap();

    let first = case.render_auralis_command_line(
        "auralis",
        "fixtures/input file.wav",
        "tmp/output file.wav",
    );
    let second = case.render_auralis_command_line(
        "auralis",
        "fixtures/input file.wav",
        "tmp/output file.wav",
    );

    assert_eq!(first, second);
    assert_eq!(
        first,
        "auralis render \"fixtures/input file.wav\" -o \"tmp/output file.wav\" --fx \"gain -3\""
    );
    assert_eq!(
        case.render_sox_ng_command_line("sox_ng", "fixtures/input file.wav", "tmp/out.wav"),
        "sox_ng -R -D \"fixtures/input file.wav\" tmp/out.wav gain -3"
    );
}

fn assert_float_eq(actual: f64, expected: f64) {
    assert!(
        (actual - expected).abs() <= f64::EPSILON,
        "expected {expected}, got {actual}"
    );
}

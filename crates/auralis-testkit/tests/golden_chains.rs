//! Golden-chain manifest coverage tests.

use auralis_testkit::golden::GoldenManifest;

const CHAIN_MANIFEST: &str = include_str!("../../../tests/golden/chains.toml");

#[test]
fn chain_golden_manifest_records_representative_chain_cases() {
    let manifest = GoldenManifest::parse_toml(CHAIN_MANIFEST).unwrap();
    let ids = manifest.iter().map(|(id, _)| id).collect::<Vec<_>>();

    assert_eq!(
        ids,
        [
            "chain_editing_trim_reverse_pad",
            "chain_filter_fade_gain",
            "chain_level_gain_dcshift_gain"
        ]
    );
}

#[test]
fn chain_golden_manifest_renders_recorded_commands() {
    let manifest = GoldenManifest::parse_toml(CHAIN_MANIFEST).unwrap();
    let editing = manifest.get("chain_editing_trim_reverse_pad").unwrap();
    let level = manifest.get("chain_level_gain_dcshift_gain").unwrap();
    let filter = manifest.get("chain_filter_fade_gain").unwrap();

    assert_eq!(
        editing.auralis_args(),
        ["trim", "2", "9", "reverse", "pad", "1", "2"]
    );
    assert_eq!(
        editing.sox_ng_args(),
        ["trim", "2s", "7s", "reverse", "pad", "1s", "2s"]
    );

    assert_eq!(
        level.render_auralis_command_line("auralis", "input file.wav", "out.wav"),
        "auralis run \"input file.wav\" out.wav gain -3 dcshift 0.125 gain -1"
    );
    assert_eq!(
        filter.render_sox_ng_command_line("sox_ng", "input.wav", "out.wav"),
        "sox_ng -R -D input.wav out.wav fade t 5s gain -2"
    );
}

#[test]
fn chain_golden_manifest_renders_boundary_tokens_deterministically() {
    let manifest = GoldenManifest::parse_toml(
        r#"
        [id.chain_boundary_rendering]
        input = "chains/stereo_steps.wav"
        auralis = ["gain", "-3", ":", "dcshift", "0.125"]
        sox_ng = ["gain", "-3", ":", "dcshift", "0.125"]
        max_abs = 0.000031
        rms = 0.000031
        snr_db = 90.0
        "#,
    )
    .unwrap();
    let case = manifest.get("chain_boundary_rendering").unwrap();

    assert_eq!(
        case.render_auralis_command_line("auralis", "input.wav", "out.wav"),
        "auralis run input.wav out.wav gain -3 : dcshift 0.125"
    );
    assert_eq!(
        case.render_sox_ng_command_line("sox_ng", "input.wav", "out.wav"),
        "sox_ng -R -D input.wav out.wav gain -3 : dcshift 0.125"
    );
}

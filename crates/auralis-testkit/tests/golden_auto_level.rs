//! Golden manifest coverage for explicit output guard and normalization policies.

use auralis_testkit::golden::GoldenManifest;

const AUTO_LEVEL_MANIFEST: &str = include_str!("../../../tests/golden/auto_level.toml");

#[test]
fn auto_level_golden_manifest_records_representative_cases() {
    let manifest = GoldenManifest::parse_toml(AUTO_LEVEL_MANIFEST).unwrap();
    let ids = manifest.iter().map(|(id, _)| id).collect::<Vec<_>>();

    assert_eq!(
        ids,
        [
            "auto_guard_gain_clipping",
            "auto_norm_default",
            "auto_norm_minus_6"
        ]
    );
}

#[test]
fn auto_level_golden_manifest_renders_guard_and_norm_commands() {
    let manifest = GoldenManifest::parse_toml(AUTO_LEVEL_MANIFEST).unwrap();
    let guard = manifest.get("auto_guard_gain_clipping").unwrap();
    let norm = manifest.get("auto_norm_minus_6").unwrap();

    assert_eq!(
        guard.render_auralis_command_line("auralis", "in.wav", "out.wav"),
        "auralis render in.wav -o out.wav --guard --fx \"gain 6\""
    );
    assert_eq!(
        guard.render_sox_ng_command_line("sox_ng", "in.wav", "out.wav"),
        "sox_ng -R -D in.wav out.wav norm"
    );
    assert_eq!(
        norm.render_auralis_command_line("auralis", "in.wav", "out.wav"),
        "auralis render in.wav -o out.wav --norm=-6"
    );
    assert_eq!(
        norm.render_sox_ng_command_line("sox_ng", "in.wav", "out.wav"),
        "sox_ng -R -D in.wav out.wav norm -6"
    );
}

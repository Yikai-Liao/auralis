//! Golden manifest coverage for automatic output dither policy.

use auralis_testkit::golden::GoldenManifest;

const AUTO_DITHER_MANIFEST: &str = include_str!("../../../tests/golden/auto_dither.toml");

#[test]
fn auto_dither_golden_manifest_records_representative_cases() {
    let manifest = GoldenManifest::parse_toml(AUTO_DITHER_MANIFEST).unwrap();
    let ids = manifest.iter().map(|(id, _)| id).collect::<Vec<_>>();

    assert_eq!(ids, ["auto_dither_gain_to_pcm16"]);
}

#[test]
fn auto_dither_golden_manifest_renders_sox_ng_without_disabled_dither() {
    let manifest = GoldenManifest::parse_toml(AUTO_DITHER_MANIFEST).unwrap();
    let dither = manifest.get("auto_dither_gain_to_pcm16").unwrap();

    assert!(dither.sox_ng_auto_dither_inserted());
    assert_eq!(
        dither.render_auralis_command_line("auralis", "in.wav", "out.wav"),
        "auralis render in.wav -o out.wav --dither --fx \"gain -0.1\""
    );
    assert_eq!(
        dither.render_sox_ng_command_line("sox_ng", "in.wav", "out.wav"),
        "sox_ng -R in.wav out.wav gain -0.1"
    );
}

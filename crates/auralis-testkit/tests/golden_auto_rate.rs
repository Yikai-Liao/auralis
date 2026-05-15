//! Golden manifest coverage for automatic output sample-rate conversion.

use auralis_testkit::golden::GoldenManifest;

const AUTO_RATE_MANIFEST: &str = include_str!("../../../tests/golden/auto_rate.toml");

#[test]
fn auto_rate_golden_manifest_records_representative_cases() {
    let manifest = GoldenManifest::parse_toml(AUTO_RATE_MANIFEST).unwrap();
    let ids = manifest.iter().map(|(id, _)| id).collect::<Vec<_>>();

    assert_eq!(
        ids,
        [
            "auto_rate_downsample_48k_to_24k",
            "auto_rate_upsample_48k_to_96k"
        ]
    );
}

#[test]
fn auto_rate_golden_manifest_renders_output_rate_options() {
    let manifest = GoldenManifest::parse_toml(AUTO_RATE_MANIFEST).unwrap();
    let downsample = manifest.get("auto_rate_downsample_48k_to_24k").unwrap();
    let upsample = manifest.get("auto_rate_upsample_48k_to_96k").unwrap();

    assert_eq!(downsample.output_sample_rate(), Some(24_000));
    assert!(downsample.sox_ng_auto_rate_inserted());
    assert_eq!(upsample.output_sample_rate(), Some(96_000));
    assert!(upsample.sox_ng_auto_rate_inserted());
    assert_eq!(
        downsample.render_auralis_command_line("auralis", "in.wav", "out.wav"),
        "auralis render in.wav -o out.wav --rate 24000"
    );
    assert_eq!(
        downsample.render_sox_ng_command_line("sox_ng", "in.wav", "out.wav"),
        "sox_ng -R -D in.wav --rate 24000 out.wav"
    );
}

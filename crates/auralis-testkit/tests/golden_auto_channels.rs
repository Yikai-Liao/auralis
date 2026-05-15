//! Golden manifest coverage for automatic output channel conversion.

use auralis_testkit::golden::GoldenManifest;

const AUTO_CHANNELS_MANIFEST: &str = include_str!("../../../tests/golden/auto_channels.toml");

#[test]
fn auto_channels_golden_manifest_records_representative_cases() {
    let manifest = GoldenManifest::parse_toml(AUTO_CHANNELS_MANIFEST).unwrap();
    let ids = manifest.iter().map(|(id, _)| id).collect::<Vec<_>>();

    assert_eq!(
        ids,
        [
            "auto_channels_mono_to_stereo",
            "auto_channels_stereo_to_mono"
        ]
    );
}

#[test]
fn auto_channels_golden_manifest_renders_output_channel_options() {
    let manifest = GoldenManifest::parse_toml(AUTO_CHANNELS_MANIFEST).unwrap();
    let mono = manifest.get("auto_channels_mono_to_stereo").unwrap();
    let stereo = manifest.get("auto_channels_stereo_to_mono").unwrap();

    assert_eq!(mono.output_channels(), Some(2));
    assert!(mono.sox_ng_auto_channels_inserted());
    assert_eq!(stereo.output_channels(), Some(1));
    assert!(stereo.sox_ng_auto_channels_inserted());
    assert_eq!(
        stereo.render_auralis_command_line("auralis", "stereo.wav", "mono.wav"),
        "auralis render stereo.wav -o mono.wav --channels 1"
    );
    assert_eq!(
        stereo.render_sox_ng_command_line("sox_ng", "stereo.wav", "mono.wav"),
        "sox_ng -R -D stereo.wav --channels 1 mono.wav"
    );
}

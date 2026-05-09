//! Golden-combiner manifest coverage tests.

use auralis_testkit::golden::GoldenManifest;

const CONCAT_MANIFEST: &str = include_str!("../../../tests/golden/concat.toml");
const SEQUENCE_MANIFEST: &str = include_str!("../../../tests/golden/sequence.toml");

#[test]
fn concat_golden_manifest_records_representative_cases() {
    let manifest = GoldenManifest::parse_toml(CONCAT_MANIFEST).unwrap();
    let ids = manifest.iter().map(|(id, _)| id).collect::<Vec<_>>();

    assert_eq!(
        ids,
        [
            "concat_mono_mismatched_lengths",
            "concat_stereo_reverse_chain"
        ]
    );
}

#[test]
fn concat_golden_manifest_renders_multi_input_commands() {
    let manifest = GoldenManifest::parse_toml(CONCAT_MANIFEST).unwrap();
    let mono = manifest.get("concat_mono_mismatched_lengths").unwrap();
    let stereo = manifest.get("concat_stereo_reverse_chain").unwrap();

    assert_eq!(
        mono.inputs()
            .iter()
            .map(|path| path.to_string_lossy().into_owned())
            .collect::<Vec<_>>(),
        ["combine/mono_short.wav", "combine/mono_long.wav"]
    );
    assert_eq!(
        mono.render_auralis_command_line_with_inputs(
            "auralis",
            ["short.wav", "long.wav"],
            "out.wav",
        ),
        "auralis run short.wav out.wav --combine concatenate --input long.wav"
    );
    assert_eq!(
        stereo.render_sox_ng_command_line_with_inputs(
            "sox_ng",
            ["front.wav", "tail.wav"],
            "out.wav",
        ),
        "sox_ng -R -D --combine concatenate front.wav tail.wav out.wav reverse"
    );
}

#[test]
fn sequence_golden_manifest_records_representative_cases() {
    let manifest = GoldenManifest::parse_toml(SEQUENCE_MANIFEST).unwrap();
    let ids = manifest.iter().map(|(id, _)| id).collect::<Vec<_>>();

    assert_eq!(
        ids,
        ["sequence_mono_boundary", "sequence_stereo_reverse_chain"]
    );
}

#[test]
fn sequence_golden_manifest_renders_multi_input_commands() {
    let manifest = GoldenManifest::parse_toml(SEQUENCE_MANIFEST).unwrap();
    let mono = manifest.get("sequence_mono_boundary").unwrap();
    let stereo = manifest.get("sequence_stereo_reverse_chain").unwrap();

    assert_eq!(mono.combine_method(), Some("sequence"));
    assert_eq!(
        mono.render_auralis_command_line_with_inputs(
            "auralis",
            ["short.wav", "long.wav"],
            "out.wav",
        ),
        "auralis run short.wav out.wav --combine sequence --input long.wav"
    );
    assert_eq!(
        stereo.render_sox_ng_command_line_with_inputs(
            "sox_ng",
            ["front.wav", "tail.wav"],
            "out.wav",
        ),
        "sox_ng -R -D --combine sequence front.wav tail.wav out.wav reverse"
    );
}

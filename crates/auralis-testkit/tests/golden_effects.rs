//! Golden manifest coverage for standalone implemented effects.

use auralis_testkit::golden::GoldenManifest;

const EFFECTS_MANIFEST: &str = include_str!("../../../tests/golden/effects.toml");

#[test]
fn effects_golden_manifest_records_standalone_effect_cases() {
    let manifest = GoldenManifest::parse_toml(EFFECTS_MANIFEST).unwrap();
    let ids = manifest.iter().map(|(id, _)| id).collect::<Vec<_>>();

    assert_eq!(
        ids,
        [
            "effect_contrast_mono_default",
            "effect_contrast_stereo_amount_25",
            "effect_dcshift_limiter_mono_positive",
            "effect_dcshift_mono_positive",
            "effect_dcshift_stereo_negative",
            "effect_fade_mono_half_sine_in",
            "effect_fade_mono_inverted_parabola_in",
            "effect_fade_mono_linear_in",
            "effect_fade_mono_linear_out_at_end",
            "effect_fade_mono_logarithmic_in",
            "effect_fade_mono_quarter_sine_in",
            "effect_fade_stereo_linear_in",
            "effect_fade_stereo_linear_stop_position",
            "effect_gain_balance_no_clip_stereo_plus_6",
            "effect_gain_balance_stereo_plus_6",
            "effect_gain_equalize_stereo_minus_6",
            "effect_gain_headroom_mono_minus_6",
            "effect_gain_limiter_mono_plus_6",
            "effect_gain_mono_minus_3",
            "effect_gain_normalize_mono_minus_3",
            "effect_gain_stereo_minus_6",
            "effect_norm_mono_minus_3",
            "effect_norm_stereo_default",
            "effect_pad_mono_both_sides",
            "effect_pad_mono_positioned",
            "effect_pad_stereo_both_sides",
            "effect_pad_stereo_positioned",
            "effect_reverse_mono_odd_length",
            "effect_reverse_stereo",
            "effect_trim_mono_middle",
            "effect_trim_mono_multiple_ranges",
            "effect_trim_stereo_absolute_resume",
            "effect_trim_stereo_middle",
            "effect_vol_limiter_mono_plus_2",
            "effect_vol_mono_half_amplitude",
            "effect_vol_stereo_minus_6_db",
            "effect_vol_stereo_quarter_power"
        ]
    );
}

#[test]
fn effects_golden_manifest_covers_each_effect_in_mono_and_stereo() {
    let manifest = GoldenManifest::parse_toml(EFFECTS_MANIFEST).unwrap();

    for effect in [
        "gain", "dcshift", "trim", "pad", "reverse", "fade", "vol", "norm", "contrast",
    ] {
        let mono = manifest
            .iter()
            .any(|(id, case)| id.contains(effect) && case.corpus_id().unwrap().contains("mono"));
        let stereo = manifest
            .iter()
            .any(|(id, case)| id.contains(effect) && case.corpus_id().unwrap().contains("stereo"));

        assert!(mono, "missing mono standalone golden case for {effect}");
        assert!(stereo, "missing stereo standalone golden case for {effect}");
    }
}

#[test]
fn effects_golden_manifest_renders_representative_commands() {
    let manifest = GoldenManifest::parse_toml(EFFECTS_MANIFEST).unwrap();
    let gain = manifest.get("effect_gain_mono_minus_3").unwrap();
    let trim = manifest.get("effect_trim_stereo_middle").unwrap();
    let fade = manifest
        .get("effect_fade_stereo_linear_stop_position")
        .unwrap();

    assert_eq!(
        gain.render_auralis_command_line("auralis", "in.wav", "out.wav"),
        "auralis run in.wav out.wav gain -3"
    );
    assert_eq!(
        trim.render_sox_ng_command_line("sox_ng", "in.wav", "out.wav"),
        "sox_ng -R -D in.wav out.wav trim 4s 24s"
    );
    assert_eq!(
        fade.render_sox_ng_command_line("sox_ng", "in.wav", "out.wav"),
        "sox_ng -R -D in.wav out.wav fade t 0 24s 6s"
    );
}

#[test]
fn effects_golden_manifest_keeps_automatic_rate_and_channels_absent() {
    let manifest = GoldenManifest::parse_toml(EFFECTS_MANIFEST).unwrap();

    for (id, case) in manifest.iter() {
        assert_eq!(
            case.output_channels(),
            None,
            "{id} should not request output channel conversion"
        );
        assert_eq!(
            case.output_sample_rate(),
            None,
            "{id} should not request output rate conversion"
        );
        assert!(
            !case.sox_ng_auto_channels_inserted(),
            "{id} should record SoX-ng channel auto-conversion as absent"
        );
        assert!(
            !case.sox_ng_auto_rate_inserted(),
            "{id} should record SoX-ng rate auto-conversion as absent"
        );
    }
}

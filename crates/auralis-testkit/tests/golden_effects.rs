//! Golden manifest coverage for standalone implemented effects.

use auralis_testkit::golden::GoldenManifest;

const EFFECTS_MANIFEST: &str = include_str!("../../../tests/golden/effects.toml");

#[test]
#[allow(clippy::too_many_lines)]
fn effects_golden_manifest_records_standalone_effect_cases() {
    let manifest = GoldenManifest::parse_toml(EFFECTS_MANIFEST).unwrap();
    let ids = manifest.iter().map(|(id, _)| id).collect::<Vec<_>>();

    assert_eq!(
        ids,
        [
            "effect_allpass_mono_rbj",
            "effect_allpass_stereo_one_pole",
            "effect_band_mono_default",
            "effect_band_stereo_unpitched",
            "effect_bandpass_mono_default",
            "effect_bandpass_stereo_constant_skirt",
            "effect_bandreject_mono_default",
            "effect_bandreject_stereo_q_width",
            "effect_bass_mono_default",
            "effect_bass_stereo_q_width",
            "effect_biquad_mono_one_pole",
            "effect_biquad_stereo_one_pole",
            "effect_centercut_stereo_default",
            "effect_centercut_stereo_options",
            "effect_channels_mono_to_stereo",
            "effect_channels_stereo_to_mono",
            "effect_chorus_mono_linear",
            "effect_chorus_stereo_multi_stage",
            "effect_contrast_mono_default",
            "effect_contrast_stereo_amount_25",
            "effect_dcshift_limiter_mono_positive",
            "effect_dcshift_mono_positive",
            "effect_dcshift_stereo_negative",
            "effect_deemph_mono_48000",
            "effect_deemph_stereo_48000",
            "effect_delay_mono_frames",
            "effect_delay_stereo_per_channel",
            "effect_downsample_mono_default",
            "effect_downsample_stereo_factor_3",
            "effect_echo_mono_delay",
            "effect_echo_stereo_zero_delay",
            "effect_echos_mono_cascaded",
            "effect_echos_stereo_single_tap",
            "effect_equalizer_mono_default",
            "effect_equalizer_stereo_octave_width",
            "effect_fade_mono_half_sine_in",
            "effect_fade_mono_inverted_parabola_in",
            "effect_fade_mono_linear_in",
            "effect_fade_mono_linear_out_at_end",
            "effect_fade_mono_logarithmic_in",
            "effect_fade_mono_quarter_sine_in",
            "effect_fade_stereo_linear_in",
            "effect_fade_stereo_linear_stop_position",
            "effect_flanger_mono_linear",
            "effect_flanger_stereo_triangle",
            "effect_gain_balance_no_clip_stereo_plus_6",
            "effect_gain_balance_stereo_plus_6",
            "effect_gain_equalize_stereo_minus_6",
            "effect_gain_headroom_mono_minus_6",
            "effect_gain_limiter_mono_plus_6",
            "effect_gain_mono_minus_3",
            "effect_gain_normalize_mono_minus_3",
            "effect_gain_stereo_minus_6",
            "effect_highpass_mono_default",
            "effect_highpass_stereo_one_pole",
            "effect_lowpass_mono_default",
            "effect_lowpass_stereo_one_pole",
            "effect_norm_mono_minus_3",
            "effect_norm_stereo_default",
            "effect_oops_stereo",
            "effect_overdrive_mono_default",
            "effect_overdrive_stereo_explicit",
            "effect_pad_mono_both_sides",
            "effect_pad_mono_positioned",
            "effect_pad_stereo_both_sides",
            "effect_pad_stereo_positioned",
            "effect_phaser_mono_linear",
            "effect_phaser_stereo_triangle",
            "effect_pitch_mono_octave_up",
            "effect_pitch_stereo_quick_octave_down",
            "effect_rate_generic_custom_overrides_stereo_identity",
            "effect_rate_high_overrides_mono_to_24000",
            "effect_rate_low_stereo_identity",
            "effect_rate_medium_mono_to_24000",
            "effect_rate_mono_to_24000",
            "effect_rate_quick_mono_to_24000",
            "effect_rate_stereo_identity",
            "effect_rate_ultra_stereo_identity",
            "effect_remix_mono_silent_copy",
            "effect_remix_stereo_auto_power",
            "effect_remix_stereo_gain_modifiers",
            "effect_remix_stereo_mixdown",
            "effect_repeat_mono_count_2",
            "effect_repeat_stereo_default",
            "effect_reverb_mono_wet_only",
            "effect_reverb_stereo_default",
            "effect_reverse_mono_odd_length",
            "effect_reverse_stereo",
            "effect_riaa_mono_48000",
            "effect_riaa_stereo_48000",
            "effect_saturation_mono_default",
            "effect_saturation_stereo_sqrt",
            "effect_softvol_mono_gain_1_5",
            "effect_softvol_stereo_recovery_headroom",
            "effect_speed_mono_ratio_1_5",
            "effect_speed_stereo_ratio_0_5",
            "effect_stretch_mono_lengthen",
            "effect_stretch_stereo_shorten",
            "effect_swap_mono_identity",
            "effect_swap_stereo",
            "effect_tempo_mono_quick_speech",
            "effect_tempo_mono_speed_up",
            "effect_tempo_stereo_linear_tuned",
            "effect_tempo_stereo_slow_down",
            "effect_treble_mono_default",
            "effect_treble_stereo_q_width",
            "effect_tremolo_mono_default_depth",
            "effect_tremolo_stereo_depth_75",
            "effect_trim_mono_middle",
            "effect_trim_mono_multiple_ranges",
            "effect_trim_stereo_absolute_resume",
            "effect_trim_stereo_middle",
            "effect_upsample_mono_default",
            "effect_upsample_stereo_factor_3",
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
        "allpass",
        "band",
        "bandpass",
        "bandreject",
        "bass",
        "treble",
        "biquad",
        "gain",
        "highpass",
        "lowpass",
        "centercut",
        "dcshift",
        "delay",
        "downsample",
        "upsample",
        "echo",
        "echos",
        "deemph",
        "equalizer",
        "trim",
        "pad",
        "reverse",
        "riaa",
        "fade",
        "flanger",
        "phaser",
        "pitch",
        "rate",
        "vol",
        "norm",
        "oops",
        "contrast",
        "softvol",
        "tremolo",
        "overdrive",
        "saturation",
        "repeat",
        "reverb",
        "speed",
        "stretch",
        "tempo",
        "channels",
        "chorus",
        "remix",
        "swap",
    ] {
        let mono = manifest
            .iter()
            .any(|(id, case)| id.contains(effect) && case.corpus_id().unwrap().contains("mono"));
        let stereo = manifest
            .iter()
            .any(|(id, case)| id.contains(effect) && case.corpus_id().unwrap().contains("stereo"));

        if !matches!(effect, "oops" | "centercut") {
            assert!(mono, "missing mono standalone golden case for {effect}");
        }
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
    let centercut = manifest.get("effect_centercut_stereo_options").unwrap();

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
    assert_eq!(
        centercut.render_auralis_command_line("auralis", "in.wav", "out.wav"),
        "auralis run in.wav out.wav centercut -a 0.5 -b -w 16"
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
        if id.starts_with("effect_downsample_")
            || id.starts_with("effect_pitch_")
            || id.starts_with("effect_speed_")
            || id.starts_with("effect_upsample_")
        {
            assert!(
                case.output_sample_rate().is_some(),
                "{id} should request the effect output rate explicitly"
            );
        } else {
            assert_eq!(
                case.output_sample_rate(),
                None,
                "{id} should not request output rate conversion"
            );
        }
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

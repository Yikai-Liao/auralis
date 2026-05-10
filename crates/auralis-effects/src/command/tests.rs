use super::{EffectCommand, EffectCommandParseError, parse_effect_command};
use crate::{
    Centercut, Contrast, DcShift, EffectError, Fade, FadeCurve, Gain, GainChannelMode, Pad,
    PositionedPad, Saturation, SoftVol, Trim, TrimPosition,
};
use auralis_core::{Decibels, FrameCount};

#[test]
fn parses_supported_aliases_through_registry_resolution() {
    assert_eq!(
        parse_effect_command(&["dc-shift", "-0.25"]).unwrap(),
        EffectCommand::DcShift(DcShift::new(-0.25).unwrap())
    );
    assert_eq!(
        parse_effect_command(&["gain-db", "6"]).unwrap(),
        EffectCommand::Gain(Gain::new(Decibels::new(6.0).unwrap()))
    );
}

#[test]
fn defaults_match_existing_identity_configs() {
    assert_eq!(
        parse_effect_command(&["gain"]).unwrap(),
        EffectCommand::Gain(Gain::new(Decibels::new(0.0).unwrap()))
    );
    assert_eq!(
        parse_effect_command(&["contrast"]).unwrap(),
        EffectCommand::Contrast(Contrast::default_amount())
    );
    assert_eq!(
        parse_effect_command(&["centercut"]).unwrap(),
        EffectCommand::Centercut(Centercut::new())
    );
    assert_eq!(
        parse_effect_command(&["pad"]).unwrap(),
        EffectCommand::Pad(Pad::new(FrameCount::new(0), FrameCount::new(0)))
    );
    assert_eq!(
        parse_effect_command(&["softvol"]).unwrap(),
        EffectCommand::SoftVol(SoftVol::default())
    );
    assert_eq!(
        parse_effect_command(&["saturation"]).unwrap(),
        EffectCommand::Saturation(Saturation::default())
    );
    assert_eq!(
        parse_effect_command(&["fade", "3"]).unwrap(),
        EffectCommand::Fade(Fade::with_curve(
            FadeCurve::Logarithmic,
            FrameCount::new(3),
            FrameCount::new(0),
        ))
    );
}

#[test]
fn sox_ng_fade_curve_types_parse_into_typed_configs() {
    let expected = [
        ("q", FadeCurve::QuarterSine),
        ("h", FadeCurve::HalfSine),
        ("l", FadeCurve::Logarithmic),
        ("t", FadeCurve::Linear),
        ("p", FadeCurve::InvertedParabola),
    ];

    for (token, curve) in expected {
        assert_eq!(
            parse_effect_command(&["fade", token, "3", "2"]).unwrap(),
            EffectCommand::Fade(Fade::with_stop_position(
                curve,
                FrameCount::new(3),
                FrameCount::new(2),
                FrameCount::new(3),
            ))
        );
    }
}

#[test]
fn sox_ng_fade_stop_position_and_fade_out_length_parse_into_typed_config() {
    assert_eq!(
        parse_effect_command(&["fade", "t", "4", "0"]).unwrap(),
        EffectCommand::Fade(Fade::with_stop_position(
            FadeCurve::Linear,
            FrameCount::new(4),
            FrameCount::new(0),
            FrameCount::new(4),
        ))
    );
    assert_eq!(
        parse_effect_command(&["fade", "t", "4", "12", "3"]).unwrap(),
        EffectCommand::Fade(Fade::with_stop_position(
            FadeCurve::Linear,
            FrameCount::new(4),
            FrameCount::new(12),
            FrameCount::new(3),
        ))
    );
    assert_eq!(
        parse_effect_command(&["fade", "t", "4", "-0", "3"]).unwrap(),
        EffectCommand::Fade(Fade::with_stop_position(
            FadeCurve::Linear,
            FrameCount::new(4),
            FrameCount::new(0),
            FrameCount::new(3),
        ))
    );
}

#[test]
fn parses_dc_shift_limiter_gain() {
    assert_eq!(
        parse_effect_command(&["dcshift", "0.5", "0.05"]).unwrap(),
        EffectCommand::DcShift(DcShift::with_limiter_gain(0.5, 0.05).unwrap())
    );
    assert_eq!(
        parse_effect_command(&["dcshift", "5e-1", "5e-2"])
            .unwrap()
            .render_tokens(),
        ["dcshift", "0.5", "0.05"]
    );
}

#[test]
fn parses_sox_ng_positioned_pad_arguments() {
    assert_eq!(
        parse_effect_command(&["pad", "1", "2@3", "4@5", "6"]).unwrap(),
        EffectCommand::Pad(
            Pad::with_positioned(
                FrameCount::new(1),
                FrameCount::new(6),
                [
                    PositionedPad::new(FrameCount::new(2), FrameCount::new(3)),
                    PositionedPad::new(FrameCount::new(4), FrameCount::new(5)),
                ],
            )
            .unwrap()
        )
    );
    assert_eq!(
        parse_effect_command(&["pad", "2@-0"]).unwrap(),
        EffectCommand::Pad(Pad::new(FrameCount::new(0), FrameCount::new(2)))
    );
    assert_eq!(
        parse_effect_command(&["pad", "1", "2@3", "4"])
            .unwrap()
            .render_tokens(),
        ["pad", "1", "2@3", "4"]
    );
}

#[test]
fn unsupported_options_name_the_effect_and_option() {
    let error = parse_effect_command(&["gain", "-q"]).unwrap_err();

    assert_eq!(
        error,
        EffectCommandParseError::UnsupportedOption {
            effect: "gain",
            option: "-q".to_owned(),
        }
    );
    assert_eq!(
        error.to_string(),
        "unsupported option `-q` for effect `gain`"
    );
}

#[test]
fn parses_gain_channel_equalize_and_balance_options() {
    assert_eq!(
        parse_effect_command(&["gain", "-e", "-3"]).unwrap(),
        EffectCommand::Gain(
            Gain::new(Decibels::new(-3.0).unwrap()).with_channel_mode(GainChannelMode::Equalize)
        )
    );
    assert_eq!(
        parse_effect_command(&["gain", "-B"]).unwrap(),
        EffectCommand::Gain(
            Gain::new(Decibels::new(0.0).unwrap()).with_channel_mode(GainChannelMode::Balance)
        )
    );
    assert_eq!(
        parse_effect_command(&["gain", "-bn", "-6"])
            .unwrap()
            .render_tokens(),
        ["gain", "-b", "-n", "-6"]
    );
}

#[test]
fn parses_gain_normalize_and_limiter_options() {
    assert_eq!(
        parse_effect_command(&["gain", "-n", "-3"]).unwrap(),
        EffectCommand::Gain(Gain::normalize(Decibels::new(-3.0).unwrap()))
    );
    assert_eq!(
        parse_effect_command(&["gain", "-l", "6"]).unwrap(),
        EffectCommand::Gain(Gain::limiter(Decibels::new(6.0).unwrap()))
    );
    assert_eq!(
        parse_effect_command(&["gain", "-nl", "6"])
            .unwrap()
            .render_tokens(),
        ["gain", "-n", "-l", "6"]
    );
}

#[test]
fn gain_rejects_sox_ng_option_combinations_that_are_mutually_exclusive() {
    let normalize_restore = parse_effect_command(&["gain", "-nr"]).unwrap_err();
    assert_eq!(
        normalize_restore,
        EffectCommandParseError::InvalidOptionCombination {
            effect: "gain",
            options: "only one of -n and -r may be given",
        }
    );

    let limiter_headroom = parse_effect_command(&["gain", "-lh", "-3"]).unwrap_err();
    assert_eq!(
        limiter_headroom,
        EffectCommandParseError::InvalidOptionCombination {
            effect: "gain",
            options: "only one of -l and -h may be given",
        }
    );

    let equalize_balance = parse_effect_command(&["gain", "-eB"]).unwrap_err();
    assert_eq!(
        equalize_balance,
        EffectCommandParseError::InvalidOptionCombination {
            effect: "gain",
            options: "only one of -e, -B, -b and -r may be given",
        }
    );
}

#[test]
fn parses_gain_headroom_and_reclaim_options() {
    assert_eq!(
        parse_effect_command(&["gain", "-h", "-6"]).unwrap(),
        EffectCommand::Gain(Gain::reserve_headroom(Decibels::new(-6.0).unwrap()))
    );
    assert_eq!(
        parse_effect_command(&["gain", "-r"]).unwrap(),
        EffectCommand::Gain(Gain::reclaim_headroom(Decibels::new(0.0).unwrap()))
    );
    assert_eq!(
        parse_effect_command(&["gain", "-rh", "-3"])
            .unwrap()
            .render_tokens(),
        ["gain", "-rh", "-3"]
    );
}

#[test]
fn missing_and_extra_arguments_are_rejected() {
    assert_eq!(
        parse_effect_command(&[]).unwrap_err(),
        EffectCommandParseError::EmptyCommand
    );
    assert_eq!(
        parse_effect_command(&["dcshift"]).unwrap_err(),
        EffectCommandParseError::MissingArgument {
            effect: "dcshift",
            argument: "shift",
        }
    );
    assert_eq!(
        parse_effect_command(&["reverse", "extra"]).unwrap_err(),
        EffectCommandParseError::UnexpectedArgument {
            effect: "reverse",
            argument: "extra".to_owned(),
        }
    );
}

#[test]
fn parses_sox_ng_trim_positions() {
    assert_eq!(
        parse_effect_command(&["trim", "2"]).unwrap(),
        EffectCommand::Trim(
            Trim::with_positions([TrimPosition::absolute(FrameCount::new(2))]).unwrap()
        )
    );
    assert_eq!(
        parse_effect_command(&["trim", "2", "4", "=10", "-2", "-0"]).unwrap(),
        EffectCommand::Trim(
            Trim::with_positions([
                TrimPosition::absolute(FrameCount::new(2)),
                TrimPosition::relative(FrameCount::new(4)),
                TrimPosition::absolute(FrameCount::new(10)),
                TrimPosition::before_end(FrameCount::new(2)),
                TrimPosition::End,
            ])
            .unwrap()
        )
    );
    assert_eq!(
        parse_effect_command(&["trim", "2s", "+4s", "=10s", "-2s", "-0"])
            .unwrap()
            .render_tokens(),
        ["trim", "2", "4", "=10", "-2", "-0"]
    );
}

#[test]
fn invalid_numeric_values_are_rejected_before_effect_construction() {
    let error = parse_effect_command(&["trim", "not-a-frame", "2"]).unwrap_err();

    assert!(matches!(
        error,
        EffectCommandParseError::InvalidFrameCount {
            effect: "trim",
            argument: "position",
            value,
            ..
        } if value == "not-a-frame"
    ));

    let error = parse_effect_command(&["gain", "not-a-number"]).unwrap_err();

    assert!(matches!(
        error,
        EffectCommandParseError::InvalidNumber {
            effect: "gain",
            argument: "gain-dB",
            value,
            ..
        } if value == "not-a-number"
    ));
}

#[test]
fn typed_effect_validation_errors_are_preserved() {
    let error = parse_effect_command(&["dcshift", "2.0001"]).unwrap_err();

    assert_eq!(
        error,
        EffectCommandParseError::InvalidEffectConfig {
            effect: "dcshift",
            argument: "shift",
            source: EffectError::InvalidDcShift,
        }
    );
}

#[test]
fn parsed_typed_configs_preserve_effect_behavior() {
    let command = parse_effect_command(&["gain", "-6"]).unwrap();
    let EffectCommand::Gain(gain) = command else {
        panic!("expected gain command");
    };
    let mut parsed = [0.25, -0.5, 1.0];
    let mut direct = parsed;

    gain.process_samples(&mut parsed);
    Gain::new(Decibels::new(-6.0).unwrap()).process_samples(&mut direct);

    for (parsed, direct) in parsed.iter().zip(direct) {
        assert_eq!(parsed.to_bits(), direct.to_bits());
    }
}

#[test]
fn equivalent_commands_render_to_identical_canonical_tokens() {
    let equivalent_commands = [
        (&["gain"][..], &["gain", "0.0"][..], &["gain", "0"][..]),
        (
            &["gain", "-h"][..],
            &["gain", "-h", "0"][..],
            &["gain", "-h", "0"][..],
        ),
        (
            &["contrast"][..],
            &["contrast", "75"][..],
            &["contrast", "75"][..],
        ),
        (
            &["gain-db", "1e0"][..],
            &["gain", "1"][..],
            &["gain", "1"][..],
        ),
        (
            &["dc-shift", "-0.0"][..],
            &["dcshift", "0"][..],
            &["dcshift", "0"][..],
        ),
        (
            &["dc-shift", "5e-1", "5e-2"][..],
            &["dcshift", "0.5", "0.05"][..],
            &["dcshift", "0.5", "0.05"][..],
        ),
        (
            &["saturation"][..],
            &["saturation", "tanh", "1", "0", "1"][..],
            &["saturation", "tanh", "1", "0", "1"][..],
        ),
        (&["pad"][..], &["pad", "0", "0"][..], &["pad", "0", "0"][..]),
        (
            &["fade", "t", "3"][..],
            &["fade", "t", "3"][..],
            &["fade", "t", "3"][..],
        ),
    ];

    for (left, right, expected) in equivalent_commands {
        let left = parse_effect_command(left).unwrap().render_tokens();
        let right = parse_effect_command(right).unwrap().render_tokens();
        assert_eq!(left, right);
        assert_eq!(left, expected);
    }
}

#[test]
fn command_rendering_uses_canonical_names_and_explicit_arguments() {
    let commands = [
        (
            parse_effect_command(&["trim", "12", "22"]).unwrap(),
            &["trim", "12", "22"][..],
        ),
        (
            parse_effect_command(&["reverse"]).unwrap(),
            &["reverse"][..],
        ),
        (
            parse_effect_command(&["fade", "q", "4", "2"]).unwrap(),
            &["fade", "q", "4", "2", "4"][..],
        ),
        (
            parse_effect_command(&["fade", "q", "4", "0", "2"]).unwrap(),
            &["fade", "q", "4", "0", "2"][..],
        ),
    ];

    for (command, expected) in commands {
        assert_eq!(command.render_tokens(), expected);
    }
}

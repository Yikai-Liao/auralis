//! Integration coverage for the public typed effect command parser.

use auralis_core::{ChannelCount, Decibels, FrameCount};
use auralis_effects::{
    Band, BandPass, BandReject, Bass, Biquad, BiquadCoefficients, BiquadWidth, Centercut, Channels,
    Contrast, DcShift, EffectCommand, Fade, FadeCurve, Gain, Oops, Pad, PositionedPad, Reverse,
    Saturation, SaturationType, SoftVol, Swap, Tremolo, Trim, parse_effect_command,
};

#[test]
fn parses_supported_effect_commands_into_typed_configs() {
    let expected = [
        (
            &["band", "-n", "1000", "2q"][..],
            EffectCommand::Band(Band::unpitched(1_000.0, Some(BiquadWidth::q(2.0))).unwrap()),
        ),
        (
            &["bandpass", "-c", "1000", "2q"][..],
            EffectCommand::BandPass(
                BandPass::constant_skirt(1_000.0, BiquadWidth::q(2.0)).unwrap(),
            ),
        ),
        (
            &["bandreject", "1000", "2q"][..],
            EffectCommand::BandReject(BandReject::new(1_000.0, BiquadWidth::q(2.0)).unwrap()),
        ),
        (
            &["bass", "6", "100", "0.5s"][..],
            EffectCommand::Bass(Bass::with_width(6.0, 100.0, BiquadWidth::slope(0.5)).unwrap()),
        ),
        (
            &["biquad", "2", "1", "0.5", "4", "-1", "0.25"][..],
            EffectCommand::Biquad(Biquad::new(
                BiquadCoefficients::normalized(0.5, 0.25, 0.125, -0.25, 0.0625).unwrap(),
            )),
        ),
        (
            &["contrast", "25"][..],
            EffectCommand::Contrast(Contrast::new(25.0).unwrap()),
        ),
        (
            &["channels", "2"][..],
            EffectCommand::Channels(Channels::new(ChannelCount::new(2).unwrap())),
        ),
        (
            &["centercut", "-a", "0.5", "-b", "-w", "16"][..],
            EffectCommand::Centercut(Centercut::with_options(0.5, true, 16).unwrap()),
        ),
        (
            &["gain", "-3"][..],
            EffectCommand::Gain(Gain::new(Decibels::new(-3.0).unwrap())),
        ),
        (
            &["dcshift", "0.25"][..],
            EffectCommand::DcShift(DcShift::new(0.25).unwrap()),
        ),
        (
            &["trim", "1", "2"][..],
            EffectCommand::Trim(Trim::new(FrameCount::new(1), FrameCount::new(3)).unwrap()),
        ),
        (
            &["pad", "2", "1"][..],
            EffectCommand::Pad(Pad::new(FrameCount::new(2), FrameCount::new(1))),
        ),
        (
            &["pad", "2@1"][..],
            EffectCommand::Pad(
                Pad::with_positioned(
                    FrameCount::new(0),
                    FrameCount::new(0),
                    [PositionedPad::new(FrameCount::new(2), FrameCount::new(1))],
                )
                .unwrap(),
            ),
        ),
        (&["reverse"][..], EffectCommand::Reverse(Reverse::new())),
        (&["oops"][..], EffectCommand::Oops(Oops::new())),
        (&["swap"][..], EffectCommand::Swap(Swap::new())),
        (
            &["softvol", "2", "10", "0.1"][..],
            EffectCommand::SoftVol(SoftVol::new(2.0, 10.0, 0.1).unwrap()),
        ),
        (
            &["tremolo", "5", "75"][..],
            EffectCommand::Tremolo(Tremolo::new(5.0, 75.0).unwrap()),
        ),
        (
            &["saturation", "sqrt", "0.75", "0.1", "0.25"][..],
            EffectCommand::Saturation(
                Saturation::new(SaturationType::Sqrt, 0.75, 0.1, 0.25).unwrap(),
            ),
        ),
        (
            &["fade", "t", "4", "2"][..],
            EffectCommand::Fade(Fade::with_stop_position(
                FadeCurve::Linear,
                FrameCount::new(4),
                FrameCount::new(2),
                FrameCount::new(4),
            )),
        ),
    ];

    for (tokens, command) in expected {
        assert_eq!(parse_effect_command(tokens).unwrap(), command);
        assert_eq!(parse_effect_command(tokens).unwrap().kind(), command.kind());
    }
}

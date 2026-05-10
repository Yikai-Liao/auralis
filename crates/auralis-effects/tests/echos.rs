//! Integration coverage for SoX-ng-style cascaded echos.

use auralis_core::{AudioBuffer, AudioSpec, ChannelCount, FrameCount, SampleFormat, SampleRate};
use auralis_effects::{
    Echo, EchoTap, Echos, EchosTap, EffectCommand, EffectCommandParseError, EffectError,
    parse_effect_chain, parse_effect_command,
};

#[test]
fn typed_echos_matches_chain_echos_command() {
    let source = mono_buffer(1_000, &[1.0, 0.0, 0.25]);
    let expected = Echos::new(0.5, 1.0, [EchosTap::new(1.0, 0.25).unwrap()])
        .unwrap()
        .process_buffer(&source)
        .unwrap();

    let mut chained = source;
    parse_effect_chain(&["echos", "0.5", "1", "1", "0.25"])
        .unwrap()
        .process_buffer(&mut chained)
        .unwrap();

    assert_eq!(chained, expected);
}

#[test]
fn multi_tap_echos_cascades_earlier_echoes() {
    let source = mono_buffer(1_000, &[1.0, 0.0, 0.0]);

    let echos = Echos::new(
        0.5,
        1.0,
        [
            EchosTap::new(1.0, 0.5).unwrap(),
            EchosTap::new(2.0, 0.25).unwrap(),
        ],
    )
    .unwrap()
    .process_buffer(&source)
    .unwrap();
    let equivalent_echo = Echo::new(
        0.5,
        1.0,
        [
            EchoTap::new(1.0, 0.5).unwrap(),
            EchoTap::new(2.0, 0.25).unwrap(),
            EchoTap::new(3.0, 0.25).unwrap(),
        ],
    )
    .unwrap()
    .process_buffer(&source)
    .unwrap();

    assert_eq!(echos, equivalent_echo);
}

#[test]
fn command_parser_accepts_multiple_delay_decay_pairs() {
    assert_eq!(
        parse_effect_command(&["echos", "0.8", "0.9", "1", "0.5", "2", "0.25"]).unwrap(),
        EffectCommand::Echos(
            Echos::new(
                0.8,
                0.9,
                [
                    EchosTap::new(1.0, 0.5).unwrap(),
                    EchosTap::new(2.0, 0.25).unwrap(),
                ],
            )
            .unwrap()
        )
    );
    assert_eq!(
        parse_effect_command(&["echos", "1.0", "0.5", "1.0", "0.25"])
            .unwrap()
            .render_tokens(),
        ["echos", "1", "0.5", "1", "0.25"]
    );
}

#[test]
fn delay_milliseconds_resolve_using_input_sample_rate() {
    let source = mono_buffer(2_000, &[1.0, 0.0]);

    let echoed = Echos::new(1.0, 1.0, [EchosTap::new(1.0, 0.5).unwrap()])
        .unwrap()
        .process_buffer(&source)
        .unwrap();

    assert_eq!(echoed.frames(), FrameCount::new(4));
    assert_eq!(echoed.as_planar_f32(), &[1.0, 0.0, 0.5, 0.0]);
}

#[test]
fn rejects_invalid_echos_config() {
    assert!(matches!(
        parse_effect_command(&["echos", "1", "1", "-1", "0.5"]).unwrap_err(),
        EffectCommandParseError::InvalidEffectConfig { .. }
    ));
    assert!(matches!(
        parse_effect_command(&["echos", "1", "1", "10", "1.5"]).unwrap_err(),
        EffectCommandParseError::InvalidEffectConfig { .. }
    ));
    assert!(matches!(
        parse_effect_command(&["echos", "1", "1", "10"]).unwrap_err(),
        EffectCommandParseError::MissingArgument { .. }
    ));

    let error = Echos::new(1.0, 1.0, []).unwrap_err();
    assert_eq!(error, EffectError::InvalidEchos);
}

fn mono_buffer(sample_rate_hz: u32, samples: &[f32]) -> AudioBuffer {
    let spec = AudioSpec::new(
        SampleRate::new(sample_rate_hz).unwrap(),
        ChannelCount::new(1).unwrap(),
        SampleFormat::Float32,
    );
    AudioBuffer::from_planar_f32(
        spec,
        FrameCount::new(u64::try_from(samples.len()).unwrap()),
        samples.to_vec(),
    )
    .unwrap()
}

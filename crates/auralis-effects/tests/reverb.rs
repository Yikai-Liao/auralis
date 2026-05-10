//! Integration coverage for SoX-ng-style reverb.

use auralis_core::{AudioBuffer, AudioSpec, ChannelCount, FrameCount, SampleFormat, SampleRate};
use auralis_effects::{
    EffectCommand, EffectCommandParseError, EffectError, Reverb, parse_effect_chain,
    parse_effect_command,
};

#[test]
fn typed_reverb_matches_chain_reverb_command() {
    let source = mono_buffer(44_100, &[1.0, 0.0, 0.25, -0.25]);
    let expected = Reverb::new(false, 50.0, 50.0, 100.0, 0.0, 0.0, 0.0)
        .unwrap()
        .process_buffer(&source)
        .unwrap();

    let mut chained = source;
    parse_effect_chain(&["reverb", "50", "50", "100", "0", "0", "0"])
        .unwrap()
        .process_buffer(&mut chained)
        .unwrap();

    assert_eq!(chained, expected);
}

#[test]
fn default_reverb_preserves_mono_channel_shape() {
    let source = mono_buffer(44_100, &[1.0, 0.0, 0.0, 0.0]);

    let processed = Reverb::with_defaults()
        .unwrap()
        .process_buffer(&source)
        .unwrap();

    assert_eq!(processed.channels(), ChannelCount::new(1).unwrap());
    assert_eq!(processed.frames(), source.frames());
    assert!(
        processed
            .as_planar_f32()
            .iter()
            .all(|sample| sample.is_finite())
    );
}

#[test]
fn wet_only_suppresses_the_dry_path() {
    let source = mono_buffer(44_100, &[1.0, 0.0, 0.0, 0.0]);
    let processed = Reverb::new(true, 50.0, 50.0, 100.0, 0.0, 0.0, 0.0)
        .unwrap()
        .process_buffer(&source)
        .unwrap();

    assert_eq!(
        processed.as_planar_f32(),
        &[0.0, 0.0, 0.0, 0.0],
        "short wet-only input should not include the dry impulse"
    );
}

#[test]
fn command_parser_accepts_wet_only_and_all_parameters() {
    assert_eq!(
        parse_effect_command(&["reverb", "-w", "75", "25", "50", "0", "10", "-3"]).unwrap(),
        EffectCommand::Reverb(Reverb::new(true, 75.0, 25.0, 50.0, 0.0, 10.0, -3.0).unwrap())
    );
}

#[test]
fn render_tokens_are_deterministic() {
    assert_eq!(
        parse_effect_command(&["reverb", "--wet-only", "75"])
            .unwrap()
            .render_tokens(),
        ["reverb", "-w", "75", "50", "100", "100", "0", "0"]
    );
}

#[test]
fn rejects_invalid_reverb_config() {
    assert!(matches!(
        parse_effect_command(&["reverb", "-x"]).unwrap_err(),
        EffectCommandParseError::UnexpectedArgument { .. }
    ));
    assert!(matches!(
        parse_effect_command(&["reverb", "101"]).unwrap_err(),
        EffectCommandParseError::InvalidEffectConfig { .. }
    ));

    let error = Reverb::new(false, 50.0, 50.0, 100.0, 100.0, 501.0, 0.0).unwrap_err();
    assert_eq!(error, EffectError::InvalidReverb);
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

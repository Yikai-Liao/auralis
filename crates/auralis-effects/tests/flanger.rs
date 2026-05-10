//! Integration coverage for SoX-ng-style flanger modulation.

use auralis_core::{AudioBuffer, AudioSpec, ChannelCount, FrameCount, SampleFormat, SampleRate};
use auralis_effects::{
    EffectCommand, EffectCommandParseError, EffectError, Flanger, FlangerInterpolation,
    FlangerWave, parse_effect_chain, parse_effect_command,
};

#[test]
fn typed_flanger_matches_chain_flanger_command() {
    let source = mono_buffer(1_000, &[1.0, 0.0, 0.25, -0.25]);
    let expected = Flanger::new(
        0.0,
        0.0,
        0.0,
        100.0,
        1.0,
        FlangerWave::Sine,
        0.0,
        FlangerInterpolation::Linear,
    )
    .unwrap()
    .process_buffer(&source)
    .unwrap();

    let mut chained = source;
    parse_effect_chain(&["flanger", "-l", "0", "0", "0", "100", "1", "sine", "0"])
        .unwrap()
        .process_buffer(&mut chained)
        .unwrap();

    assert_eq!(chained, expected);
}

#[test]
fn command_parser_accepts_flags_and_positional_modes() {
    assert_eq!(
        parse_effect_command(&[
            "flanger", "-q", "-t", "1", "2", "25", "100", "1", "sine", "50", "none",
        ])
        .unwrap(),
        EffectCommand::Flanger(
            Flanger::new(
                1.0,
                2.0,
                25.0,
                100.0,
                1.0,
                FlangerWave::Sine,
                50.0,
                FlangerInterpolation::None,
            )
            .unwrap()
        )
    );
}

#[test]
fn render_tokens_are_deterministic() {
    assert_eq!(
        parse_effect_command(&["flanger", "-n", "1", "2", "0", "71", "0.5"])
            .unwrap()
            .render_tokens(),
        ["flanger", "-n", "1", "2", "0", "71", "0.5", "sine", "25"]
    );
}

#[test]
fn triangle_modulation_produces_finite_length_preserving_output() {
    let source = stereo_buffer(1_000, &[1.0, 0.0, 0.25, -0.25], &[0.5, -0.5, 0.0, 0.25]);
    let flanger = Flanger::new(
        1.0,
        2.0,
        25.0,
        71.0,
        1.0,
        FlangerWave::Triangle,
        50.0,
        FlangerInterpolation::Quadratic,
    )
    .unwrap();

    let processed = flanger.process_buffer(&source).unwrap();

    assert_eq!(processed.frames(), source.frames());
    assert!(
        processed
            .as_planar_f32()
            .iter()
            .all(|sample| sample.is_finite())
    );
}

#[test]
fn rejects_invalid_flanger_config() {
    assert!(matches!(
        parse_effect_command(&["flanger", "-x"]).unwrap_err(),
        EffectCommandParseError::UnexpectedArgument { .. }
    ));
    assert!(matches!(
        parse_effect_command(&["flanger", "1001"]).unwrap_err(),
        EffectCommandParseError::InvalidEffectConfig { .. }
    ));

    let error = Flanger::new(
        0.0,
        0.0,
        101.0,
        71.0,
        1.0,
        FlangerWave::Sine,
        0.0,
        FlangerInterpolation::Linear,
    )
    .unwrap_err();
    assert_eq!(error, EffectError::InvalidFlanger);
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

fn stereo_buffer(sample_rate_hz: u32, left: &[f32], right: &[f32]) -> AudioBuffer {
    assert_eq!(left.len(), right.len());
    let spec = AudioSpec::new(
        SampleRate::new(sample_rate_hz).unwrap(),
        ChannelCount::new(2).unwrap(),
        SampleFormat::Float32,
    );
    let mut samples = left.to_vec();
    samples.extend_from_slice(right);
    AudioBuffer::from_planar_f32(
        spec,
        FrameCount::new(u64::try_from(left.len()).unwrap()),
        samples,
    )
    .unwrap()
}

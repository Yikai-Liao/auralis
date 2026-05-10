//! Integration coverage for SoX-ng-style chorus modulation.

use auralis_core::{AudioBuffer, AudioSpec, ChannelCount, FrameCount, SampleFormat, SampleRate};
use auralis_effects::{
    Chorus, ChorusInterpolation, ChorusStage, ChorusWave, EffectCommand, EffectCommandParseError,
    EffectError, parse_effect_chain, parse_effect_command,
};

#[test]
fn typed_chorus_matches_chain_chorus_command() {
    let source = mono_buffer(1_000, &[1.0, 0.0, 0.25]);
    let expected = Chorus::with_options(
        0.5,
        1.0,
        [ChorusStage::new(1.0, 0.25, 1.0, 0.0).unwrap()],
        ChorusInterpolation::None,
    )
    .unwrap()
    .process_buffer(&source)
    .unwrap();

    let mut chained = source;
    parse_effect_chain(&["chorus", "0.5", "1", "1", "0.25", "1", "0"])
        .unwrap()
        .process_buffer(&mut chained)
        .unwrap();

    assert_eq!(chained, expected);
}

#[test]
fn command_parser_accepts_interpolation_wave_and_multiple_stages() {
    assert_eq!(
        parse_effect_command(&[
            "chorus", "-q", "-t", "0.6", "0.8", "1", "0.25", "1", "0", "2", "-0.125", "1", "0",
            "-sine",
        ])
        .unwrap(),
        EffectCommand::Chorus(
            Chorus::with_options(
                0.6,
                0.8,
                [
                    ChorusStage::with_wave(1.0, 0.25, 1.0, 0.0, ChorusWave::Triangle).unwrap(),
                    ChorusStage::with_wave(2.0, -0.125, 1.0, 0.0, ChorusWave::Sine).unwrap(),
                ],
                ChorusInterpolation::Quadratic,
            )
            .unwrap()
        )
    );
}

#[test]
fn render_tokens_are_deterministic() {
    assert_eq!(
        parse_effect_command(&["chorus", "-l", "0.5", "1", "1", "0.25", "1", "0"])
            .unwrap()
            .render_tokens(),
        ["chorus", "-l", "0.5", "1", "1", "0.25", "1", "0"]
    );
}

#[test]
fn triangle_modulation_produces_finite_output() {
    let source = mono_buffer(1_000, &[1.0, 0.0, 0.25, -0.25]);
    let chorus = Chorus::with_options(
        0.5,
        1.0,
        [ChorusStage::with_wave(1.0, 0.25, 1.0, 2.0, ChorusWave::Triangle).unwrap()],
        ChorusInterpolation::Linear,
    )
    .unwrap();

    let processed = chorus.process_buffer(&source).unwrap();

    assert_eq!(processed.frames(), FrameCount::new(9));
    assert!(
        processed
            .as_planar_f32()
            .iter()
            .all(|sample| sample.is_finite())
    );
}

#[test]
fn rejects_invalid_chorus_config() {
    assert!(matches!(
        parse_effect_command(&["chorus", "-x"]).unwrap_err(),
        EffectCommandParseError::UnexpectedArgument { .. }
    ));
    assert!(matches!(
        parse_effect_command(&["chorus", "1.5"]).unwrap_err(),
        EffectCommandParseError::InvalidEffectConfig { .. }
    ));

    let error = Chorus::with_options(1.0, 1.0, [], ChorusInterpolation::None).unwrap_err();
    assert_eq!(error, EffectError::InvalidChorus);
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

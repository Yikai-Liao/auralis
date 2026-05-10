//! Integration coverage for SoX-ng-style phaser modulation.

use auralis_core::{AudioBuffer, AudioSpec, ChannelCount, FrameCount, SampleFormat, SampleRate};
use auralis_effects::{
    EffectCommand, EffectCommandParseError, EffectError, Phaser, PhaserInterpolation, PhaserWave,
    parse_effect_chain, parse_effect_command,
};

#[test]
fn typed_phaser_matches_chain_phaser_command() {
    let source = mono_buffer(1_000, &[1.0, 0.0, 0.25, -0.25, 0.0]);
    let expected = Phaser::new(
        0.4,
        0.74,
        3.0,
        0.4,
        0.5,
        PhaserWave::Sine,
        PhaserInterpolation::Linear,
    )
    .unwrap()
    .process_buffer(&source)
    .unwrap();

    let mut chained = source;
    parse_effect_chain(&["phaser", "-l", "0.4", "0.74", "3", "0.4", "0.5"])
        .unwrap()
        .process_buffer(&mut chained)
        .unwrap();

    assert_eq!(chained, expected);
}

#[test]
fn command_parser_accepts_flags_and_trailing_wave_option() {
    assert_eq!(
        parse_effect_command(&["phaser", "-q", "-t", "0.8", "0.74", "3", "0.4", "0.5", "-s"])
            .unwrap(),
        EffectCommand::Phaser(
            Phaser::new(
                0.8,
                0.74,
                3.0,
                0.4,
                0.5,
                PhaserWave::Sine,
                PhaserInterpolation::Quadratic,
            )
            .unwrap()
        )
    );
}

#[test]
fn render_tokens_are_deterministic() {
    assert_eq!(
        parse_effect_command(&["phaser", "-n", "0.4", "0.74", "3", "0.4", "0.5"])
            .unwrap()
            .render_tokens(),
        ["phaser", "0.4", "0.74", "3", "0.4", "0.5"]
    );
}

#[test]
fn triangle_modulation_produces_finite_length_preserving_output() {
    let source = stereo_buffer(1_000, &[1.0, 0.0, 0.25, -0.25], &[0.5, -0.5, 0.0, 0.25]);
    let phaser = Phaser::new(
        0.4,
        0.74,
        3.0,
        0.4,
        1.0,
        PhaserWave::Triangle,
        PhaserInterpolation::Quadratic,
    )
    .unwrap();

    let processed = phaser.process_buffer(&source).unwrap();

    assert_eq!(processed.frames(), source.frames());
    assert!(
        processed
            .as_planar_f32()
            .iter()
            .all(|sample| sample.is_finite())
    );
}

#[test]
fn rejects_invalid_phaser_config() {
    assert!(matches!(
        parse_effect_command(&["phaser", "-x"]).unwrap_err(),
        EffectCommandParseError::UnexpectedArgument { .. }
    ));
    assert!(matches!(
        parse_effect_command(&["phaser", "1.1"]).unwrap_err(),
        EffectCommandParseError::InvalidEffectConfig { .. }
    ));

    let error = Phaser::new(
        0.4,
        0.74,
        3.0,
        1.1,
        1.0,
        PhaserWave::Sine,
        PhaserInterpolation::None,
    )
    .unwrap_err();
    assert_eq!(error, EffectError::InvalidPhaser);
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

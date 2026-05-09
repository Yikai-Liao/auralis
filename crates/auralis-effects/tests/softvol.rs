//! Integration coverage for SoX-ng-style soft volume control.

use auralis_core::{AudioBuffer, AudioSpec, ChannelCount, FrameCount, SampleFormat, SampleRate};
use auralis_effects::{
    EffectChainParseError, EffectCommand, EffectCommandParseError, EffectError, SoftVol,
    parse_effect_chain,
};

#[test]
fn softvol_chain_parses_defaults_and_explicit_arguments() {
    let default = parse_effect_chain(&["softvol", "reverse"]).unwrap();

    assert_eq!(
        default.render_tokens(),
        ["softvol", "1", "0", "0", "reverse"]
    );
    assert_eq!(
        default.commands()[0],
        EffectCommand::SoftVol(SoftVol::default())
    );

    let explicit = parse_effect_chain(&["softvol", "2", "10", "0.1"]).unwrap();
    assert_eq!(explicit.render_tokens(), ["softvol", "2", "10", "0.1"]);
    assert_eq!(
        explicit.commands()[0],
        EffectCommand::SoftVol(SoftVol::new(2.0, 10.0, 0.1).unwrap())
    );
}

#[test]
fn softvol_reduces_current_volume_to_avoid_clipping() {
    let mut audio = mono_audio_buffer(vec![0.25, 0.75, -1.0]);

    parse_effect_chain(&["softvol", "2"])
        .unwrap()
        .process_buffer(&mut audio)
        .unwrap();

    assert_samples_close(audio.as_planar_f32(), &[0.5, 1.0, -1.0]);
}

#[test]
fn softvol_recovery_uses_sample_rate_and_double_time() {
    let mut audio = mono_audio_buffer_at_rate(vec![1.0, 0.25, 0.25], 1);

    parse_effect_chain(&["softvol", "2", "1"])
        .unwrap()
        .process_buffer(&mut audio)
        .unwrap();

    assert_samples_close(audio.as_planar_f32(), &[1.0, 0.5, 1.0]);
}

#[test]
fn softvol_stereo_uses_frame_peak_across_channels() {
    let mut audio = stereo_audio_buffer(vec![0.25, 0.25, 1.0, 0.25]);

    parse_effect_chain(&["softvol", "2"])
        .unwrap()
        .process_buffer(&mut audio)
        .unwrap();

    assert_samples_close(audio.as_planar_f32(), &[0.25, 0.25, 1.0, 0.25]);
}

#[test]
fn softvol_rejects_invalid_values_and_extra_arguments() {
    let invalid = parse_effect_chain(&["softvol", "-1"]).unwrap_err();
    assert!(matches!(
        invalid,
        EffectChainParseError::CommandParseFailed {
            source: EffectCommandParseError::InvalidEffectConfig {
                effect: "softvol",
                argument: "softvol",
                source: EffectError::InvalidSoftVol,
            },
            ..
        }
    ));

    let extra = parse_effect_chain(&["softvol", "1", "0", "0", "extra"]).unwrap_err();
    assert!(matches!(
        extra,
        EffectChainParseError::CommandParseFailed {
            source: EffectCommandParseError::UnexpectedArgument {
                effect: "softvol",
                argument,
            },
            ..
        } if argument == "extra"
    ));
}

fn mono_audio_buffer(samples: Vec<f32>) -> AudioBuffer {
    mono_audio_buffer_at_rate(samples, 48_000)
}

fn mono_audio_buffer_at_rate(samples: Vec<f32>, sample_rate: u32) -> AudioBuffer {
    let spec = AudioSpec::new(
        SampleRate::new(sample_rate).unwrap(),
        ChannelCount::new(1).unwrap(),
        SampleFormat::Float32,
    );
    AudioBuffer::from_planar_f32(
        spec,
        FrameCount::new(u64::try_from(samples.len()).unwrap()),
        samples,
    )
    .unwrap()
}

fn stereo_audio_buffer(samples: Vec<f32>) -> AudioBuffer {
    assert_eq!(samples.len() % 2, 0);
    let spec = AudioSpec::new(
        SampleRate::new(48_000).unwrap(),
        ChannelCount::new(2).unwrap(),
        SampleFormat::Float32,
    );
    AudioBuffer::from_planar_f32(
        spec,
        FrameCount::new(u64::try_from(samples.len() / 2).unwrap()),
        samples,
    )
    .unwrap()
}

fn assert_samples_close(actual: &[f32], expected: &[f32]) {
    assert_eq!(actual.len(), expected.len());
    for (index, (&actual, &expected)) in actual.iter().zip(expected).enumerate() {
        assert!(
            (actual - expected).abs() <= 0.000_001,
            "sample {index}: actual={actual}, expected={expected}"
        );
    }
}

//! Integration coverage for SoX-ng-style tremolo modulation.

use auralis_core::{AudioSpec, ChannelCount, FrameCount, SampleFormat, SampleRate};
use auralis_effects::{
    EffectChainParseError, EffectCommand, EffectCommandParseError, EffectError, Reverse, Tremolo,
    parse_effect_chain,
};

#[test]
fn tremolo_chain_parses_default_and_explicit_depth() {
    let default = parse_effect_chain(&["tremolo", "5", "reverse"]).unwrap();
    assert_eq!(
        default.commands(),
        &[
            EffectCommand::Tremolo(Tremolo::with_default_depth(5.0).unwrap()),
            EffectCommand::Reverse(Reverse::new())
        ]
    );
    assert_eq!(default.render_tokens(), ["tremolo", "5", "40", "reverse"]);

    let explicit = parse_effect_chain(&["tremolo", "5", "75"]).unwrap();
    assert_eq!(
        explicit.commands(),
        &[EffectCommand::Tremolo(Tremolo::new(5.0, 75.0).unwrap())]
    );
    assert_eq!(explicit.render_tokens(), ["tremolo", "5", "75"]);
}

#[test]
fn tremolo_uses_sox_ng_sine_fmod_envelope() {
    let mut audio = mono_audio(vec![1.0; 5], 8);

    parse_effect_chain(&["tremolo", "1", "80"])
        .unwrap()
        .process_buffer(&mut audio)
        .unwrap();

    assert_samples_close(
        audio.as_planar_f32(),
        &[1.0, 0.882_842_7, 0.6, 0.317_157_3, 0.2],
    );
}

#[test]
fn tremolo_stereo_uses_shared_frame_phase_for_each_channel() {
    let mut audio = stereo_audio(vec![1.0, 1.0, 1.0, 1.0], vec![0.5, 0.5, 0.5, 0.5], 4);

    Tremolo::new(1.0, 40.0).unwrap().process_buffer(&mut audio);

    assert_samples_close(
        audio.as_planar_f32(),
        &[1.0, 0.8, 0.6, 0.8, 0.5, 0.4, 0.3, 0.4],
    );
}

#[test]
fn tremolo_rejects_invalid_values_and_extra_arguments() {
    let missing = parse_effect_chain(&["tremolo"]).unwrap_err();
    assert!(matches!(
        missing,
        EffectChainParseError::CommandParseFailed {
            source: EffectCommandParseError::MissingArgument {
                effect: "tremolo",
                argument: "speed",
            },
            ..
        }
    ));

    let invalid = parse_effect_chain(&["tremolo", "1", "0"]).unwrap_err();
    assert!(matches!(
        invalid,
        EffectChainParseError::CommandParseFailed {
            source: EffectCommandParseError::InvalidEffectConfig {
                effect: "tremolo",
                argument: "tremolo",
                source: EffectError::InvalidTremolo,
            },
            ..
        }
    ));

    let extra = parse_effect_chain(&["tremolo", "1", "40", "extra"]).unwrap_err();
    assert!(matches!(
        extra,
        EffectChainParseError::CommandParseFailed {
            source: EffectCommandParseError::UnexpectedArgument {
                effect: "tremolo",
                argument,
            },
            ..
        } if argument == "extra"
    ));
}

fn mono_audio(samples: Vec<f32>, sample_rate: u32) -> auralis_core::AudioBuffer {
    let frames = u64::try_from(samples.len()).unwrap();
    let spec = AudioSpec::new(
        SampleRate::new(sample_rate).unwrap(),
        ChannelCount::new(1).unwrap(),
        SampleFormat::Float32,
    );
    auralis_core::AudioBuffer::from_planar_f32(spec, FrameCount::new(frames), samples).unwrap()
}

fn stereo_audio(left: Vec<f32>, right: Vec<f32>, sample_rate: u32) -> auralis_core::AudioBuffer {
    assert_eq!(left.len(), right.len());
    let frames = u64::try_from(left.len()).unwrap();
    let spec = AudioSpec::new(
        SampleRate::new(sample_rate).unwrap(),
        ChannelCount::new(2).unwrap(),
        SampleFormat::Float32,
    );
    let samples = left.into_iter().chain(right).collect();
    auralis_core::AudioBuffer::from_planar_f32(spec, FrameCount::new(frames), samples).unwrap()
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

//! Integration coverage for SoX-ng-style loudness compensation.

use auralis_core::{AudioBuffer, AudioSpec, ChannelCount, FrameCount, SampleFormat, SampleRate};
use auralis_effects::{
    EffectChainParseError, EffectCommand, EffectCommandParseError, EffectError, Loudness,
    parse_effect_chain,
};

#[test]
fn loudness_chain_parses_defaults_and_explicit_arguments() {
    let default = parse_effect_chain(&["loudness", "reverse"]).unwrap();

    assert_eq!(
        default.render_tokens(),
        ["loudness", "-10", "65", "1023", "reverse"]
    );
    assert_eq!(
        default.commands()[0],
        EffectCommand::Loudness(Loudness::default())
    );

    let explicit = parse_effect_chain(&["loudness", "-6", "70", "127"]).unwrap();
    assert_eq!(explicit.render_tokens(), ["loudness", "-6", "70", "127"]);
    assert_eq!(
        explicit.commands()[0],
        EffectCommand::Loudness(Loudness::new(-6.0, 70.0, 127).unwrap())
    );
}

#[test]
fn loudness_zero_gain_is_identity() {
    let mut audio = mono_audio_buffer(vec![0.25, 0.0, -0.25]);

    parse_effect_chain(&["loudness", "0"])
        .unwrap()
        .process_buffer(&mut audio)
        .unwrap();

    assert_eq!(audio.as_planar_f32(), &[0.25, 0.0, -0.25]);
}

#[test]
fn loudness_processes_stereo_without_changing_shape() {
    let mut audio = stereo_audio_buffer(vec![0.5, -0.25, 0.25, -0.5]);

    parse_effect_chain(&["loudness", "-6", "65", "127"])
        .unwrap()
        .process_buffer(&mut audio)
        .unwrap();

    assert_eq!(audio.channels().as_usize(), 2);
    assert_eq!(audio.frames().as_u64(), 2);
    assert!(
        audio
            .as_planar_f32()
            .iter()
            .all(|sample| sample.is_finite())
    );
}

#[test]
fn loudness_rejects_invalid_values_and_extra_arguments() {
    let invalid = parse_effect_chain(&["loudness", "-51"]).unwrap_err();
    assert!(matches!(
        invalid,
        EffectChainParseError::CommandParseFailed {
            source: EffectCommandParseError::InvalidEffectConfig {
                effect: "loudness",
                argument: "loudness",
                source: EffectError::InvalidLoudness,
            },
            ..
        }
    ));

    let extra = parse_effect_chain(&["loudness", "-10", "65", "1023", "extra"]).unwrap_err();
    assert!(matches!(
        extra,
        EffectChainParseError::CommandParseFailed {
            source: EffectCommandParseError::UnexpectedArgument {
                effect: "loudness",
                argument,
            },
            ..
        } if argument == "extra"
    ));
}

fn mono_audio_buffer(samples: Vec<f32>) -> AudioBuffer {
    audio_buffer(samples, 1)
}

fn stereo_audio_buffer(samples: Vec<f32>) -> AudioBuffer {
    audio_buffer(samples, 2)
}

fn audio_buffer(samples: Vec<f32>, channels: u16) -> AudioBuffer {
    let spec = AudioSpec::new(
        SampleRate::new(48_000).unwrap(),
        ChannelCount::new(channels).unwrap(),
        SampleFormat::Float32,
    );
    AudioBuffer::from_planar_f32(
        spec,
        FrameCount::new(u64::try_from(samples.len() / usize::from(channels)).unwrap()),
        samples,
    )
    .unwrap()
}

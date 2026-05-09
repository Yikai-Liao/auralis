//! Integration coverage for SoX-ng-style finite repeat.

use auralis_core::{AudioSpec, ChannelCount, FrameCount, SampleFormat, SampleRate};
use auralis_effects::{
    EffectChainParseError, EffectCommand, EffectCommandParseError, EffectError, Repeat, Reverse,
    parse_effect_chain,
};

#[test]
fn repeat_chain_parses_default_and_explicit_count() {
    let default = parse_effect_chain(&["repeat", "reverse"]).unwrap();
    assert_eq!(
        default.commands(),
        &[
            EffectCommand::Repeat(Repeat::default()),
            EffectCommand::Reverse(Reverse::new())
        ]
    );
    assert_eq!(default.render_tokens(), ["repeat", "1", "reverse"]);

    let explicit = parse_effect_chain(&["repeat", "2"]).unwrap();
    assert_eq!(
        explicit.commands(),
        &[EffectCommand::Repeat(Repeat::new(2).unwrap())]
    );
    assert_eq!(explicit.render_tokens(), ["repeat", "2"]);
}

#[test]
fn repeat_appends_planar_copies_and_preserves_metadata() {
    let mut audio = stereo_audio(vec![0.0, 0.25], vec![-0.5, 0.5]);

    parse_effect_chain(&["repeat", "2"])
        .unwrap()
        .process_buffer(&mut audio)
        .unwrap();

    assert_eq!(audio.frames(), FrameCount::new(6));
    assert_eq!(audio.channels(), ChannelCount::new(2).unwrap());
    assert_eq!(
        audio.as_planar_f32(),
        &[
            0.0, 0.25, 0.0, 0.25, 0.0, 0.25, -0.5, 0.5, -0.5, 0.5, -0.5, 0.5
        ]
    );
}

#[test]
fn repeat_zero_is_identity_in_chain() {
    let mut audio = mono_audio(vec![-0.5, 0.0, 0.5]);

    parse_effect_chain(&["repeat", "0"])
        .unwrap()
        .process_buffer(&mut audio)
        .unwrap();

    assert_eq!(audio.as_planar_f32(), &[-0.5, 0.0, 0.5]);
    assert_eq!(audio.frames(), FrameCount::new(3));
}

#[test]
fn repeat_rejects_indefinite_and_out_of_range_counts() {
    let indefinite = parse_effect_chain(&["repeat", "-"]).unwrap_err();
    assert!(matches!(
        indefinite,
        EffectChainParseError::CommandParseFailed {
            source: EffectCommandParseError::UnsupportedOption {
                effect: "repeat",
                option,
            },
            ..
        } if option == "-"
    ));

    let out_of_range = parse_effect_chain(&["repeat", "4294967295"]).unwrap_err();
    assert!(matches!(
        out_of_range,
        EffectChainParseError::CommandParseFailed {
            source: EffectCommandParseError::InvalidEffectConfig {
                effect: "repeat",
                argument: "repeat",
                source: EffectError::InvalidRepeatCount,
            },
            ..
        }
    ));
}

fn mono_audio(samples: Vec<f32>) -> auralis_core::AudioBuffer {
    audio_buffer(1, samples)
}

fn stereo_audio(left: Vec<f32>, right: Vec<f32>) -> auralis_core::AudioBuffer {
    assert_eq!(left.len(), right.len());
    audio_buffer(2, left.into_iter().chain(right).collect())
}

fn audio_buffer(channels: u16, samples: Vec<f32>) -> auralis_core::AudioBuffer {
    let channels = ChannelCount::new(channels).unwrap();
    let frames = samples.len() / channels.as_usize();
    let spec = AudioSpec::new(
        SampleRate::new(48_000).unwrap(),
        channels,
        SampleFormat::Float32,
    );
    auralis_core::AudioBuffer::from_planar_f32(
        spec,
        FrameCount::new(u64::try_from(frames).unwrap()),
        samples,
    )
    .unwrap()
}

//! Integration coverage for SoX-ng-style explicit channels conversion.

use auralis_core::{AudioSpec, ChannelCount, FrameCount, SampleFormat, SampleRate};
use auralis_effects::{
    Channels, EffectChainParseError, EffectCommand, EffectCommandParseError, parse_effect_chain,
};

#[test]
fn channels_chain_parses_and_renders_target_count() {
    let chain = parse_effect_chain(&["channels", "2"]).unwrap();

    assert_eq!(
        chain.commands(),
        &[EffectCommand::Channels(Channels::new(
            ChannelCount::new(2).unwrap()
        ))]
    );
    assert_eq!(chain.render_tokens(), ["channels", "2"]);
}

#[test]
fn channels_downmixes_stereo_to_mono_in_chain() {
    let mut audio = stereo_audio(vec![0.25, -0.5], vec![0.75, 0.5]);

    parse_effect_chain(&["channels", "1"])
        .unwrap()
        .process_buffer(&mut audio)
        .unwrap();

    assert_eq!(audio.channels(), ChannelCount::new(1).unwrap());
    assert_eq!(audio.frames(), FrameCount::new(2));
    assert_eq!(audio.as_planar_f32(), &[0.5, 0.0]);
}

#[test]
fn channels_upmixes_mono_to_stereo_in_chain() {
    let mut audio = mono_audio(vec![0.25, -0.5, 0.75]);

    parse_effect_chain(&["channels", "2"])
        .unwrap()
        .process_buffer(&mut audio)
        .unwrap();

    assert_eq!(audio.channels(), ChannelCount::new(2).unwrap());
    assert_eq!(audio.as_planar_f32(), &[0.25, -0.5, 0.75, 0.25, -0.5, 0.75]);
}

#[test]
fn channels_rejects_missing_zero_and_extra_arguments() {
    let missing = parse_effect_chain(&["channels"]).unwrap_err();
    assert!(matches!(
        missing,
        EffectChainParseError::CommandParseFailed {
            source: EffectCommandParseError::MissingArgument {
                effect: "channels",
                argument: "channels"
            },
            ..
        }
    ));

    let zero = parse_effect_chain(&["channels", "0"]).unwrap_err();
    assert!(matches!(
        zero,
        EffectChainParseError::CommandParseFailed {
            source: EffectCommandParseError::InvalidCoreValue {
                effect: "channels",
                argument: "channels",
                ..
            },
            ..
        }
    ));

    let extra = parse_effect_chain(&["channels", "1", "extra"]).unwrap_err();
    assert!(matches!(
        extra,
        EffectChainParseError::CommandParseFailed {
            source: EffectCommandParseError::UnexpectedArgument {
                effect: "channels",
                argument,
            },
            ..
        } if argument == "extra"
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

//! Integration coverage for SoX-ng-style rate scaffolding.

use auralis_core::{AudioSpec, ChannelCount, FrameCount, SampleFormat, SampleRate};
use auralis_effects::{
    EffectChainParseError, EffectCommand, EffectCommandParseError, Rate, Reverse,
    parse_effect_chain,
};

#[test]
fn rate_chain_parses_integer_and_kilohertz_frequency() {
    let integer = parse_effect_chain(&["rate", "24000", "reverse"]).unwrap();
    assert_eq!(
        integer.commands(),
        &[
            EffectCommand::Rate(Rate::new(SampleRate::new(24_000).unwrap())),
            EffectCommand::Reverse(Reverse::new())
        ]
    );
    assert_eq!(integer.render_tokens(), ["rate", "24000", "reverse"]);

    let kilohertz = parse_effect_chain(&["rate", "44.1k"]).unwrap();
    assert_eq!(kilohertz.render_tokens(), ["rate", "44100"]);
}

#[test]
fn rate_chain_parses_quick_and_low_quality_modes() {
    let quick = parse_effect_chain(&["rate", "-q", "24000"]).unwrap();
    assert_eq!(
        quick.commands(),
        &[EffectCommand::Rate(Rate::quick(
            SampleRate::new(24_000).unwrap()
        ))]
    );
    assert_eq!(quick.render_tokens(), ["rate", "-q", "24000"]);

    let low = parse_effect_chain(&["rate", "-Q", "1", "44.1k"]).unwrap();
    assert_eq!(
        low.commands(),
        &[EffectCommand::Rate(Rate::low(
            SampleRate::new(44_100).unwrap()
        ))]
    );
    assert_eq!(low.render_tokens(), ["rate", "-l", "44100"]);
}

#[test]
fn rate_changes_sample_rate_and_resamples_decoded_audio() {
    let mut audio = stereo_audio(vec![0.0, 1.0, 0.0], vec![1.0, 0.0, -1.0]);

    parse_effect_chain(&["rate", "96000"])
        .unwrap()
        .process_buffer(&mut audio)
        .unwrap();

    assert_eq!(audio.frames(), FrameCount::new(6));
    assert_eq!(audio.spec().sample_rate().as_u32(), 96_000);
    assert_eq!(
        audio.as_planar_f32(),
        &[
            0.0, 0.5, 1.0, 0.5, 0.0, 0.0, 1.0, 0.5, 0.0, -0.5, -1.0, -1.0
        ]
    );
}

#[test]
fn rate_matching_target_is_identity_in_chain() {
    let mut audio = mono_audio(vec![-0.5, 0.0, 0.5]);

    parse_effect_chain(&["rate", "48000"])
        .unwrap()
        .process_buffer(&mut audio)
        .unwrap();

    assert_eq!(audio.as_planar_f32(), &[-0.5, 0.0, 0.5]);
    assert_eq!(audio.frames(), FrameCount::new(3));
    assert_eq!(audio.spec().sample_rate().as_u32(), 48_000);
}

#[test]
fn rate_rejects_missing_invalid_options() {
    let missing = parse_effect_chain(&["rate"]).unwrap_err();
    assert!(matches!(
        missing,
        EffectChainParseError::CommandParseFailed {
            source: EffectCommandParseError::MissingArgument {
                effect: "rate",
                argument: "frequency",
            },
            ..
        }
    ));

    let invalid = parse_effect_chain(&["rate", "0"]).unwrap_err();
    assert!(matches!(
        invalid,
        EffectChainParseError::CommandParseFailed {
            source: EffectCommandParseError::InvalidCoreValue {
                effect: "rate",
                argument: "frequency",
                source: auralis_core::AuralisError::InvalidSampleRate,
            },
            ..
        }
    ));

    let unsupported_quality = parse_effect_chain(&["rate", "-m", "24000"]).unwrap_err();
    assert!(matches!(
        unsupported_quality,
        EffectChainParseError::CommandParseFailed {
            source: EffectCommandParseError::UnsupportedOption { effect: "rate", .. },
            ..
        }
    ));

    let higher_quality = parse_effect_chain(&["rate", "-Q", "4", "24000"]).unwrap_err();
    assert!(matches!(
        higher_quality,
        EffectChainParseError::CommandParseFailed {
            source: EffectCommandParseError::UnsupportedOption { effect: "rate", .. },
            ..
        }
    ));
}

fn mono_audio(samples: Vec<f32>) -> auralis_core::AudioBuffer {
    audio_buffer(1, 48_000, samples)
}

fn stereo_audio(left: Vec<f32>, right: Vec<f32>) -> auralis_core::AudioBuffer {
    assert_eq!(left.len(), right.len());
    audio_buffer(2, 48_000, left.into_iter().chain(right).collect())
}

fn audio_buffer(channels: u16, sample_rate: u32, samples: Vec<f32>) -> auralis_core::AudioBuffer {
    let channels = ChannelCount::new(channels).unwrap();
    let frames = samples.len() / channels.as_usize();
    let spec = AudioSpec::new(
        SampleRate::new(sample_rate).unwrap(),
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

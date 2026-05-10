//! Integration coverage for SoX-ng-style upsample.

use auralis_core::{AudioSpec, ChannelCount, FrameCount, SampleFormat, SampleRate};
use auralis_effects::{
    EffectChainParseError, EffectCommand, EffectCommandParseError, EffectError, Reverse, Upsample,
    parse_effect_chain,
};

#[test]
fn upsample_chain_parses_default_and_explicit_factor() {
    let default = parse_effect_chain(&["upsample", "reverse"]).unwrap();
    assert_eq!(
        default.commands(),
        &[
            EffectCommand::Upsample(Upsample::default()),
            EffectCommand::Reverse(Reverse::new())
        ]
    );
    assert_eq!(default.render_tokens(), ["upsample", "2", "reverse"]);

    let explicit = parse_effect_chain(&["upsample", "3"]).unwrap();
    assert_eq!(
        explicit.commands(),
        &[EffectCommand::Upsample(Upsample::new(3).unwrap())]
    );
    assert_eq!(explicit.render_tokens(), ["upsample", "3"]);
}

#[test]
fn upsample_inserts_zero_frames_and_changes_sample_rate() {
    let mut audio = stereo_audio(vec![0.25, -0.5], vec![0.75, 1.0]);

    parse_effect_chain(&["upsample", "2"])
        .unwrap()
        .process_buffer(&mut audio)
        .unwrap();

    assert_eq!(audio.frames(), FrameCount::new(4));
    assert_eq!(audio.spec().sample_rate().as_u32(), 96_000);
    assert_eq!(
        audio.as_planar_f32(),
        &[0.25, 0.0, -0.5, 0.0, 0.75, 0.0, 1.0, 0.0]
    );
}

#[test]
fn upsample_supports_factor_larger_than_two() {
    let mut audio = mono_audio(vec![0.25, -0.5]);

    parse_effect_chain(&["upsample", "3"])
        .unwrap()
        .process_buffer(&mut audio)
        .unwrap();

    assert_eq!(audio.frames(), FrameCount::new(6));
    assert_eq!(audio.spec().sample_rate().as_u32(), 144_000);
    assert_eq!(audio.as_planar_f32(), &[0.25, 0.0, 0.0, -0.5, 0.0, 0.0]);
}

#[test]
fn upsample_factor_one_is_identity_in_chain() {
    let mut audio = mono_audio(vec![-0.5, 0.0, 0.5]);

    parse_effect_chain(&["upsample", "1"])
        .unwrap()
        .process_buffer(&mut audio)
        .unwrap();

    assert_eq!(audio.as_planar_f32(), &[-0.5, 0.0, 0.5]);
    assert_eq!(audio.frames(), FrameCount::new(3));
    assert_eq!(audio.spec().sample_rate().as_u32(), 48_000);
}

#[test]
fn upsample_rejects_invalid_factor_and_rate_overflow() {
    let invalid_factor = parse_effect_chain(&["upsample", "0"]).unwrap_err();
    assert!(matches!(
        invalid_factor,
        EffectChainParseError::CommandParseFailed {
            source: EffectCommandParseError::InvalidEffectConfig {
                effect: "upsample",
                argument: "factor",
                source: EffectError::InvalidUpsampleFactor,
            },
            ..
        }
    ));

    let mut high_rate = audio_buffer(1, u32::MAX, vec![0.0]);
    let high_rate_error = parse_effect_chain(&["upsample", "2"])
        .unwrap()
        .process_buffer(&mut high_rate)
        .unwrap_err();
    assert!(matches!(
        high_rate_error,
        auralis_effects::EffectChainError::CommandFailed {
            argument: "factor",
            source: EffectError::UpsampleRateOverflow,
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

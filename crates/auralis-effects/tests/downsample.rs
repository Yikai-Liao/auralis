//! Integration coverage for SoX-ng-style downsample.

use auralis_core::{AudioSpec, ChannelCount, FrameCount, SampleFormat, SampleRate};
use auralis_effects::{
    Downsample, EffectChainParseError, EffectCommand, EffectCommandParseError, EffectError,
    Reverse, parse_effect_chain,
};

#[test]
fn downsample_chain_parses_default_and_explicit_factor() {
    let default = parse_effect_chain(&["downsample", "reverse"]).unwrap();
    assert_eq!(
        default.commands(),
        &[
            EffectCommand::Downsample(Downsample::default()),
            EffectCommand::Reverse(Reverse::new())
        ]
    );
    assert_eq!(default.render_tokens(), ["downsample", "2", "reverse"]);

    let explicit = parse_effect_chain(&["downsample", "3"]).unwrap();
    assert_eq!(
        explicit.commands(),
        &[EffectCommand::Downsample(Downsample::new(3).unwrap())]
    );
    assert_eq!(explicit.render_tokens(), ["downsample", "3"]);
}

#[test]
fn downsample_decimates_frames_and_changes_sample_rate() {
    let mut audio = stereo_audio(vec![0.0, 0.25, 0.5, 0.75], vec![-0.5, -0.25, 0.0, 0.25]);

    parse_effect_chain(&["downsample", "2"])
        .unwrap()
        .process_buffer(&mut audio)
        .unwrap();

    assert_eq!(audio.frames(), FrameCount::new(2));
    assert_eq!(audio.spec().sample_rate().as_u32(), 24_000);
    assert_eq!(audio.as_planar_f32(), &[0.0, 0.5, -0.5, 0.0]);
}

#[test]
fn downsample_keeps_first_frame_of_final_partial_group() {
    let mut audio = mono_audio(vec![0.0, 0.25, 0.5, 0.75, 1.0]);

    parse_effect_chain(&["downsample", "3"])
        .unwrap()
        .process_buffer(&mut audio)
        .unwrap();

    assert_eq!(audio.frames(), FrameCount::new(2));
    assert_eq!(audio.spec().sample_rate().as_u32(), 16_000);
    assert_eq!(audio.as_planar_f32(), &[0.0, 0.75]);
}

#[test]
fn downsample_factor_one_is_identity_in_chain() {
    let mut audio = mono_audio(vec![-0.5, 0.0, 0.5]);

    parse_effect_chain(&["downsample", "1"])
        .unwrap()
        .process_buffer(&mut audio)
        .unwrap();

    assert_eq!(audio.as_planar_f32(), &[-0.5, 0.0, 0.5]);
    assert_eq!(audio.frames(), FrameCount::new(3));
    assert_eq!(audio.spec().sample_rate().as_u32(), 48_000);
}

#[test]
fn downsample_rejects_invalid_factor_and_zero_output_rate() {
    let invalid_factor = parse_effect_chain(&["downsample", "0"]).unwrap_err();
    assert!(matches!(
        invalid_factor,
        EffectChainParseError::CommandParseFailed {
            source: EffectCommandParseError::InvalidEffectConfig {
                effect: "downsample",
                argument: "factor",
                source: EffectError::InvalidDownsampleFactor,
            },
            ..
        }
    ));

    let mut low_rate = audio_buffer(1, 8_000, vec![0.0]);
    let low_rate_error = parse_effect_chain(&["downsample", "16384"])
        .unwrap()
        .process_buffer(&mut low_rate)
        .unwrap_err();
    assert!(matches!(
        low_rate_error,
        auralis_effects::EffectChainError::CommandFailed {
            argument: "factor",
            source: EffectError::DownsampleRateTooLow,
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

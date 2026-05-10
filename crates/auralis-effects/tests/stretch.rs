//! Integration coverage for SoX-ng-style stretch processing.

use auralis_core::{AudioSpec, ChannelCount, FrameCount, SampleFormat, SampleRate};
use auralis_effects::{
    EffectChainParseError, EffectCommand, EffectCommandParseError, EffectError, Reverse, Stretch,
    StretchFade, parse_effect_chain,
};

#[test]
fn stretch_chain_parses_default_and_explicit_options() {
    let default = parse_effect_chain(&["stretch"]).unwrap();
    assert_eq!(
        default.commands(),
        &[EffectCommand::Stretch(Stretch::default())]
    );
    assert_eq!(
        default.render_tokens(),
        ["stretch", "1", "20", "l", "1", "0"]
    );

    let explicit =
        parse_effect_chain(&["stretch", "1.5", "10", "q", "0.75", "0.25", "reverse"]).unwrap();
    assert_eq!(
        explicit.commands(),
        &[
            EffectCommand::Stretch(
                Stretch::with_options(1.5, 10.0, StretchFade::QuarterCosine, 0.75, 0.25).unwrap()
            ),
            EffectCommand::Reverse(Reverse::new())
        ]
    );
    assert_eq!(
        explicit.render_tokens(),
        ["stretch", "1.5", "10", "q", "0.75", "0.25", "reverse"]
    );
}

#[test]
fn stretch_chain_changes_frame_count_and_preserves_sample_rate() {
    let mut audio = mono_audio((0_u16..256).map(|frame| f32::from(frame) / 256.0).collect());

    parse_effect_chain(&["stretch", "1.5", "1", "l", "1", "0"])
        .unwrap()
        .process_buffer(&mut audio)
        .unwrap();

    assert!(audio.frames().as_u64() > 256);
    assert_eq!(audio.spec().sample_rate().as_u32(), 48_000);
    assert!(
        audio
            .as_planar_f32()
            .iter()
            .all(|sample| sample.is_finite())
    );
}

#[test]
fn stretch_chain_preserves_stereo_channel_count() {
    let mut audio = stereo_audio(
        (0_u16..256).map(|frame| f32::from(frame) / 256.0).collect(),
        (0_u16..256)
            .map(|frame| -f32::from(frame) / 256.0)
            .collect(),
    );

    parse_effect_chain(&["stretch", "0.75", "1", "s", "1", "0.25"])
        .unwrap()
        .process_buffer(&mut audio)
        .unwrap();

    assert_eq!(audio.channels(), ChannelCount::new(2).unwrap());
    assert_eq!(
        audio.as_planar_f32().len(),
        audio.channels().as_usize() * usize::try_from(audio.frames().as_u64()).unwrap()
    );
}

#[test]
fn stretch_chain_rejects_invalid_values() {
    let invalid = parse_effect_chain(&["stretch", "1", "0"]).unwrap_err();
    assert_eq!(
        invalid,
        EffectChainParseError::CommandParseFailed {
            index: 0,
            command: "stretch 1 0".to_owned(),
            source: EffectCommandParseError::InvalidEffectConfig {
                effect: "stretch",
                argument: "stretch",
                source: EffectError::InvalidStretch,
            },
        }
    );
}

fn mono_audio(samples: Vec<f32>) -> auralis_core::AudioBuffer {
    let spec = AudioSpec::new(
        SampleRate::new(48_000).unwrap(),
        ChannelCount::new(1).unwrap(),
        SampleFormat::Float32,
    );
    auralis_core::AudioBuffer::from_planar_f32(
        spec,
        FrameCount::new(u64::try_from(samples.len()).unwrap()),
        samples,
    )
    .unwrap()
}

fn stereo_audio(left: Vec<f32>, right: Vec<f32>) -> auralis_core::AudioBuffer {
    assert_eq!(left.len(), right.len());
    let spec = AudioSpec::new(
        SampleRate::new(48_000).unwrap(),
        ChannelCount::new(2).unwrap(),
        SampleFormat::Float32,
    );
    let frames = FrameCount::new(u64::try_from(left.len()).unwrap());
    let mut samples = left;
    samples.extend(right);
    auralis_core::AudioBuffer::from_planar_f32(spec, frames, samples).unwrap()
}

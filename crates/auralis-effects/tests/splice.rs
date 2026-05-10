//! Integration coverage for SoX-ng-style splice processing.

use auralis_core::{AudioSpec, ChannelCount, FrameCount, SampleFormat, SampleRate};
use auralis_effects::{
    EffectChainParseError, EffectCommand, EffectCommandParseError, EffectError, Reverse, Splice,
    SpliceAmount, SpliceFade, SplicePoint, SplicePosition, parse_effect_chain,
};

#[test]
fn splice_chain_parses_fade_points_and_stops_before_next_effect() {
    let chain = parse_effect_chain(&["splice", "-t", "48s,4s,0s", "96s,2s", "reverse"]).unwrap();

    assert_eq!(
        chain.commands(),
        &[
            EffectCommand::Splice(
                Splice::with_fade(
                    SpliceFade::Triangular,
                    [
                        SplicePoint::new(
                            SplicePosition::frames(FrameCount::new(48)),
                            Some(SpliceAmount::Frames(FrameCount::new(4))),
                            Some(SpliceAmount::Frames(FrameCount::new(0))),
                        ),
                        SplicePoint::new(
                            SplicePosition::frames(FrameCount::new(96)),
                            Some(SpliceAmount::Frames(FrameCount::new(2))),
                            None,
                        )
                    ]
                )
                .unwrap()
            ),
            EffectCommand::Reverse(Reverse::new())
        ]
    );
    assert_eq!(
        chain.render_tokens(),
        ["splice", "-t", "48s,4s,0s", "96s,2s", "reverse"]
    );
}

#[test]
fn splice_chain_shortens_output_and_preserves_sample_rate_channels() {
    let mut audio = stereo_audio(
        (0_u16..128).map(|frame| f32::from(frame) / 128.0).collect(),
        (0_u16..128)
            .map(|frame| -f32::from(frame) / 128.0)
            .collect(),
    );

    parse_effect_chain(&["splice", "-q", "64s,4s,0s"])
        .unwrap()
        .process_buffer(&mut audio)
        .unwrap();

    assert_eq!(audio.channels(), ChannelCount::new(2).unwrap());
    assert_eq!(audio.spec().sample_rate().as_u32(), 48_000);
    assert_eq!(audio.frames().as_u64(), 112);
    assert!(
        audio
            .as_planar_f32()
            .iter()
            .all(|sample| sample.is_finite())
    );
}

#[test]
fn splice_chain_rejects_missing_option_and_invalid_points() {
    assert_eq!(
        parse_effect_chain(&["splice"]).unwrap_err(),
        EffectChainParseError::CommandParseFailed {
            index: 0,
            command: "splice".to_owned(),
            source: EffectCommandParseError::MissingArgument {
                effect: "splice",
                argument: "position[,excess[,leeway]]",
            },
        }
    );
    assert_eq!(
        parse_effect_chain(&["splice", "-x", "48s"]).unwrap_err(),
        EffectChainParseError::CommandParseFailed {
            index: 0,
            command: "splice -x 48s".to_owned(),
            source: EffectCommandParseError::UnsupportedOption {
                effect: "splice",
                option: "-x".to_owned(),
            },
        }
    );
    assert_eq!(
        parse_effect_chain(&["splice", "8s,4s,0s"])
            .unwrap()
            .process_buffer(&mut mono_audio(vec![0.0; 32]))
            .unwrap_err(),
        auralis_effects::EffectChainError::CommandFailed {
            index: 0,
            command: Box::new(
                Splice::new([SplicePoint::new(
                    SplicePosition::frames(FrameCount::new(8)),
                    Some(SpliceAmount::Frames(FrameCount::new(4))),
                    Some(SpliceAmount::Frames(FrameCount::new(0))),
                )])
                .map(EffectCommand::Splice)
                .unwrap()
            ),
            argument: "splice",
            source: EffectError::InvalidSplice,
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

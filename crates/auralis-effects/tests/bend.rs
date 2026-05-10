//! Integration coverage for SoX-ng-style bend processing.

use auralis_core::{AudioSpec, ChannelCount, FrameCount, SampleFormat, SampleRate};
use auralis_effects::{
    Bend, BendAmount, BendAnchor, BendPosition, BendSegment, EffectChainParseError, EffectCommand,
    EffectCommandParseError, EffectError, Reverse, parse_effect_chain,
};

#[test]
fn bend_chain_parses_options_segments_and_stops_before_next_effect() {
    let chain =
        parse_effect_chain(&["bend", "-f", "40", "-o", "8", "0s,100,+200s", "reverse"]).unwrap();

    assert_eq!(
        chain.commands(),
        &[
            EffectCommand::Bend(
                Bend::with_options(
                    40,
                    8,
                    [BendSegment::new(
                        BendPosition::frames(FrameCount::new(0)),
                        100.0,
                        BendPosition::new(
                            BendAnchor::Previous,
                            BendAmount::Frames(FrameCount::new(200))
                        )
                    )
                    .unwrap()]
                )
                .unwrap()
            ),
            EffectCommand::Reverse(Reverse::new())
        ]
    );
    assert_eq!(
        chain.render_tokens(),
        ["bend", "-f", "40", "-o", "8", "0s,100,+200s", "reverse"]
    );
}

#[test]
fn bend_chain_preserves_shape_and_finite_samples() {
    let mut audio = stereo_audio(
        (0_u16..4096)
            .map(|frame| f32::from(frame % 128) / 128.0)
            .collect(),
        (0_u16..4096)
            .map(|frame| -f32::from(frame % 128) / 128.0)
            .collect(),
    );

    parse_effect_chain(&["bend", "0s,100,2048s"])
        .unwrap()
        .process_buffer(&mut audio)
        .unwrap();

    assert_eq!(audio.channels(), ChannelCount::new(2).unwrap());
    assert_eq!(audio.frames().as_u64(), 4096);
    assert_eq!(audio.spec().sample_rate().as_u32(), 48_000);
    assert!(
        audio
            .as_planar_f32()
            .iter()
            .all(|sample| sample.is_finite())
    );
}

#[test]
fn bend_chain_rejects_missing_options_and_invalid_segments() {
    assert_eq!(
        parse_effect_chain(&["bend"]).unwrap_err(),
        EffectChainParseError::CommandParseFailed {
            index: 0,
            command: "bend".to_owned(),
            source: EffectCommandParseError::MissingArgument {
                effect: "bend",
                argument: "start,cents,end",
            },
        }
    );
    assert_eq!(
        parse_effect_chain(&["bend", "-x", "0,0,0"]).unwrap_err(),
        EffectChainParseError::CommandParseFailed {
            index: 0,
            command: "bend -x 0,0,0".to_owned(),
            source: EffectCommandParseError::UnsupportedOption {
                effect: "bend",
                option: "-x".to_owned(),
            },
        }
    );
    assert_eq!(
        parse_effect_chain(&["bend", "-o", "33", "0,0,0"]).unwrap_err(),
        EffectChainParseError::CommandParseFailed {
            index: 0,
            command: "bend -o 33 0,0,0".to_owned(),
            source: EffectCommandParseError::InvalidEffectConfig {
                effect: "bend",
                argument: "bend",
                source: EffectError::InvalidBend,
            },
        }
    );
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

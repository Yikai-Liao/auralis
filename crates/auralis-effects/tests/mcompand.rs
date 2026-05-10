//! Integration coverage for SoX-ng-style mcompand processing.

use auralis_core::{AudioBuffer, AudioSpec, ChannelCount, FrameCount, SampleFormat, SampleRate};
use auralis_effects::{
    EffectChainParseError, EffectCommand, EffectCommandParseError, EffectError, MCompand, Reverse,
    parse_effect_chain,
};

#[test]
fn mcompand_chain_parses_quoted_band_args_and_stops_before_next_effect() {
    let chain = parse_effect_chain(&[
        "mcompand",
        "0,0 -60,-60,0,0",
        "1000",
        "0,0 -60,-60,0,0",
        "reverse",
    ])
    .unwrap();

    assert_eq!(
        chain.commands(),
        &[
            EffectCommand::MCompand(
                MCompand::parse_sox_args(&["0,0 -60,-60,0,0", "1000", "0,0 -60,-60,0,0"]).unwrap()
            ),
            EffectCommand::Reverse(Reverse::new()),
        ]
    );
}

#[test]
fn mcompand_chain_executes_single_band_command() {
    let chain = parse_effect_chain(&["mcompand", "0,0 -60,-60,0,-6"]).unwrap();
    let mut audio = mono_buffer(vec![0.25, -0.5, 0.0]);

    chain.process_buffer(&mut audio).unwrap();

    assert_eq!(audio.frames(), FrameCount::new(3));
    assert!(
        audio
            .as_planar_f32()
            .iter()
            .all(|sample| sample.is_finite())
    );
}

#[test]
fn mcompand_chain_rejects_missing_and_invalid_arguments() {
    assert_eq!(
        parse_effect_chain(&["mcompand"]).unwrap_err(),
        EffectChainParseError::CommandParseFailed {
            index: 0,
            command: "mcompand".to_owned(),
            source: EffectCommandParseError::MissingArgument {
                effect: "mcompand",
                argument: "quoted_compand_args",
            },
        }
    );
    assert_eq!(
        parse_effect_chain(&["mcompand", "0,0 -60,-60,0,0", "100"]).unwrap_err(),
        EffectChainParseError::CommandParseFailed {
            index: 0,
            command: "mcompand 0,0 -60,-60,0,0 100".to_owned(),
            source: EffectCommandParseError::InvalidEffectConfig {
                effect: "mcompand",
                argument: "mcompand",
                source: EffectError::InvalidMCompand,
            },
        }
    );
}

fn mono_buffer(samples: Vec<f32>) -> AudioBuffer {
    AudioBuffer::from_planar_f32(
        AudioSpec::new(
            SampleRate::new(48_000).unwrap(),
            ChannelCount::new(1).unwrap(),
            SampleFormat::Float32,
        ),
        FrameCount::new(u64::try_from(samples.len()).unwrap()),
        samples,
    )
    .unwrap()
}

//! Integration coverage for SoX-ng-style compand processing.

use auralis_core::{AudioSpec, ChannelCount, FrameCount, SampleFormat, SampleRate};
use auralis_effects::{
    Compand, EffectChainParseError, EffectCommand, EffectCommandParseError, EffectError, Reverse,
    parse_effect_chain,
};

#[test]
fn compand_chain_parses_command_and_stops_before_next_effect() {
    let chain = parse_effect_chain(&[
        "compand",
        "0.3,1",
        "6:-70,-60,-20,-20,0,0",
        "-3",
        "-90",
        "0.01",
        "reverse",
    ])
    .unwrap();

    assert_eq!(
        chain.commands(),
        &[
            EffectCommand::Compand(
                Compand::parse_sox_args(&["0.3,1", "6:-70,-60,-20,-20,0,0", "-3", "-90", "0.01"])
                    .unwrap()
            ),
            EffectCommand::Reverse(Reverse::new())
        ]
    );
    assert_eq!(
        chain.render_tokens(),
        [
            "compand",
            "0.3,1",
            "6:-70,-60,-20,-20,0,0",
            "-3",
            "-90",
            "0.01",
            "reverse"
        ]
    );
}

#[test]
fn compand_shared_stereo_envelope_uses_frame_peak() {
    let mut audio = stereo_audio(vec![0.5, 0.25], vec![0.25, 0.5]);
    let compand = Compand::parse_sox_args(&["0,0", "-60,-60,0,-6"]).unwrap();
    let gain = compand.transfer().linear_gain_for_level(0.5);

    compand.process_buffer(&mut audio).unwrap();

    assert_samples_close(
        audio.as_planar_f32(),
        &[
            expected_sample(0.5, gain),
            expected_sample(0.25, gain),
            expected_sample(0.25, gain),
            expected_sample(0.5, gain),
        ],
    );
}

#[test]
fn compand_delay_is_lookahead_and_preserves_length() {
    let mut audio = mono_audio(vec![1.0, 0.25]);
    let compand = Compand::parse_sox_args(&["0,0", "-60,-60,0,-6", "0", "0", "0.1"]).unwrap();
    let gain = compand.transfer().linear_gain_for_level(0.25);

    compand.process_buffer(&mut audio).unwrap();

    assert_eq!(audio.frames(), FrameCount::new(2));
    assert_samples_close(
        audio.as_planar_f32(),
        &[expected_sample(1.0, gain), expected_sample(0.25, gain)],
    );
}

#[test]
fn compand_rejects_mismatched_channel_specific_envelopes() {
    let mut audio = stereo_audio(vec![0.25, 0.5], vec![0.5, 0.25]);
    let compand = Compand::parse_sox_args(&["0,0,0,0,0,0", "-60,-60,0,0"]).unwrap();

    assert_eq!(
        compand.process_buffer(&mut audio).unwrap_err(),
        EffectError::InvalidCompand
    );
}

#[test]
fn compand_chain_rejects_missing_and_invalid_arguments() {
    assert_eq!(
        parse_effect_chain(&["compand"]).unwrap_err(),
        EffectChainParseError::CommandParseFailed {
            index: 0,
            command: "compand".to_owned(),
            source: EffectCommandParseError::MissingArgument {
                effect: "compand",
                argument: "attack,decay and transfer",
            },
        }
    );
    assert_eq!(
        parse_effect_chain(&["compand", "0,1", "-20,-20,-30,-30"]).unwrap_err(),
        EffectChainParseError::CommandParseFailed {
            index: 0,
            command: "compand 0,1 -20,-20,-30,-30".to_owned(),
            source: EffectCommandParseError::InvalidEffectConfig {
                effect: "compand",
                argument: "compand",
                source: EffectError::InvalidCompand,
            },
        }
    );
}

fn mono_audio(samples: Vec<f32>) -> auralis_core::AudioBuffer {
    let spec = AudioSpec::new(
        SampleRate::new(10).unwrap(),
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

fn assert_samples_close(actual: &[f32], expected: &[f32]) {
    assert_eq!(actual.len(), expected.len());
    for (index, (&actual, &expected)) in actual.iter().zip(expected).enumerate() {
        assert!(
            (actual - expected).abs() <= 0.000_001,
            "sample {index} differed: {actual} != {expected}"
        );
    }
}

#[allow(
    clippy::cast_possible_truncation,
    reason = "test fixtures compare against the f32 effect buffer format"
)]
fn expected_sample(sample: f64, gain: f64) -> f32 {
    (sample * gain) as f32
}

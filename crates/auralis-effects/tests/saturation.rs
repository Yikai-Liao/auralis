//! Integration coverage for SoX-ng-style saturation distortion.

use auralis_core::{AudioSpec, ChannelCount, FrameCount, SampleFormat, SampleRate};
use auralis_effects::{
    EffectChainParseError, EffectCommand, EffectCommandParseError, EffectError, Reverse,
    Saturation, SaturationType, parse_effect_chain,
};

#[test]
fn saturation_chain_parses_default_and_explicit_arguments() {
    let default = parse_effect_chain(&["saturation", "reverse"]).unwrap();
    assert_eq!(
        default.commands(),
        &[
            EffectCommand::Saturation(Saturation::default()),
            EffectCommand::Reverse(Reverse::new())
        ]
    );
    assert_eq!(
        default.render_tokens(),
        ["saturation", "tanh", "1", "0", "1", "reverse"]
    );

    let explicit = parse_effect_chain(&["saturation", "sqrt", "0.75", "0.1", "0.25"]).unwrap();
    assert_eq!(
        explicit.commands(),
        &[EffectCommand::Saturation(
            Saturation::new(SaturationType::Sqrt, 0.75, 0.1, 0.25).unwrap()
        )]
    );
    assert_eq!(
        explicit.render_tokens(),
        ["saturation", "sqrt", "0.75", "0.1", "0.25"]
    );
}

#[test]
fn saturation_uses_sox_ng_tanh_sqrt_and_diode_transfers() {
    let source = [-1.0, -0.5, 0.0, 0.5, 1.0];

    let mut tanh = source;
    Saturation::tanh(0.75, 0.25, 3.0)
        .unwrap()
        .process_samples(&mut tanh);
    assert_samples_close(
        &tanh,
        &[-0.999_925, -0.715_529_9, 0.0, 0.284_395_07, 0.419_096_23],
    );

    let mut sqrt = source;
    Saturation::sqrt(1.0, 0.0, 0.5)
        .unwrap()
        .process_samples(&mut sqrt);
    assert_samples_close(&sqrt, &[-0.9999, -0.530_277, 0.0, 0.530_277, 0.9999]);

    let mut diode = source;
    Saturation::diode(0.8, 0.2, 0.4)
        .unwrap()
        .process_samples(&mut diode);
    assert_samples_close(&diode, &[-0.999_92, -0.7666, 0.0, 0.366_64, 0.466_64]);
}

#[test]
fn saturation_processes_planar_stereo_samples_independently() {
    let mut audio = stereo_audio(vec![-0.5, 0.0, 0.5], vec![0.25, -0.25, 1.0]);

    parse_effect_chain(&["saturation", "diode", "1", "0", "0.5"])
        .unwrap()
        .process_buffer(&mut audio)
        .unwrap();

    assert_samples_close(
        audio.as_planar_f32(),
        &[-0.9999, 0.0, 0.9999, 0.499_95, -0.499_95, 0.9999],
    );
}

#[test]
fn saturation_rejects_invalid_values_and_extra_arguments() {
    let invalid = parse_effect_chain(&["saturation", "sqrt", "1.1"]).unwrap_err();
    assert!(matches!(
        invalid,
        EffectChainParseError::CommandParseFailed {
            source: EffectCommandParseError::InvalidEffectConfig {
                effect: "saturation",
                argument: "saturation",
                source: EffectError::InvalidSaturation,
            },
            ..
        }
    ));

    let unknown_type = parse_effect_chain(&["saturation", "unknown"]).unwrap_err();
    assert!(matches!(
        unknown_type,
        EffectChainParseError::CommandParseFailed {
            source: EffectCommandParseError::UnexpectedArgument {
                effect: "saturation",
                argument,
            },
            ..
        } if argument == "unknown"
    ));

    let extra = parse_effect_chain(&["saturation", "tanh", "1", "0", "1", "extra"]).unwrap_err();
    assert!(matches!(
        extra,
        EffectChainParseError::CommandParseFailed {
            source: EffectCommandParseError::UnexpectedArgument {
                effect: "saturation",
                argument,
            },
            ..
        } if argument == "extra"
    ));
}

fn stereo_audio(left: Vec<f32>, right: Vec<f32>) -> auralis_core::AudioBuffer {
    assert_eq!(left.len(), right.len());
    let frames = u64::try_from(left.len()).unwrap();
    let spec = AudioSpec::new(
        SampleRate::new(48_000).unwrap(),
        ChannelCount::new(2).unwrap(),
        SampleFormat::Float32,
    );
    let samples = left.into_iter().chain(right).collect();
    auralis_core::AudioBuffer::from_planar_f32(spec, FrameCount::new(frames), samples).unwrap()
}

fn assert_samples_close(actual: &[f32], expected: &[f32]) {
    assert_eq!(actual.len(), expected.len());
    for (index, (&actual, &expected)) in actual.iter().zip(expected).enumerate() {
        assert!(
            (actual - expected).abs() <= 0.000_001,
            "sample {index}: actual={actual}, expected={expected}"
        );
    }
}

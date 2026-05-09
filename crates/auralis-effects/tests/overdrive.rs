//! Integration coverage for SoX-ng-style overdrive distortion.

use auralis_core::{AudioSpec, ChannelCount, FrameCount, SampleFormat, SampleRate};
use auralis_effects::{
    EffectChainParseError, EffectCommand, EffectCommandParseError, EffectError, Overdrive, Reverse,
    parse_effect_chain,
};

#[test]
fn overdrive_chain_parses_default_and_explicit_arguments() {
    let default = parse_effect_chain(&["overdrive", "reverse"]).unwrap();
    assert_eq!(
        default.commands(),
        &[
            EffectCommand::Overdrive(Overdrive::default()),
            EffectCommand::Reverse(Reverse::new())
        ]
    );
    assert_eq!(
        default.render_tokens(),
        ["overdrive", "20", "20", "reverse"]
    );

    let explicit = parse_effect_chain(&["overdrive", "12", "25"]).unwrap();
    assert_eq!(
        explicit.commands(),
        &[EffectCommand::Overdrive(
            Overdrive::new(12.0, 25.0).unwrap()
        )]
    );
    assert_eq!(explicit.render_tokens(), ["overdrive", "12", "25"]);
}

#[test]
fn overdrive_uses_sox_ng_transfer_and_output_blend() {
    let mut audio = mono_audio(vec![0.0, 0.1, -0.1], 48_000);

    parse_effect_chain(&["overdrive"])
        .unwrap()
        .process_buffer(&mut audio)
        .unwrap();

    assert_samples_close(
        audio.as_planar_f32(),
        &[0.074_75, 0.549_626_23, -0.545_621_9],
    );
}

#[test]
fn overdrive_zero_gain_is_null_effect_even_with_color() {
    let mut audio = mono_audio(vec![0.25, -0.5, 0.75], 48_000);

    Overdrive::new(0.0, 100.0)
        .unwrap()
        .process_buffer(&mut audio);

    assert_eq!(audio.as_planar_f32(), &[0.25, -0.5, 0.75]);
}

#[test]
fn overdrive_rejects_invalid_values_and_extra_arguments() {
    let invalid = parse_effect_chain(&["overdrive", "20", "-1"]).unwrap_err();
    assert!(matches!(
        invalid,
        EffectChainParseError::CommandParseFailed {
            source: EffectCommandParseError::InvalidEffectConfig {
                effect: "overdrive",
                argument: "overdrive",
                source: EffectError::InvalidOverdrive,
            },
            ..
        }
    ));

    let extra = parse_effect_chain(&["overdrive", "20", "20", "extra"]).unwrap_err();
    assert!(matches!(
        extra,
        EffectChainParseError::CommandParseFailed {
            source: EffectCommandParseError::UnexpectedArgument {
                effect: "overdrive",
                argument,
            },
            ..
        } if argument == "extra"
    ));
}

fn mono_audio(samples: Vec<f32>, sample_rate: u32) -> auralis_core::AudioBuffer {
    let frames = u64::try_from(samples.len()).unwrap();
    let spec = AudioSpec::new(
        SampleRate::new(sample_rate).unwrap(),
        ChannelCount::new(1).unwrap(),
        SampleFormat::Float32,
    );
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

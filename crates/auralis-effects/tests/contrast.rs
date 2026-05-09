//! Integration coverage for SoX-ng-style contrast enhancement.

use auralis_core::{AudioBuffer, AudioSpec, ChannelCount, FrameCount, SampleFormat, SampleRate};
use auralis_effects::{
    Contrast, EffectChainParseError, EffectCommand, EffectCommandParseError, EffectError,
    parse_effect_chain,
};

#[test]
fn contrast_chain_parses_default_and_explicit_amount() {
    let default = parse_effect_chain(&["contrast", "reverse"]).unwrap();

    assert_eq!(default.render_tokens(), ["contrast", "75", "reverse"]);
    assert_eq!(
        default.commands()[0],
        EffectCommand::Contrast(Contrast::default_amount())
    );

    let explicit = parse_effect_chain(&["contrast", "25"]).unwrap();
    assert_eq!(explicit.render_tokens(), ["contrast", "25"]);
    assert_eq!(
        explicit.commands()[0],
        EffectCommand::Contrast(Contrast::new(25.0).unwrap())
    );
}

#[test]
fn contrast_amount_zero_applies_base_sine_curve() {
    let mut audio = mono_audio_buffer(vec![-1.0, -0.5, 0.0, 0.5, 1.0]);

    parse_effect_chain(&["contrast", "0"])
        .unwrap()
        .process_buffer(&mut audio)
        .unwrap();

    assert_samples_close(
        audio.as_planar_f32(),
        &[
            -1.0,
            -std::f32::consts::FRAC_1_SQRT_2,
            0.0,
            std::f32::consts::FRAC_1_SQRT_2,
            1.0,
        ],
    );
}

#[test]
fn contrast_default_matches_typed_processor() {
    let source = mono_audio_buffer(vec![-0.75, -0.25, 0.0, 0.25, 0.75]);
    let mut chained = source.clone();
    let mut direct = source;

    parse_effect_chain(&["contrast"])
        .unwrap()
        .process_buffer(&mut chained)
        .unwrap();
    Contrast::default_amount().process_buffer(&mut direct);

    assert_eq!(chained.as_planar_f32(), direct.as_planar_f32());
}

#[test]
fn contrast_rejects_invalid_amount_and_extra_arguments() {
    let invalid_amount = parse_effect_chain(&["contrast", "101"]).unwrap_err();
    assert!(matches!(
        invalid_amount,
        EffectChainParseError::CommandParseFailed {
            source: EffectCommandParseError::InvalidEffectConfig {
                effect: "contrast",
                argument: "amount",
                source: EffectError::InvalidContrastAmount,
            },
            ..
        }
    ));

    let extra = parse_effect_chain(&["contrast", "75", "extra"]).unwrap_err();
    assert!(matches!(
        extra,
        EffectChainParseError::CommandParseFailed {
            source: EffectCommandParseError::UnexpectedArgument {
                effect: "contrast",
                argument,
            },
            ..
        } if argument == "extra"
    ));
}

fn mono_audio_buffer(samples: Vec<f32>) -> AudioBuffer {
    let spec = AudioSpec::new(
        SampleRate::new(48_000).unwrap(),
        ChannelCount::new(1).unwrap(),
        SampleFormat::Float32,
    );
    AudioBuffer::from_planar_f32(
        spec,
        FrameCount::new(u64::try_from(samples.len()).unwrap()),
        samples,
    )
    .unwrap()
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

//! Integration coverage for SoX-ng-style direct-coefficient biquad filtering.

use auralis_core::{AudioBuffer, AudioSpec, ChannelCount, FrameCount, SampleFormat, SampleRate};
use auralis_effects::{
    Biquad, BiquadCoefficients, EffectCommandParseError, EffectError, parse_effect_chain,
};

#[test]
fn biquad_chain_parses_and_renders_normalized_coefficients() {
    let chain = parse_effect_chain(&["biquad", "2", "1", "0.5", "4", "-1", "0.25"]).unwrap();

    assert_eq!(chain.len(), 1);
    assert_eq!(
        chain.render_tokens(),
        ["biquad", "0.5", "0.25", "0.125", "1", "-0.25", "0.0625"]
    );
}

#[test]
fn biquad_chain_matches_typed_processor() {
    let spec = AudioSpec::new(
        SampleRate::new(48_000).unwrap(),
        ChannelCount::new(2).unwrap(),
        SampleFormat::Float32,
    );
    let source = AudioBuffer::from_planar_f32(
        spec,
        FrameCount::new(5),
        vec![1.0, 0.0, 0.0, 0.0, 0.0, -0.5, 0.25, 0.0, 0.0, 0.0],
    )
    .unwrap();
    let coefficients = BiquadCoefficients::from_raw(0.5, 0.0, 0.0, 1.0, -0.5, 0.0).unwrap();
    let mut direct = source.clone();
    let mut parsed = source;

    Biquad::new(coefficients).process_buffer(&mut direct);
    parse_effect_chain(&["biquad", "0.5", "0", "0", "1", "-0.5", "0"])
        .unwrap()
        .process_buffer(&mut parsed)
        .unwrap();

    assert_sample_bits_eq(parsed.as_planar_f32(), direct.as_planar_f32());
}

#[test]
fn biquad_rejects_bad_command_shapes() {
    let missing = parse_effect_chain(&["biquad", "1", "0", "0", "1", "0"]).unwrap_err();
    assert!(matches!(
        missing,
        auralis_effects::EffectChainParseError::CommandParseFailed {
            source: EffectCommandParseError::MissingArgument {
                effect: "biquad",
                argument: "a2",
            },
            ..
        }
    ));

    let invalid = parse_effect_chain(&["biquad", "1", "0", "0", "0", "0", "0"]).unwrap_err();
    assert!(matches!(
        invalid,
        auralis_effects::EffectChainParseError::CommandParseFailed {
            source: EffectCommandParseError::InvalidEffectConfig {
                effect: "biquad",
                argument: "coefficients",
                source: EffectError::InvalidBiquadCoefficients,
            },
            ..
        }
    ));

    let extra = parse_effect_chain(&["biquad", "1", "0", "0", "1", "0", "0", "extra"]).unwrap_err();
    assert!(matches!(
        extra,
        auralis_effects::EffectChainParseError::CommandParseFailed {
            source: EffectCommandParseError::UnexpectedArgument {
                effect: "biquad",
                ..
            },
            ..
        }
    ));
}

fn assert_sample_bits_eq(actual: &[f32], expected: &[f32]) {
    assert_eq!(actual.len(), expected.len());
    for (index, (actual, expected)) in actual.iter().zip(expected).enumerate() {
        assert_eq!(
            actual.to_bits(),
            expected.to_bits(),
            "sample {index} differed: actual={actual} expected={expected}"
        );
    }
}

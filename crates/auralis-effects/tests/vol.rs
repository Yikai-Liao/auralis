//! Integration coverage for SoX-ng-style vol scaling.

use auralis_core::{
    AudioBuffer, AudioSpec, ChannelCount, Decibels, FrameCount, SampleFormat, SampleRate,
};
use auralis_effects::{
    EffectChainParseError, EffectCommand, EffectCommandParseError, EffectError, Vol,
    parse_effect_chain,
};
use auralis_simd::BackendKind;

#[test]
fn vol_chain_parses_gain_types_and_limiter_gain() {
    let chain = parse_effect_chain(&["vol", "0.25", "power", "reverse"]).unwrap();

    assert_eq!(chain.render_tokens(), ["vol", "0.25", "power", "reverse"]);
    assert_eq!(
        chain.commands()[0],
        EffectCommand::Vol(Vol::power(0.25).unwrap())
    );

    let limited = parse_effect_chain(&["vol", "2dB", "0.05"]).unwrap();
    assert_eq!(limited.render_tokens(), ["vol", "2", "dB", "0.05"]);
}

#[test]
fn vol_amplitude_scales_and_clips_like_sox_ng() {
    let mut audio = mono_audio_buffer(vec![0.25, -0.75, 0.75]);
    let chain = parse_effect_chain(&["vol", "2"]).unwrap();

    chain.process_buffer(&mut audio).unwrap();

    assert_eq!(audio.as_planar_f32(), &[0.5, -1.0, 1.0]);
}

#[test]
fn vol_power_and_decibel_types_convert_to_amplitude() {
    let mut power = mono_audio_buffer(vec![0.25, -0.5]);
    parse_effect_chain(&["vol", "-0.25", "power"])
        .unwrap()
        .process_buffer(&mut power)
        .unwrap();
    assert_eq!(power.as_planar_f32(), &[-0.125, 0.25]);

    let mut db = mono_audio_buffer(vec![0.25, -0.5]);
    parse_effect_chain(&["vol", "6", "dB"])
        .unwrap()
        .process_buffer(&mut db)
        .unwrap();
    let multiplier = auralis_dsp::linear_gain(Decibels::new(6.0).unwrap());
    assert_samples_close(db.as_planar_f32(), &[0.25 * multiplier, -0.5 * multiplier]);
}

#[test]
fn vol_limiter_applies_sox_ng_piecewise_curve() {
    let mut audio = mono_audio_buffer(vec![0.25, 0.5, 1.0, -1.0]);
    let chain = parse_effect_chain(&["vol", "2", "amplitude", "0.05"]).unwrap();

    chain.process_buffer(&mut audio).unwrap();

    assert_samples_close(audio.as_planar_f32(), &[0.5, 0.975, 1.0, -1.0]);
}

#[test]
fn vol_rejects_invalid_type_and_limiter_arguments() {
    let invalid_type = parse_effect_chain(&["vol", "1", "loud"]).unwrap_err();
    assert!(matches!(
        invalid_type,
        EffectChainParseError::CommandParseFailed {
            source: EffectCommandParseError::UnexpectedArgument {
                effect: "vol",
                argument
            },
            ..
        } if argument == "loud"
    ));

    let limiter_without_amplification = parse_effect_chain(&["vol", "0.5", "0.05"]).unwrap_err();
    assert!(matches!(
        limiter_without_amplification,
        EffectChainParseError::CommandParseFailed {
            source: EffectCommandParseError::InvalidEffectConfig {
                effect: "vol",
                argument: "limiter-gain",
                source: EffectError::InvalidVolLimiterGain,
            },
            ..
        }
    ));
}

#[test]
fn vol_matches_under_forced_scalar_and_requested_simd() {
    let source = mono_audio_buffer(vec![-0.75, -0.25, 0.0, 0.25, 0.75]);
    let mut scalar = source.clone();
    let mut simd = source;
    let chain = parse_effect_chain(&["vol", "1.5"]).unwrap();

    chain
        .process_buffer_with_backend(&mut scalar, BackendKind::Scalar)
        .unwrap();
    chain
        .process_buffer_with_backend(&mut simd, BackendKind::Simd)
        .unwrap();

    assert_eq!(simd.as_planar_f32(), scalar.as_planar_f32());
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

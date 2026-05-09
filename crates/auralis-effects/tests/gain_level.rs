//! Integration coverage for SoX-ng-style gain normalization and limiter options.

use auralis_core::{
    AudioBuffer, AudioSpec, ChannelCount, Decibels, FrameCount, SampleFormat, SampleRate,
};
use auralis_dsp::linear_gain;
use auralis_effects::{EffectChainError, EffectCommand, EffectError, Gain, parse_effect_chain};
use auralis_simd::BackendKind;

#[test]
fn gain_normalize_scales_peak_to_full_scale_then_applies_fixed_gain() {
    let mut audio = mono_audio_buffer(vec![0.25, -0.5, 0.125]);
    let chain = parse_effect_chain(&["gain", "-n", "-6"]).unwrap();
    let fixed = linear_gain(Decibels::new(-6.0).unwrap());
    let expected = [0.5 * fixed, -fixed, 0.25 * fixed];

    chain.process_buffer(&mut audio).unwrap();

    assert_samples_close(audio.as_planar_f32(), &expected);
    assert_eq!(chain.render_tokens(), ["gain", "-n", "-6"]);
}

#[test]
fn gain_normalize_leaves_silence_silent() {
    let mut audio = mono_audio_buffer(vec![0.0, -0.0, 0.0]);
    let chain = parse_effect_chain(&["gain", "-n"]).unwrap();

    chain.process_buffer(&mut audio).unwrap();

    assert_eq!(audio.as_planar_f32(), &[0.0, 0.0, 0.0]);
}

#[test]
fn gain_limiter_applies_sox_ng_simple_limiter_curve_after_fixed_gain() {
    let mut audio = mono_audio_buffer(vec![0.5, 1.0, -1.0]);
    let chain = parse_effect_chain(&["gain", "-l", "6"]).unwrap();
    let fixed = linear_gain(Decibels::new(6.0).unwrap());
    let limiter = 1.0 - fixed.recip();
    let expected = [
        limited(0.5 * fixed, limiter),
        limited(fixed, limiter),
        limited(-fixed, limiter),
    ];

    chain.process_buffer(&mut audio).unwrap();

    assert_samples_close(audio.as_planar_f32(), &expected);
    assert_eq!(chain.render_tokens(), ["gain", "-l", "6"]);
}

#[test]
fn gain_normalize_reports_non_finite_samples() {
    let mut audio = mono_audio_buffer(vec![0.25, f32::NAN]);
    let chain = parse_effect_chain(&["gain", "-n"]).unwrap();

    let error = chain.process_buffer(&mut audio).unwrap_err();

    assert_eq!(
        error,
        EffectChainError::CommandFailed {
            index: 0,
            command: EffectCommand::Gain(Gain::normalize(Decibels::new(0.0).unwrap())),
            argument: "normalize",
            source: EffectError::NonFiniteGainSample { sample_index: 1 },
        }
    );
}

#[test]
fn gain_normalize_chain_matches_under_forced_scalar_and_requested_simd() {
    let source = mono_audio_buffer(vec![-0.8, -0.25, 0.0, 0.25, 0.75]);
    let mut scalar = source.clone();
    let mut simd = source;
    let chain = parse_effect_chain(&["gain", "-n", "-3"]).unwrap();

    chain
        .process_buffer_with_backend(&mut scalar, BackendKind::Scalar)
        .unwrap();
    chain
        .process_buffer_with_backend(&mut simd, BackendKind::Simd)
        .unwrap();

    assert_eq!(simd.as_planar_f32(), scalar.as_planar_f32());
}

fn limited(sample: f32, limiter: f32) -> f32 {
    sample / (1.0 + limiter * sample.abs())
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

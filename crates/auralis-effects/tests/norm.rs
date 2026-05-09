//! Integration coverage for SoX-ng-style norm effect normalization.

use auralis_core::{
    AudioBuffer, AudioSpec, ChannelCount, Decibels, FrameCount, SampleFormat, SampleRate,
};
use auralis_dsp::linear_gain;
use auralis_effects::{EffectChainError, EffectCommand, EffectError, Norm, parse_effect_chain};
use auralis_simd::BackendKind;

#[test]
fn norm_default_scales_peak_to_full_scale() {
    let mut audio = mono_audio_buffer(vec![0.25, -0.5, 0.125]);
    let chain = parse_effect_chain(&["norm"]).unwrap();

    chain.process_buffer(&mut audio).unwrap();

    assert_samples_close(audio.as_planar_f32(), &[0.5, -1.0, 0.25]);
    assert_eq!(chain.render_tokens(), ["norm", "0"]);
}

#[test]
fn norm_level_scales_peak_to_target_db() {
    let mut audio = mono_audio_buffer(vec![0.25, -0.5, 0.125]);
    let chain = parse_effect_chain(&["norm", "-6"]).unwrap();
    let target = linear_gain(Decibels::new(-6.0).unwrap());

    chain.process_buffer(&mut audio).unwrap();

    assert_samples_close(
        audio.as_planar_f32(),
        &[0.5 * target, -target, 0.25 * target],
    );
    assert_eq!(chain.render_tokens(), ["norm", "-6"]);
}

#[test]
fn norm_is_positioned_effect_not_final_output_policy() {
    let mut before_gain = mono_audio_buffer(vec![0.25, -0.5]);
    parse_effect_chain(&["norm", "-6", "gain", "-6"])
        .unwrap()
        .process_buffer(&mut before_gain)
        .unwrap();

    let mut after_gain = mono_audio_buffer(vec![0.25, -0.5]);
    parse_effect_chain(&["gain", "-6", "norm", "-6"])
        .unwrap()
        .process_buffer(&mut after_gain)
        .unwrap();

    let target = linear_gain(Decibels::new(-6.0).unwrap());
    assert!(before_gain.as_planar_f32()[1].abs() < target);
    assert!((after_gain.as_planar_f32()[1].abs() - target).abs() <= 0.000_001);
}

#[test]
fn norm_reports_non_finite_samples() {
    let mut audio = mono_audio_buffer(vec![0.25, f32::NAN]);
    let chain = parse_effect_chain(&["norm"]).unwrap();

    let error = chain.process_buffer(&mut audio).unwrap_err();

    assert_eq!(
        error,
        EffectChainError::CommandFailed {
            index: 0,
            command: EffectCommand::Norm(Norm::new(Decibels::new(0.0).unwrap())),
            argument: "level",
            source: EffectError::NonFiniteNormSample { sample_index: 1 },
        }
    );
}

#[test]
fn norm_matches_under_forced_scalar_and_requested_simd() {
    let source = mono_audio_buffer(vec![-0.8, -0.25, 0.0, 0.25, 0.75]);
    let mut scalar = source.clone();
    let mut simd = source;
    let chain = parse_effect_chain(&["norm", "-3"]).unwrap();

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

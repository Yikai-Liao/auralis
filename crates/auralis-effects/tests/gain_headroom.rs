//! Integration coverage for SoX-ng-style gain headroom and reclaim options.

use auralis_core::{AudioBuffer, AudioSpec, ChannelCount, FrameCount, SampleFormat, SampleRate};
use auralis_effects::{
    EffectChain, EffectChainError, EffectCommand, EffectError, Gain, parse_effect_chain,
};
use auralis_simd::BackendKind;

#[test]
fn gain_headroom_then_reclaim_restores_prior_level_without_clipping() {
    let mut audio = mono_audio_buffer(vec![0.25, -0.5, 0.75]);
    let expected = audio.as_planar_f32().to_vec();
    let chain = parse_effect_chain(&["gain", "-h", "-6", "gain", "-r"]).unwrap();

    chain.process_buffer(&mut audio).unwrap();

    assert_samples_close(audio.as_planar_f32(), &expected);
    assert_eq!(
        chain.render_tokens(),
        ["gain", "-h", "-6", "gain", "-r", "0"]
    );
}

#[test]
fn gain_reclaim_limits_restoration_to_current_peak() {
    let mut audio = mono_audio_buffer(vec![0.75]);
    let chain = parse_effect_chain(&["gain", "-h", "-6", "dcshift", "0.8", "gain", "-r"]).unwrap();

    chain.process_buffer(&mut audio).unwrap();

    assert_samples_close(audio.as_planar_f32(), &[1.0]);
}

#[test]
fn gain_reclaim_without_prior_headroom_is_an_error() {
    let mut audio = mono_audio_buffer(vec![0.25]);
    let chain = EffectChain::new(vec![EffectCommand::Gain(Gain::reclaim_headroom(
        auralis_core::Decibels::new(0.0).unwrap(),
    ))]);

    let error = chain.process_buffer(&mut audio).unwrap_err();

    assert_eq!(
        error,
        EffectChainError::CommandFailed {
            index: 0,
            command: chain.commands()[0].clone(),
            argument: "headroom",
            source: EffectError::MissingGainHeadroom,
        }
    );
}

#[test]
fn gain_headroom_chain_matches_under_forced_scalar_and_requested_simd() {
    let source = mono_audio_buffer(vec![-0.8, -0.25, 0.0, 0.25, 0.8]);
    let mut scalar = source.clone();
    let mut simd = source;
    let chain =
        parse_effect_chain(&["gain", "-h", "-6", "dcshift", "0.125", "gain", "-r"]).unwrap();

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

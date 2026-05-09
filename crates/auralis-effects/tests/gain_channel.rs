//! Integration coverage for SoX-ng-style gain channel equalize and balance options.

use auralis_core::{
    AudioBuffer, AudioSpec, ChannelCount, Decibels, FrameCount, SampleFormat, SampleRate,
};
use auralis_dsp::linear_gain;
use auralis_effects::{EffectChainError, EffectCommand, EffectError, Gain, parse_effect_chain};
use auralis_simd::BackendKind;

#[test]
fn gain_equalize_scales_each_channel_to_the_largest_channel_peak() {
    let mut audio = stereo_audio_buffer(vec![0.25, -0.5, 0.0], vec![0.125, -0.25, 0.25]);
    let chain = parse_effect_chain(&["gain", "-e", "-6"]).unwrap();
    let fixed = linear_gain(Decibels::new(-6.0).unwrap());
    let expected = [
        0.25 * fixed,
        -0.5 * fixed,
        0.0,
        0.125 * 2.0 * fixed,
        -0.25 * 2.0 * fixed,
        0.25 * 2.0 * fixed,
    ];

    chain.process_buffer(&mut audio).unwrap();

    assert_samples_close(audio.as_planar_f32(), &expected);
    assert_eq!(chain.render_tokens(), ["gain", "-e", "-6"]);
}

#[test]
fn gain_balance_scales_each_channel_to_the_largest_channel_rms_without_clip_protection() {
    let mut audio = stereo_audio_buffer(vec![1.0, 1.0], vec![0.5, 0.5]);
    let chain = parse_effect_chain(&["gain", "-B", "6"]).unwrap();
    let fixed = linear_gain(Decibels::new(6.0).unwrap());
    let expected = [fixed, fixed, fixed, fixed];

    chain.process_buffer(&mut audio).unwrap();

    assert_samples_close(audio.as_planar_f32(), &expected);
    assert_eq!(chain.render_tokens(), ["gain", "-B", "6"]);
}

#[test]
fn gain_balance_no_clip_attenuates_balanced_channels_to_full_scale() {
    let mut audio = stereo_audio_buffer(vec![0.75, 0.75, 0.75, 0.75], vec![1.0, 0.0, 0.0, 0.0]);
    let chain = parse_effect_chain(&["gain", "-b"]).unwrap();

    chain.process_buffer(&mut audio).unwrap();

    assert_samples_close(
        audio.as_planar_f32(),
        &[0.5, 0.5, 0.5, 0.5, 1.0, 0.0, 0.0, 0.0],
    );
}

#[test]
fn gain_balance_with_normalize_matches_balance_no_clip() {
    let source = stereo_audio_buffer(vec![0.75, 0.75, 0.75, 0.75], vec![1.0, 0.0, 0.0, 0.0]);
    let mut balance_normalize = source.clone();
    let mut balance_no_clip = source;
    let normalize = parse_effect_chain(&["gain", "-B", "-n"]).unwrap();
    let no_clip = parse_effect_chain(&["gain", "-b"]).unwrap();

    normalize.process_buffer(&mut balance_normalize).unwrap();
    no_clip.process_buffer(&mut balance_no_clip).unwrap();

    assert_samples_close(
        balance_normalize.as_planar_f32(),
        balance_no_clip.as_planar_f32(),
    );
    assert_eq!(normalize.render_tokens(), ["gain", "-B", "-n", "0"]);
}

#[test]
fn gain_channel_scan_reports_non_finite_samples() {
    let mut audio = stereo_audio_buffer(vec![0.25, 0.5], vec![f32::NAN, 0.5]);
    let chain = parse_effect_chain(&["gain", "-e"]).unwrap();

    let error = chain.process_buffer(&mut audio).unwrap_err();

    assert_eq!(
        error,
        EffectChainError::CommandFailed {
            index: 0,
            command: EffectCommand::Gain(
                Gain::new(Decibels::new(0.0).unwrap())
                    .with_channel_mode(auralis_effects::GainChannelMode::Equalize)
            ),
            argument: "channel",
            source: EffectError::NonFiniteGainSample { sample_index: 2 },
        }
    );
}

#[test]
fn gain_channel_modes_match_under_forced_scalar_and_requested_simd() {
    let source = stereo_audio_buffer(vec![0.25, -0.5, 0.75], vec![0.125, -0.25, 0.25]);
    let mut scalar = source.clone();
    let mut simd = source;
    let chain = parse_effect_chain(&["gain", "-e", "-3"]).unwrap();

    chain
        .process_buffer_with_backend(&mut scalar, BackendKind::Scalar)
        .unwrap();
    chain
        .process_buffer_with_backend(&mut simd, BackendKind::Simd)
        .unwrap();

    assert_eq!(simd.as_planar_f32(), scalar.as_planar_f32());
}

fn stereo_audio_buffer(left: Vec<f32>, right: Vec<f32>) -> AudioBuffer {
    assert_eq!(left.len(), right.len());
    let spec = AudioSpec::new(
        SampleRate::new(48_000).unwrap(),
        ChannelCount::new(2).unwrap(),
        SampleFormat::Float32,
    );
    let frames = left.len();
    AudioBuffer::from_planar_f32(
        spec,
        FrameCount::new(u64::try_from(frames).unwrap()),
        [left, right].concat(),
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

//! High-level output policy facade tests.
#![allow(missing_docs)]

mod support;

use std::fs;

use auralis::{
    AudioFile, BackendKind, ChannelConversionError, ChannelConversionPolicy, Error,
    OutputLevelError, SampleRateConversionError, SampleRateConversionPolicy,
    convert_audio_channels, convert_audio_channels_with_backend, convert_audio_sample_rate,
    guard_audio_level, guard_audio_level_with_backend, normalize_audio_level,
    normalize_audio_level_with_backend,
};
use auralis_core::{ChannelCount, Decibels, FrameCount, SampleFormat, SampleRate};

use support::{
    assert_sample_bits_eq, assert_samples_close, audio_buffer, audio_buffer_with_shape,
    audio_buffer_with_spec, stereo_audio_buffer, temp_path,
};

#[test]
fn convert_audio_sample_rate_clones_matching_rate() {
    let source = audio_buffer(vec![0.25, -0.5]);

    let actual = convert_audio_sample_rate(&source, SampleRate::new(48_000).unwrap()).unwrap();

    assert_eq!(actual, source);
}

#[test]
fn convert_audio_sample_rate_downsamples_with_linear_policy() {
    let source =
        audio_buffer_with_spec(vec![0.0, 0.25, 0.5, 0.75], 48_000, 1, SampleFormat::Float32);

    let actual = convert_audio_sample_rate(&source, SampleRate::new(24_000).unwrap()).unwrap();

    assert_eq!(
        actual.spec().sample_rate(),
        SampleRate::new(24_000).unwrap()
    );
    assert_eq!(actual.frames(), FrameCount::new(2));
    assert_eq!(actual.channels(), ChannelCount::new(1).unwrap());
    assert_samples_close(actual.as_planar_f32(), &[0.0, 0.5]);
}

#[test]
fn convert_audio_sample_rate_upsamples_with_linear_policy() {
    let source = audio_buffer_with_spec(vec![0.0, 1.0], 24_000, 1, SampleFormat::Float32);

    let actual = convert_audio_sample_rate(&source, SampleRate::new(48_000).unwrap()).unwrap();

    assert_eq!(
        actual.spec().sample_rate(),
        SampleRate::new(48_000).unwrap()
    );
    assert_eq!(actual.frames(), FrameCount::new(4));
    assert_samples_close(actual.as_planar_f32(), &[0.0, 0.5, 1.0, 1.0]);
}

#[test]
fn convert_audio_sample_rate_preserves_channel_grouping() {
    let source = audio_buffer_with_shape(
        vec![0.0, 0.5, -1.0, -0.5],
        2,
        24_000,
        2,
        SampleFormat::Float32,
    );

    let actual = convert_audio_sample_rate(&source, SampleRate::new(48_000).unwrap()).unwrap();

    assert_eq!(actual.frames(), FrameCount::new(4));
    assert_eq!(actual.channels(), ChannelCount::new(2).unwrap());
    assert_samples_close(
        actual.as_planar_f32(),
        &[0.0, 0.25, 0.5, 0.5, -1.0, -0.75, -0.5, -0.5],
    );
}

#[test]
fn sample_rate_conversion_policy_require_disables_automatic_conversion() {
    let error = AudioFile::from_audio_buffer(audio_buffer(vec![0.25, -0.5]))
        .into_pipeline()
        .with_sample_rate_conversion_policy(SampleRateConversionPolicy::require(
            SampleRate::new(24_000).unwrap(),
        ))
        .write_wav(temp_path("auralis-rate-policy-reject", "wav"))
        .unwrap_err();

    assert_eq!(
        error,
        Error::SampleRateConversion(SampleRateConversionError::AutomaticConversionDisabled {
            actual: SampleRate::new(48_000).unwrap(),
            target: SampleRate::new(24_000).unwrap(),
        })
    );
}

#[test]
fn pipeline_write_wav_applies_explicit_output_sample_rate_policy() {
    let output = temp_path("auralis-rate-policy-output", "wav");
    let source = audio_buffer(vec![0.25, -0.5, 0.75, 0.0]);

    AudioFile::from_audio_buffer(source)
        .into_pipeline()
        .with_output_sample_rate(SampleRate::new(24_000).unwrap())
        .write_wav(&output)
        .unwrap();

    let decoded = auralis_wav::decode_pcm16_path(&output).unwrap();
    assert_eq!(
        decoded.spec().sample_rate(),
        SampleRate::new(24_000).unwrap()
    );
    assert_eq!(decoded.frames(), FrameCount::new(2));
    assert_samples_close(decoded.as_planar_f32(), &[0.25, 0.75]);
    fs::remove_file(output).unwrap();
}

#[test]
fn pipeline_into_audio_buffer_does_not_apply_output_sample_rate_policy() {
    let actual = AudioFile::from_audio_buffer(audio_buffer(vec![0.25, -0.5]))
        .into_pipeline()
        .with_output_sample_rate(SampleRate::new(24_000).unwrap())
        .into_audio_buffer()
        .unwrap();

    assert_eq!(
        actual.spec().sample_rate(),
        SampleRate::new(48_000).unwrap()
    );
    assert_eq!(actual.frames(), FrameCount::new(2));
    assert_samples_close(actual.as_planar_f32(), &[0.25, -0.5]);
}

#[test]
fn guard_audio_level_attenuates_only_when_peak_exceeds_full_scale() {
    let source = audio_buffer(vec![0.5, 2.0, -1.0]);

    let actual = guard_audio_level(&source).unwrap();

    assert_samples_close(actual.as_planar_f32(), &[0.25, 1.0, -0.5]);
}

#[test]
fn guard_audio_level_preserves_full_scale_or_quieter_audio() {
    let source = audio_buffer(vec![0.5, 1.0, -0.25]);

    let actual = guard_audio_level(&source).unwrap();

    assert_eq!(actual, source);
}

#[test]
fn normalize_audio_level_scales_non_silent_audio_to_target_peak() {
    let source = audio_buffer(vec![0.25, -0.5, 0.0]);

    let actual = normalize_audio_level(&source, Decibels::new(0.0).unwrap()).unwrap();

    assert_samples_close(actual.as_planar_f32(), &[0.5, -1.0, 0.0]);
}

#[test]
fn normalize_audio_level_preserves_silence() {
    let source = audio_buffer(vec![0.0, -0.0]);

    let actual = normalize_audio_level(&source, Decibels::new(0.0).unwrap()).unwrap();

    assert_sample_bits_eq(actual.as_planar_f32(), source.as_planar_f32());
}

#[test]
fn output_level_policy_rejects_non_finite_samples_before_encoding() {
    let error = AudioFile::from_audio_buffer(audio_buffer(vec![0.25, f32::NAN]))
        .into_pipeline()
        .with_output_guard()
        .write_wav(temp_path("auralis-output-level-non-finite", "wav"))
        .unwrap_err();

    assert_eq!(
        error,
        Error::OutputLevel(OutputLevelError::NonFiniteSample {
            channel_index: 0,
            frame_index: 1,
        })
    );
}

#[test]
fn output_level_policy_normalize_rejects_overflowing_target() {
    let error = normalize_audio_level(
        &audio_buffer(vec![0.25, -0.5]),
        Decibels::new(f64::MAX).unwrap(),
    )
    .unwrap_err();

    assert_eq!(
        error,
        OutputLevelError::TargetLevelOverflow {
            target: Decibels::new(f64::MAX).unwrap(),
        }
    );
}

#[test]
fn output_level_policy_matches_under_forced_scalar_and_requested_simd() {
    let source = audio_buffer(vec![-2.0, -1.0, -0.5, -0.0, 0.0, 0.25, 0.999_984_74, 2.0]);

    let scalar = guard_audio_level_with_backend(&source, BackendKind::Scalar).unwrap();
    let simd = guard_audio_level_with_backend(&source, BackendKind::Simd).unwrap();

    assert_sample_bits_eq(simd.as_planar_f32(), scalar.as_planar_f32());

    let scalar = normalize_audio_level_with_backend(
        &source,
        Decibels::new(-6.0).unwrap(),
        BackendKind::Scalar,
    )
    .unwrap();
    let simd = normalize_audio_level_with_backend(
        &source,
        Decibels::new(-6.0).unwrap(),
        BackendKind::Simd,
    )
    .unwrap();

    assert_sample_bits_eq(simd.as_planar_f32(), scalar.as_planar_f32());
}

#[test]
fn pipeline_write_wav_applies_explicit_output_guard_policy() {
    let output = temp_path("auralis-output-guard-policy", "wav");
    let source = audio_buffer(vec![2.0, -1.0]);

    AudioFile::from_audio_buffer(source)
        .into_pipeline()
        .with_output_guard()
        .write_wav(&output)
        .unwrap();

    let decoded = auralis_wav::decode_pcm16_path(&output).unwrap();
    assert_samples_close(decoded.as_planar_f32(), &[1.0, -0.5]);
    fs::remove_file(output).unwrap();
}

#[test]
fn pipeline_write_wav_applies_explicit_output_normalization_policy() {
    let output = temp_path("auralis-output-normalize-policy", "wav");
    let source = audio_buffer(vec![0.25, -0.5]);

    AudioFile::from_audio_buffer(source)
        .into_pipeline()
        .with_output_normalization(Decibels::new(0.0).unwrap())
        .write_wav(&output)
        .unwrap();

    let decoded = auralis_wav::decode_pcm16_path(&output).unwrap();
    assert_samples_close(decoded.as_planar_f32(), &[0.5, -1.0]);
    fs::remove_file(output).unwrap();
}

#[test]
fn pipeline_into_audio_buffer_does_not_apply_output_level_policy() {
    let actual = AudioFile::from_audio_buffer(audio_buffer(vec![0.25, -0.5]))
        .into_pipeline()
        .with_output_normalization(Decibels::new(0.0).unwrap())
        .into_audio_buffer()
        .unwrap();

    assert_samples_close(actual.as_planar_f32(), &[0.25, -0.5]);
}

#[test]
fn convert_audio_channels_downmixes_stereo_to_mono() {
    let source = stereo_audio_buffer(vec![1.0, -1.0, 0.0, 0.5]);

    let actual = convert_audio_channels(&source, ChannelCount::new(1).unwrap()).unwrap();

    assert_eq!(actual.frames(), FrameCount::new(2));
    assert_eq!(actual.channels(), ChannelCount::new(1).unwrap());
    assert_sample_bits_eq(actual.as_planar_f32(), &[0.5, -0.25]);
}

#[test]
fn convert_audio_channels_upmixes_mono_to_stereo_by_duplication() {
    let source = audio_buffer(vec![0.25, -0.5]);

    let actual = convert_audio_channels(&source, ChannelCount::new(2).unwrap()).unwrap();

    assert_eq!(actual.frames(), FrameCount::new(2));
    assert_eq!(actual.channels(), ChannelCount::new(2).unwrap());
    assert_sample_bits_eq(actual.as_planar_f32(), &[0.25, -0.5, 0.25, -0.5]);
}

#[test]
fn convert_audio_channels_downmixes_three_channels_with_sox_grouping() {
    let source = audio_buffer_with_shape(
        vec![1.0, 2.0, 10.0, 20.0, 3.0, 4.0],
        2,
        48_000,
        3,
        SampleFormat::Float32,
    );

    let actual = convert_audio_channels(&source, ChannelCount::new(2).unwrap()).unwrap();

    assert_eq!(actual.frames(), FrameCount::new(2));
    assert_eq!(actual.channels(), ChannelCount::new(2).unwrap());
    assert_sample_bits_eq(actual.as_planar_f32(), &[2.0, 3.0, 10.0, 20.0]);
}

#[test]
fn convert_audio_channels_matches_under_forced_scalar_and_requested_simd() {
    let source = audio_buffer_with_shape(
        vec![
            -1.0,
            -0.5,
            -0.0,
            0.0,
            0.5,
            1.0,
            0.999_984_74,
            -0.999_984_74,
            0.25,
            -0.25,
            0.75,
            -0.75,
        ],
        4,
        48_000,
        3,
        SampleFormat::Float32,
    );

    let scalar = convert_audio_channels_with_backend(
        &source,
        ChannelCount::new(1).unwrap(),
        BackendKind::Scalar,
    )
    .unwrap();
    let simd = convert_audio_channels_with_backend(
        &source,
        ChannelCount::new(1).unwrap(),
        BackendKind::Simd,
    )
    .unwrap();

    assert_sample_bits_eq(simd.as_planar_f32(), scalar.as_planar_f32());
}

#[test]
fn channel_conversion_policy_require_disables_automatic_conversion() {
    let source = stereo_audio_buffer(vec![0.25, -0.5, 0.75, 0.0]);

    let error = AudioFile::from_audio_buffer(source)
        .into_pipeline()
        .with_channel_conversion_policy(ChannelConversionPolicy::require(
            ChannelCount::new(1).unwrap(),
        ))
        .write_wav(temp_path("auralis-channel-policy-reject", "wav"))
        .unwrap_err();

    assert_eq!(
        error,
        Error::ChannelConversion(ChannelConversionError::AutomaticConversionDisabled {
            actual: ChannelCount::new(2).unwrap(),
            target: ChannelCount::new(1).unwrap(),
        })
    );
}

#[test]
fn pipeline_write_wav_applies_explicit_output_channel_policy() {
    let output = temp_path("auralis-channel-policy-output", "wav");
    let source = stereo_audio_buffer(vec![0.5, -0.5, 1.0, 0.0]);

    AudioFile::from_audio_buffer(source)
        .into_pipeline()
        .with_output_channels(ChannelCount::new(1).unwrap())
        .write_wav(&output)
        .unwrap();

    let decoded = auralis_wav::decode_pcm16_path(&output).unwrap();
    assert_eq!(decoded.channels(), ChannelCount::new(1).unwrap());
    assert_samples_close(decoded.as_planar_f32(), &[0.75, -0.25]);
    fs::remove_file(output).unwrap();
}

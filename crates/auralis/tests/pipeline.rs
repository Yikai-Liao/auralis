//! High-level pipeline facade tests.
#![allow(missing_docs)]

mod support;

use std::fs;

use auralis::{AudioFile, BackendKind, EffectChain, EffectCommand, Error, OutputFormat};
use auralis_codec::{
    CodecError, CodecKind, RawPcmEncodeOptions, WavEncodeOptions, WavSampleFormat,
};
use auralis_core::{Decibels, FrameCount};
use auralis_effects::{DcShift, Fade, Gain, Reverse, Trim};
use auralis_wav::decode_pcm16_path;

use support::{
    assert_sample_bits_eq, assert_samples_close, audio_buffer, stereo_audio_buffer, temp_dir,
};

#[test]
fn chain_gain_matches_direct_effect_execution() {
    let source = audio_buffer(vec![0.25, -0.5, 1.0]);
    let mut expected = source.clone();

    Gain::new(Decibels::new(-3.0).unwrap()).process_buffer(&mut expected);

    let actual = AudioFile::from_audio_buffer(source)
        .into_pipeline()
        .gain_db(-3.0)
        .into_audio_buffer()
        .unwrap();

    assert_samples_close(actual.as_planar_f32(), expected.as_planar_f32());
}

#[test]
fn chain_gain_matches_under_forced_scalar_and_requested_simd() {
    let source = audio_buffer(vec![
        -1.0,
        -0.999_984_74,
        -0.5,
        -0.0,
        0.0,
        0.5,
        0.999_984_74,
        1.0,
    ]);

    let scalar = AudioFile::from_audio_buffer(source.clone())
        .into_pipeline()
        .with_backend(BackendKind::Scalar)
        .gain_db(-3.0)
        .into_audio_buffer()
        .unwrap();
    let simd = AudioFile::from_audio_buffer(source)
        .into_pipeline()
        .with_backend(BackendKind::Simd)
        .gain_db(-3.0)
        .into_audio_buffer()
        .unwrap();

    assert_sample_bits_eq(simd.as_planar_f32(), scalar.as_planar_f32());
}

#[test]
fn chain_dc_shift_preserves_stereo_frame_grouping() {
    let source = stereo_audio_buffer(vec![0.75, 1.0, -0.75, -1.0]);

    let actual = AudioFile::from_audio_buffer(source)
        .into_pipeline()
        .dc_shift(0.5)
        .into_audio_buffer()
        .unwrap();

    assert_eq!(actual.frames(), FrameCount::new(2));
    assert_eq!(actual.as_planar_f32(), &[1.25, 1.5, -0.25, -0.5]);
}

#[test]
fn chain_dc_shift_matches_under_forced_scalar_and_requested_simd() {
    let source = audio_buffer(vec![
        -1.0,
        -0.999_984_74,
        -f32::MIN_POSITIVE,
        -f32::from_bits(1),
        -0.0,
        0.0,
        f32::from_bits(1),
        f32::MIN_POSITIVE,
        0.999_984_74,
        1.0,
    ]);

    let scalar = AudioFile::from_audio_buffer(source.clone())
        .into_pipeline()
        .with_backend(BackendKind::Scalar)
        .dc_shift(0.125)
        .into_audio_buffer()
        .unwrap();
    let simd = AudioFile::from_audio_buffer(source)
        .into_pipeline()
        .with_backend(BackendKind::Simd)
        .dc_shift(0.125)
        .into_audio_buffer()
        .unwrap();

    assert_sample_bits_eq(simd.as_planar_f32(), scalar.as_planar_f32());
}

#[test]
fn chain_trim_frames_preserves_stereo_frame_grouping() {
    let source = stereo_audio_buffer(vec![0.0, 0.25, 0.5, 0.75, 1.0, -0.25, -0.5, -0.75]);

    let actual = AudioFile::from_audio_buffer(source)
        .into_pipeline()
        .trim_frames(1, 3)
        .into_audio_buffer()
        .unwrap();

    assert_eq!(actual.frames(), FrameCount::new(2));
    assert_eq!(actual.as_planar_f32(), &[0.25, 0.5, -0.25, -0.5]);
}

#[test]
fn chain_trim_seconds_uses_floor_conversion() {
    let actual = AudioFile::from_audio_buffer(audio_buffer(vec![0.0, 0.25, 0.5, 0.75]))
        .into_pipeline()
        .trim_seconds(1.0 / 48_000.0, 3.9 / 48_000.0)
        .into_audio_buffer()
        .unwrap();

    assert_eq!(actual.frames(), FrameCount::new(2));
    assert_eq!(actual.as_planar_f32(), &[0.25, 0.5]);
}

#[test]
fn chain_pad_frames_preserves_stereo_frame_grouping() {
    let source = stereo_audio_buffer(vec![0.25, 0.5, -0.25, -0.5]);

    let actual = AudioFile::from_audio_buffer(source)
        .into_pipeline()
        .pad_frames(1, 1)
        .into_audio_buffer()
        .unwrap();

    assert_eq!(actual.frames(), FrameCount::new(4));
    assert_eq!(
        actual.as_planar_f32(),
        &[0.0, 0.25, 0.5, 0.0, 0.0, -0.25, -0.5, 0.0]
    );
}

#[test]
fn chain_fade_frames_preserves_stereo_frame_grouping() {
    let source = stereo_audio_buffer(vec![1.0, 1.0, 1.0, 1.0, -1.0, -1.0, -1.0, -1.0]);

    let actual = AudioFile::from_audio_buffer(source)
        .into_pipeline()
        .fade_frames(2, 2)
        .into_audio_buffer()
        .unwrap();

    assert_eq!(actual.frames(), FrameCount::new(4));
    assert_samples_close(
        actual.as_planar_f32(),
        &[0.0, 0.5, 0.5, 0.0, -0.0, -0.5, -0.5, -0.0],
    );
}

#[test]
fn chain_fade_frames_matches_under_forced_scalar_and_requested_simd() {
    let source = stereo_audio_buffer(vec![
        -1.0,
        -0.999_984_74,
        -0.5,
        -0.0,
        0.0,
        0.5,
        0.999_984_74,
        1.0,
        1.0,
        0.999_984_74,
        0.5,
        0.0,
        -0.0,
        -0.5,
        -0.999_984_74,
        -1.0,
    ]);

    let scalar = AudioFile::from_audio_buffer(source.clone())
        .into_pipeline()
        .with_backend(BackendKind::Scalar)
        .fade_frames(5, 7)
        .into_audio_buffer()
        .unwrap();
    let simd = AudioFile::from_audio_buffer(source)
        .into_pipeline()
        .with_backend(BackendKind::Simd)
        .fade_frames(5, 7)
        .into_audio_buffer()
        .unwrap();

    assert_sample_bits_eq(simd.as_planar_f32(), scalar.as_planar_f32());
}

#[test]
fn chain_reverse_preserves_stereo_frame_grouping() {
    let source = stereo_audio_buffer(vec![0.0, 0.25, 0.5, 1.0, -0.25, -0.5]);

    let actual = AudioFile::from_audio_buffer(source)
        .into_pipeline()
        .reverse()
        .into_audio_buffer()
        .unwrap();

    assert_eq!(actual.frames(), FrameCount::new(3));
    assert_eq!(actual.as_planar_f32(), &[0.5, 0.25, 0.0, -0.5, -0.25, 1.0]);
}

#[test]
fn apply_effect_chain_matches_repeated_fluent_pipeline_calls() {
    let source = stereo_audio_buffer(vec![0.25, -0.5, 0.75, 1.0, -0.25, 0.5, -0.75, -1.0]);
    let chain = EffectChain::new(vec![
        EffectCommand::Gain(Gain::new(Decibels::new(-3.0).unwrap())),
        EffectCommand::DcShift(DcShift::new(0.125).unwrap()),
        EffectCommand::Fade(Fade::new(FrameCount::new(2), FrameCount::new(2))),
        EffectCommand::Trim(Trim::new(FrameCount::new(1), FrameCount::new(3)).unwrap()),
        EffectCommand::Reverse(Reverse::new()),
    ]);

    let actual = AudioFile::from_audio_buffer(source.clone())
        .into_pipeline()
        .apply_effect_chain(&chain)
        .into_audio_buffer()
        .unwrap();
    let expected = AudioFile::from_audio_buffer(source)
        .into_pipeline()
        .gain_db(-3.0)
        .dc_shift(0.125)
        .fade_frames(2, 2)
        .trim_frames(1, 3)
        .reverse()
        .into_audio_buffer()
        .unwrap();

    assert_samples_close(actual.as_planar_f32(), expected.as_planar_f32());
    assert_eq!(actual.frames(), expected.frames());
    assert_eq!(actual.channels(), expected.channels());
}

#[test]
fn write_output_format_wav_matches_write_wav() {
    let source = stereo_audio_buffer(vec![0.25, -0.5, 0.75, 1.0, -0.25, 0.5]);
    let baseline_path = support::temp_path("auralis-pipeline-write-wav-baseline", "wav");
    let format_path = support::temp_path("auralis-pipeline-write-wav-format", "wav");

    AudioFile::from_audio_buffer(source.clone())
        .into_pipeline()
        .gain_db(-3.0)
        .write_wav(&baseline_path)
        .unwrap();
    let summary = AudioFile::from_audio_buffer(source)
        .into_pipeline()
        .gain_db(-3.0)
        .write(&format_path, OutputFormat::Wav(WavEncodeOptions::default()))
        .unwrap();

    let baseline = decode_pcm16_path(&baseline_path).unwrap();
    let format_output = decode_pcm16_path(&format_path).unwrap();

    fs::remove_file(baseline_path).unwrap();
    fs::remove_file(format_path).unwrap();
    assert_eq!(summary.codec_kind(), CodecKind::Wav);
    assert_eq!(summary.frames(), baseline.frames());
    assert_eq!(format_output, baseline);
}

#[test]
fn write_output_format_pcm8_wav_uses_pcm8_encoder() {
    let source = stereo_audio_buffer(vec![-1.0, 0.0, 0.5, 1.0]);
    let format_path = support::temp_path("auralis-pipeline-write-wav-pcm8", "wav");

    let summary = AudioFile::from_audio_buffer(source)
        .into_pipeline()
        .write(&format_path, OutputFormat::Wav(WavEncodeOptions::pcm8()))
        .unwrap();

    let decoded = auralis_wav::decode_pcm8_path(&format_path).unwrap();

    fs::remove_file(format_path).unwrap();
    assert_eq!(summary.codec_kind(), CodecKind::Wav);
    assert_eq!(summary.spec(), decoded.spec());
    assert_eq!(summary.frames(), decoded.frames());
    assert_eq!(decoded.channel(0).unwrap(), &[-1.0, 0.0]);
    assert_eq!(decoded.channel(1).unwrap(), &[0.5, 127.0 / 128.0]);
    assert_eq!(
        WavEncodeOptions::pcm8().sample_format(),
        WavSampleFormat::Pcm8
    );
}

#[test]
fn write_output_format_pcm24_wav_uses_pcm24_encoder() {
    let source = stereo_audio_buffer(vec![-1.0, 0.0, 0.5, 8_388_607.0 / 8_388_608.0]);
    let format_path = support::temp_path("auralis-pipeline-write-wav-pcm24", "wav");

    let summary = AudioFile::from_audio_buffer(source)
        .into_pipeline()
        .write(&format_path, OutputFormat::Wav(WavEncodeOptions::pcm24()))
        .unwrap();

    let decoded = auralis_wav::decode_pcm24_path(&format_path).unwrap();

    fs::remove_file(format_path).unwrap();
    assert_eq!(summary.codec_kind(), CodecKind::Wav);
    assert_eq!(summary.spec(), decoded.spec());
    assert_eq!(summary.frames(), decoded.frames());
    assert_eq!(decoded.channel(0).unwrap(), &[-1.0, 0.0]);
    assert_eq!(
        decoded.channel(1).unwrap(),
        &[0.5, 8_388_607.0 / 8_388_608.0]
    );
    assert_eq!(
        WavEncodeOptions::pcm24().sample_format(),
        WavSampleFormat::Pcm24
    );
}

#[test]
fn write_output_format_pcm32_wav_uses_pcm32_encoder() {
    let source = stereo_audio_buffer(vec![-1.0, 0.0, 0.5, 1.0]);
    let format_path = support::temp_path("auralis-pipeline-write-wav-pcm32", "wav");

    let summary = AudioFile::from_audio_buffer(source)
        .into_pipeline()
        .write(&format_path, OutputFormat::Wav(WavEncodeOptions::pcm32()))
        .unwrap();

    let decoded = auralis_wav::decode_pcm32_path(&format_path).unwrap();

    fs::remove_file(format_path).unwrap();
    assert_eq!(summary.codec_kind(), CodecKind::Wav);
    assert_eq!(summary.spec(), decoded.spec());
    assert_eq!(summary.frames(), decoded.frames());
    assert_eq!(decoded.channel(0).unwrap(), &[-1.0, 0.0]);
    assert_eq!(decoded.channel(1).unwrap(), &[0.5, 1.0]);
    assert_eq!(
        WavEncodeOptions::pcm32().sample_format(),
        WavSampleFormat::Pcm32
    );
}

#[test]
fn write_output_format_float32_wav_uses_float32_encoder() {
    let source = stereo_audio_buffer(vec![-1.25, 0.0, 0.5, 1.25]);
    let format_path = support::temp_path("auralis-pipeline-write-wav-float32", "wav");

    let summary = AudioFile::from_audio_buffer(source)
        .into_pipeline()
        .write(&format_path, OutputFormat::Wav(WavEncodeOptions::float32()))
        .unwrap();

    let decoded = auralis_wav::decode_float32_path(&format_path).unwrap();

    fs::remove_file(format_path).unwrap();
    assert_eq!(summary.codec_kind(), CodecKind::Wav);
    assert_eq!(summary.spec(), decoded.spec());
    assert_eq!(summary.frames(), decoded.frames());
    assert_eq!(decoded.channel(0).unwrap(), &[-1.25, 0.0]);
    assert_eq!(decoded.channel(1).unwrap(), &[0.5, 1.25]);
    assert_eq!(
        WavEncodeOptions::float32().sample_format(),
        WavSampleFormat::Float32
    );
}

#[test]
fn write_output_format_float64_wav_uses_float64_encoder() {
    let source = stereo_audio_buffer(vec![-1.5, 0.0, 0.5, 1.5]);
    let format_path = support::temp_path("auralis-pipeline-write-wav-float64", "wav");

    let summary = AudioFile::from_audio_buffer(source)
        .into_pipeline()
        .write(&format_path, OutputFormat::Wav(WavEncodeOptions::float64()))
        .unwrap();

    let decoded = auralis_wav::decode_float64_path(&format_path).unwrap();

    fs::remove_file(format_path).unwrap();
    assert_eq!(summary.codec_kind(), CodecKind::Wav);
    assert_eq!(summary.spec(), decoded.spec());
    assert_eq!(summary.frames(), decoded.frames());
    assert_eq!(decoded.channel(0).unwrap(), &[-1.5, 0.0]);
    assert_eq!(decoded.channel(1).unwrap(), &[0.5, 1.5]);
    assert_eq!(
        WavEncodeOptions::float64().sample_format(),
        WavSampleFormat::Float64
    );
}

#[test]
fn write_output_format_rejects_unsupported_formats() {
    let path = support::temp_path("auralis-pipeline-write-raw", "raw");
    let error = AudioFile::from_audio_buffer(audio_buffer(vec![0.0, 0.25]))
        .into_pipeline()
        .write(&path, OutputFormat::RawPcm(RawPcmEncodeOptions))
        .unwrap_err();

    let _ = fs::remove_file(path);
    assert_eq!(
        error,
        Error::Codec(CodecError::UnsupportedFormat(
            auralis_codec::UnsupportedFormat::new(CodecKind::RawPcm)
        ))
    );
}

#[test]
fn apply_effect_chain_matches_under_forced_scalar_and_requested_simd() {
    let source = stereo_audio_buffer(vec![
        -1.0,
        -0.999_984_74,
        -0.5,
        -0.0,
        0.0,
        0.5,
        0.999_984_74,
        1.0,
        1.0,
        0.999_984_74,
        0.5,
        0.0,
        -0.0,
        -0.5,
        -0.999_984_74,
        -1.0,
    ]);
    let chain = EffectChain::new(vec![
        EffectCommand::Gain(Gain::new(Decibels::new(-3.0).unwrap())),
        EffectCommand::DcShift(DcShift::new(0.125).unwrap()),
        EffectCommand::Fade(Fade::new(FrameCount::new(5), FrameCount::new(7))),
        EffectCommand::Reverse(Reverse::new()),
    ]);

    let scalar = AudioFile::from_audio_buffer(source.clone())
        .into_pipeline()
        .with_backend(BackendKind::Scalar)
        .apply_effect_chain(&chain)
        .into_audio_buffer()
        .unwrap();
    let simd = AudioFile::from_audio_buffer(source)
        .into_pipeline()
        .with_backend(BackendKind::Simd)
        .apply_effect_chain(&chain)
        .into_audio_buffer()
        .unwrap();

    assert_sample_bits_eq(simd.as_planar_f32(), scalar.as_planar_f32());
}

#[test]
fn apply_effect_chain_errors_include_failing_command_context() {
    let chain = EffectChain::new(vec![EffectCommand::Trim(
        Trim::new(FrameCount::new(0), FrameCount::new(3)).unwrap(),
    )]);

    let error = AudioFile::from_audio_buffer(audio_buffer(vec![0.25, -0.5]))
        .into_pipeline()
        .apply_effect_chain(&chain)
        .into_audio_buffer()
        .unwrap_err();

    assert!(matches!(
        error,
        Error::Chain(auralis_effects::EffectChainError::CommandFailed {
            index: 0,
            ref command,
            argument: "frame-range",
            source: auralis_effects::EffectError::TrimRangeOutOfBounds,
        }) if matches!(command.as_ref(), EffectCommand::Trim(_))
    ));
    assert_eq!(
        error.to_string(),
        "effect chain command 0 (`trim 0 3`) failed while applying `frame-range`: trim frame range must be within the input duration"
    );
}

#[test]
fn invalid_trim_range_propagates_without_panic() {
    let error = AudioFile::from_audio_buffer(audio_buffer(vec![0.25]))
        .into_pipeline()
        .trim_frames(2, 1)
        .into_audio_buffer()
        .unwrap_err();

    assert_eq!(
        error,
        Error::Effect(auralis_effects::EffectError::InvalidTrimOrder)
    );
}

#[test]
fn invalid_trim_seconds_propagates_without_panic() {
    let error = AudioFile::from_audio_buffer(audio_buffer(vec![0.25]))
        .into_pipeline()
        .trim_seconds(f64::NAN, 1.0)
        .into_audio_buffer()
        .unwrap_err();

    assert_eq!(
        error,
        Error::Core(auralis_core::AuralisError::InvalidTimeSeconds)
    );
}

#[test]
fn invalid_gain_propagates_without_panic() {
    let error = AudioFile::from_audio_buffer(audio_buffer(vec![0.25]))
        .into_pipeline()
        .gain_db(f64::NAN)
        .into_audio_buffer()
        .unwrap_err();

    assert_eq!(
        error,
        Error::Core(auralis_core::AuralisError::InvalidDecibels)
    );
}

#[test]
fn invalid_dc_shift_propagates_without_panic() {
    let error = AudioFile::from_audio_buffer(audio_buffer(vec![0.25]))
        .into_pipeline()
        .dc_shift(f32::NAN)
        .into_audio_buffer()
        .unwrap_err();

    assert_eq!(
        error,
        Error::Effect(auralis_effects::EffectError::InvalidDcShift)
    );
}

#[test]
fn wav_chain_round_trips_through_file_boundary() {
    let tempdir = temp_dir();
    fs::create_dir(&tempdir).unwrap();
    let input = tempdir.join("input.wav");
    let output = tempdir.join("output.wav");
    let source = audio_buffer(vec![0.25, -0.5, 0.75]);

    auralis_wav::encode_pcm16_path(&input, &source).unwrap();

    AudioFile::open_wav(&input)
        .unwrap()
        .into_pipeline()
        .gain_db(0.0)
        .write_wav(&output)
        .unwrap();

    let decoded = auralis_wav::decode_pcm16_path(output).unwrap();
    assert_samples_close(decoded.as_planar_f32(), source.as_planar_f32());
    fs::remove_dir_all(tempdir).unwrap();
}

#[test]
fn open_wav_accepts_pcm24_input() {
    let tempdir = temp_dir();
    fs::create_dir(&tempdir).unwrap();
    let input = tempdir.join("input-pcm24.wav");
    let output = tempdir.join("output-pcm24.wav");
    let source = audio_buffer(vec![-1.0, 0.0, 8_388_607.0 / 8_388_608.0]);

    auralis_wav::encode_pcm24_path(&input, &source).unwrap();

    AudioFile::open_wav(&input)
        .unwrap()
        .into_pipeline()
        .gain_db(0.0)
        .write(&output, OutputFormat::Wav(WavEncodeOptions::pcm24()))
        .unwrap();

    let decoded = auralis_wav::decode_pcm24_path(output).unwrap();
    assert_samples_close(decoded.as_planar_f32(), source.as_planar_f32());
    fs::remove_dir_all(tempdir).unwrap();
}

#[test]
fn open_wav_accepts_float64_input() {
    let tempdir = temp_dir();
    fs::create_dir(&tempdir).unwrap();
    let input = tempdir.join("input-float64.wav");
    let output = tempdir.join("output-float64.wav");
    let source = audio_buffer(vec![-1.5, 0.0, 1.5]);

    auralis_wav::encode_float64_path(&input, &source).unwrap();

    AudioFile::open_wav(&input)
        .unwrap()
        .into_pipeline()
        .gain_db(0.0)
        .write(&output, OutputFormat::Wav(WavEncodeOptions::float64()))
        .unwrap();

    let decoded = auralis_wav::decode_float64_path(output).unwrap();
    assert_samples_close(decoded.as_planar_f32(), source.as_planar_f32());
    fs::remove_dir_all(tempdir).unwrap();
}

#[test]
fn deferred_error_prevents_later_processing() {
    let error = AudioFile::from_audio_buffer(audio_buffer(vec![0.25]))
        .into_pipeline()
        .gain_db(f64::INFINITY)
        .gain_db(6.0)
        .into_audio_buffer()
        .unwrap_err();

    assert_eq!(
        error,
        Error::Core(auralis_core::AuralisError::InvalidDecibels)
    );
}

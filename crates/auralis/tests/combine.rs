//! High-level input combiner facade tests.
#![allow(missing_docs)]

mod support;

use std::fs;

use auralis::{
    AudioFile, BackendKind, InputCombineError, concatenate_audio_buffers, merge_audio_buffers,
    mix_audio_buffers, mix_audio_buffers_with_backend, mix_power_audio_buffers,
    mix_power_audio_buffers_with_backend, multiply_audio_buffers,
    multiply_audio_buffers_with_backend, sequence_audio_buffers,
};
use auralis_core::{ChannelCount, FrameCount, SampleFormat, SampleRate};

use support::{
    assert_sample_bits_eq, assert_samples_close, audio_buffer, audio_buffer_with_spec,
    stereo_audio_buffer, temp_dir,
};

#[test]
fn concatenate_audio_buffers_accepts_mismatched_mono_lengths() {
    let first = audio_buffer(vec![0.25, -0.5, 0.75]);
    let second = audio_buffer(vec![1.0]);

    let actual = concatenate_audio_buffers(&[first, second]).unwrap();

    assert_eq!(actual.frames(), FrameCount::new(4));
    assert_eq!(actual.channels(), ChannelCount::new(1).unwrap());
    assert_eq!(actual.as_planar_f32(), &[0.25, -0.5, 0.75, 1.0]);
}

#[test]
fn concatenate_audio_buffers_preserves_stereo_channel_grouping() {
    let first = stereo_audio_buffer(vec![1.0, 2.0, -1.0, -2.0]);
    let second = stereo_audio_buffer(vec![3.0, -3.0]);

    let actual = concatenate_audio_buffers(&[first, second]).unwrap();

    assert_eq!(actual.frames(), FrameCount::new(3));
    assert_eq!(actual.channels(), ChannelCount::new(2).unwrap());
    assert_eq!(actual.as_planar_f32(), &[1.0, 2.0, 3.0, -1.0, -2.0, -3.0]);
}

#[test]
fn concatenate_audio_buffers_rejects_mismatched_channel_count() {
    let first = audio_buffer(vec![0.25, -0.5]);
    let second = stereo_audio_buffer(vec![1.0, 2.0, -1.0, -2.0]);

    let error = concatenate_audio_buffers(&[first, second]).unwrap_err();

    assert_eq!(
        error,
        InputCombineError::MismatchedChannelCount {
            input_index: 1,
            expected: ChannelCount::new(1).unwrap(),
            actual: ChannelCount::new(2).unwrap(),
        }
    );
}

#[test]
fn concatenate_audio_buffers_rejects_mismatched_sample_rate() {
    let first = audio_buffer(vec![0.25]);
    let second = audio_buffer_with_spec(vec![-0.25], 44_100, 1, SampleFormat::Float32);

    let error = concatenate_audio_buffers(&[first, second]).unwrap_err();

    assert_eq!(
        error,
        InputCombineError::MismatchedSampleRate {
            input_index: 1,
            expected: SampleRate::new(48_000).unwrap(),
            actual: SampleRate::new(44_100).unwrap(),
        }
    );
}

#[test]
fn concatenate_audio_buffers_rejects_mismatched_sample_format() {
    let first = audio_buffer(vec![0.25]);
    let second = audio_buffer_with_spec(vec![-0.25], 48_000, 1, SampleFormat::Pcm16);

    let error = concatenate_audio_buffers(&[first, second]).unwrap_err();

    assert_eq!(
        error,
        InputCombineError::MismatchedSampleFormat {
            input_index: 1,
            expected: SampleFormat::Float32,
            actual: SampleFormat::Pcm16,
        }
    );
}

#[test]
fn concatenated_audio_enters_effect_pipeline_before_effects() {
    let first = audio_buffer(vec![0.25, -0.5]);
    let second = audio_buffer(vec![0.75]);

    let actual = AudioFile::from_audio_buffers_concatenated(&[first, second])
        .unwrap()
        .into_pipeline()
        .gain_db(6.0)
        .reverse()
        .into_audio_buffer()
        .unwrap();

    let multiplier = 10.0_f32.powf(6.0 / 20.0);
    assert_samples_close(
        actual.as_planar_f32(),
        &[0.75 * multiplier, -0.5 * multiplier, 0.25 * multiplier],
    );
}

#[test]
fn open_wavs_concatenated_round_trips_through_file_boundary() {
    let tempdir = temp_dir();
    fs::create_dir(&tempdir).unwrap();
    let first = tempdir.join("first.wav");
    let second = tempdir.join("second.wav");
    let output = tempdir.join("output.wav");

    auralis_wav::encode_pcm16_path(&first, &audio_buffer(vec![0.25, -0.5])).unwrap();
    auralis_wav::encode_pcm16_path(&second, &audio_buffer(vec![0.75])).unwrap();

    AudioFile::open_wavs_concatenated([&first, &second])
        .unwrap()
        .into_pipeline()
        .write_wav(&output)
        .unwrap();

    let decoded = auralis_wav::decode_pcm16_path(output).unwrap();
    assert_samples_close(decoded.as_planar_f32(), &[0.25, -0.5, 0.75]);
    fs::remove_dir_all(tempdir).unwrap();
}

#[test]
fn sequence_audio_buffers_accepts_mismatched_mono_lengths() {
    let first = audio_buffer(vec![0.25, -0.5]);
    let second = audio_buffer(vec![0.75, 0.0, -0.25]);

    let actual = sequence_audio_buffers(&[first, second]).unwrap();

    assert_eq!(actual.frames(), FrameCount::new(5));
    assert_eq!(actual.channels(), ChannelCount::new(1).unwrap());
    assert_eq!(actual.as_planar_f32(), &[0.25, -0.5, 0.75, 0.0, -0.25]);
}

#[test]
fn sequence_audio_buffers_preserves_stereo_channel_grouping() {
    let first = stereo_audio_buffer(vec![1.0, 2.0, -1.0, -2.0]);
    let second = stereo_audio_buffer(vec![3.0, -3.0]);

    let actual = sequence_audio_buffers(&[first, second]).unwrap();

    assert_eq!(actual.frames(), FrameCount::new(3));
    assert_eq!(actual.channels(), ChannelCount::new(2).unwrap());
    assert_eq!(actual.as_planar_f32(), &[1.0, 2.0, 3.0, -1.0, -2.0, -3.0]);
}

#[test]
fn sequence_audio_buffers_rejects_unrepresentable_channel_boundary() {
    let first = audio_buffer(vec![0.25, -0.5]);
    let second = stereo_audio_buffer(vec![1.0, 2.0, -1.0, -2.0]);

    let error = sequence_audio_buffers(&[first, second]).unwrap_err();

    assert_eq!(
        error,
        InputCombineError::SequenceBoundaryChannelCount {
            input_index: 1,
            previous_index: 0,
            expected: ChannelCount::new(1).unwrap(),
            actual: ChannelCount::new(2).unwrap(),
        }
    );
}

#[test]
fn sequence_audio_buffers_rejects_unrepresentable_sample_rate_boundary() {
    let first = audio_buffer(vec![0.25]);
    let second = audio_buffer_with_spec(vec![-0.25], 44_100, 1, SampleFormat::Float32);

    let error = sequence_audio_buffers(&[first, second]).unwrap_err();

    assert_eq!(
        error,
        InputCombineError::SequenceBoundarySampleRate {
            input_index: 1,
            previous_index: 0,
            expected: SampleRate::new(48_000).unwrap(),
            actual: SampleRate::new(44_100).unwrap(),
        }
    );
}

#[test]
fn sequenced_audio_enters_effect_pipeline_before_effects() {
    let first = audio_buffer(vec![0.25, -0.5]);
    let second = audio_buffer(vec![0.75]);

    let actual = AudioFile::from_audio_buffers_sequenced(&[first, second])
        .unwrap()
        .into_pipeline()
        .gain_db(6.0)
        .reverse()
        .into_audio_buffer()
        .unwrap();

    let multiplier = 10.0_f32.powf(6.0 / 20.0);
    assert_samples_close(
        actual.as_planar_f32(),
        &[0.75 * multiplier, -0.5 * multiplier, 0.25 * multiplier],
    );
}

#[test]
fn open_wavs_sequenced_round_trips_through_file_boundary() {
    let tempdir = temp_dir();
    fs::create_dir(&tempdir).unwrap();
    let first = tempdir.join("first.wav");
    let second = tempdir.join("second.wav");
    let output = tempdir.join("output.wav");

    auralis_wav::encode_pcm16_path(&first, &audio_buffer(vec![0.25, -0.5])).unwrap();
    auralis_wav::encode_pcm16_path(&second, &audio_buffer(vec![0.75])).unwrap();

    AudioFile::open_wavs_sequenced([&first, &second])
        .unwrap()
        .into_pipeline()
        .write_wav(&output)
        .unwrap();

    let decoded = auralis_wav::decode_pcm16_path(output).unwrap();
    assert_samples_close(decoded.as_planar_f32(), &[0.25, -0.5, 0.75]);
    fs::remove_dir_all(tempdir).unwrap();
}

#[test]
fn mix_audio_buffers_averages_equal_length_mono_inputs() {
    let first = audio_buffer(vec![1.0, -1.0, 0.5]);
    let second = audio_buffer(vec![0.5, 1.0, -0.5]);

    let actual = mix_audio_buffers(&[first, second]).unwrap();

    assert_eq!(actual.frames(), FrameCount::new(3));
    assert_eq!(actual.channels(), ChannelCount::new(1).unwrap());
    assert_sample_bits_eq(actual.as_planar_f32(), &[0.75, 0.0, 0.0]);
}

#[test]
fn mix_audio_buffers_treats_mismatched_lengths_as_trailing_silence() {
    let first = audio_buffer(vec![0.5, -0.5]);
    let second = audio_buffer(vec![1.0, 1.0, 1.0]);

    let actual = mix_audio_buffers(&[first, second]).unwrap();

    assert_eq!(actual.frames(), FrameCount::new(3));
    assert_sample_bits_eq(actual.as_planar_f32(), &[0.75, 0.25, 0.5]);
}

#[test]
fn mix_audio_buffers_preserves_stereo_channel_grouping() {
    let first = stereo_audio_buffer(vec![0.5, -0.5, 1.0, -1.0]);
    let second = stereo_audio_buffer(vec![0.5, 0.5, -0.5, -0.5]);

    let actual = mix_audio_buffers(&[first, second]).unwrap();

    assert_eq!(actual.frames(), FrameCount::new(2));
    assert_eq!(actual.channels(), ChannelCount::new(2).unwrap());
    assert_sample_bits_eq(actual.as_planar_f32(), &[0.5, 0.0, 0.25, -0.75]);
}

#[test]
fn mix_audio_buffers_accepts_mismatched_channel_counts_with_silence() {
    let mono = audio_buffer(vec![0.5, -0.5]);
    let stereo = stereo_audio_buffer(vec![1.0, 0.0, -1.0, 0.5]);

    let actual = mix_audio_buffers(&[mono, stereo]).unwrap();

    assert_eq!(actual.frames(), FrameCount::new(2));
    assert_eq!(actual.channels(), ChannelCount::new(2).unwrap());
    assert_sample_bits_eq(actual.as_planar_f32(), &[0.75, -0.25, -0.5, 0.25]);
}

#[test]
fn mix_audio_buffers_rejects_mismatched_sample_rate() {
    let first = audio_buffer(vec![0.25]);
    let second = audio_buffer_with_spec(vec![-0.25], 44_100, 1, SampleFormat::Float32);

    let error = mix_audio_buffers(&[first, second]).unwrap_err();

    assert_eq!(
        error,
        InputCombineError::MismatchedSampleRate {
            input_index: 1,
            expected: SampleRate::new(48_000).unwrap(),
            actual: SampleRate::new(44_100).unwrap(),
        }
    );
}

#[test]
fn mix_audio_buffers_rejects_mismatched_sample_format() {
    let first = audio_buffer(vec![0.25]);
    let second = audio_buffer_with_spec(vec![-0.25], 48_000, 1, SampleFormat::Pcm16);

    let error = mix_audio_buffers(&[first, second]).unwrap_err();

    assert_eq!(
        error,
        InputCombineError::MismatchedSampleFormat {
            input_index: 1,
            expected: SampleFormat::Float32,
            actual: SampleFormat::Pcm16,
        }
    );
}

#[test]
fn mix_audio_buffers_matches_under_forced_scalar_and_requested_simd() {
    let first = audio_buffer(vec![
        -1.0,
        -0.999_984_74,
        -0.5,
        -0.0,
        0.0,
        0.5,
        0.999_984_74,
        1.0,
    ]);
    let second = audio_buffer(vec![1.0, 0.999_984_74, 0.5, 0.0, -0.0]);

    let scalar =
        mix_audio_buffers_with_backend(&[first.clone(), second.clone()], BackendKind::Scalar)
            .unwrap();
    let simd = mix_audio_buffers_with_backend(&[first, second], BackendKind::Simd).unwrap();

    assert_sample_bits_eq(simd.as_planar_f32(), scalar.as_planar_f32());
}

#[test]
fn mixed_audio_enters_effect_pipeline_before_effects() {
    let first = audio_buffer(vec![0.25, -0.5]);
    let second = audio_buffer(vec![0.75, 0.0, -0.25]);

    let actual = AudioFile::from_audio_buffers_mixed(&[first, second])
        .unwrap()
        .into_pipeline()
        .gain_db(6.0)
        .reverse()
        .into_audio_buffer()
        .unwrap();

    let multiplier = 10.0_f32.powf(6.0 / 20.0);
    assert_samples_close(
        actual.as_planar_f32(),
        &[
            (-0.25 * 0.5) * multiplier,
            (-0.5 * 0.5) * multiplier,
            0.5 * multiplier,
        ],
    );
}

#[test]
fn mixed_audio_is_not_clipped_until_wav_write_boundary() {
    let tempdir = temp_dir();
    fs::create_dir(&tempdir).unwrap();
    let output = tempdir.join("output.wav");
    let first = audio_buffer(vec![2.0]);
    let second = audio_buffer(vec![2.0]);

    let mixed = mix_audio_buffers(&[first, second]).unwrap();

    assert_sample_bits_eq(mixed.as_planar_f32(), &[2.0]);
    AudioFile::from_audio_buffer(mixed)
        .into_pipeline()
        .write_wav(&output)
        .unwrap();
    let decoded = auralis_wav::decode_pcm16_path(output).unwrap();
    assert_samples_close(decoded.as_planar_f32(), &[f32::from(i16::MAX) / 32768.0]);
    fs::remove_dir_all(tempdir).unwrap();
}

#[test]
fn open_wavs_mixed_round_trips_through_file_boundary() {
    let tempdir = temp_dir();
    fs::create_dir(&tempdir).unwrap();
    let first = tempdir.join("first.wav");
    let second = tempdir.join("second.wav");
    let output = tempdir.join("output.wav");

    auralis_wav::encode_pcm16_path(&first, &audio_buffer(vec![0.25, -0.5])).unwrap();
    auralis_wav::encode_pcm16_path(&second, &audio_buffer(vec![0.75, 0.0, -0.25])).unwrap();

    AudioFile::open_wavs_mixed([&first, &second])
        .unwrap()
        .into_pipeline()
        .write_wav(&output)
        .unwrap();

    let decoded = auralis_wav::decode_pcm16_path(output).unwrap();
    assert_samples_close(decoded.as_planar_f32(), &[0.5, -0.25, -0.125]);
    fs::remove_dir_all(tempdir).unwrap();
}

#[test]
fn mix_power_audio_buffers_uses_equal_power_scale_for_equal_length_mono_inputs() {
    let first = audio_buffer(vec![1.0, -1.0, 0.5]);
    let second = audio_buffer(vec![0.5, 1.0, -0.5]);
    let scale = 1.0_f32 / 2.0_f32.sqrt();

    let actual = mix_power_audio_buffers(&[first, second]).unwrap();

    assert_eq!(actual.frames(), FrameCount::new(3));
    assert_eq!(actual.channels(), ChannelCount::new(1).unwrap());
    assert_sample_bits_eq(
        actual.as_planar_f32(),
        &[
            1.0 * scale + 0.5 * scale,
            -scale + scale,
            0.5 * scale - 0.5 * scale,
        ],
    );
}

#[test]
fn mix_power_audio_buffers_treats_mismatched_lengths_as_trailing_silence() {
    let first = audio_buffer(vec![0.5, -0.5]);
    let second = audio_buffer(vec![1.0, 1.0, 1.0]);
    let scale = 1.0_f32 / 2.0_f32.sqrt();

    let actual = mix_power_audio_buffers(&[first, second]).unwrap();

    assert_eq!(actual.frames(), FrameCount::new(3));
    assert_sample_bits_eq(
        actual.as_planar_f32(),
        &[
            0.5 * scale + 1.0 * scale,
            -0.5 * scale + 1.0 * scale,
            1.0 * scale,
        ],
    );
}

#[test]
fn mix_power_audio_buffers_accepts_mismatched_channel_counts_with_silence() {
    let mono = audio_buffer(vec![0.5, -0.5]);
    let stereo = stereo_audio_buffer(vec![1.0, 0.0, -1.0, 0.5]);
    let scale = 1.0_f32 / 2.0_f32.sqrt();

    let actual = mix_power_audio_buffers(&[mono, stereo]).unwrap();

    assert_eq!(actual.frames(), FrameCount::new(2));
    assert_eq!(actual.channels(), ChannelCount::new(2).unwrap());
    assert_sample_bits_eq(
        actual.as_planar_f32(),
        &[0.5 * scale + 1.0 * scale, -0.5 * scale, -scale, 0.5 * scale],
    );
}

#[test]
fn mix_power_audio_buffers_rejects_mismatched_sample_rate() {
    let first = audio_buffer(vec![0.25]);
    let second = audio_buffer_with_spec(vec![-0.25], 44_100, 1, SampleFormat::Float32);

    let error = mix_power_audio_buffers(&[first, second]).unwrap_err();

    assert_eq!(
        error,
        InputCombineError::MismatchedSampleRate {
            input_index: 1,
            expected: SampleRate::new(48_000).unwrap(),
            actual: SampleRate::new(44_100).unwrap(),
        }
    );
}

#[test]
fn mix_power_audio_buffers_matches_under_forced_scalar_and_requested_simd() {
    let first = audio_buffer(vec![
        -1.0,
        -0.999_984_74,
        -0.5,
        -0.0,
        0.0,
        0.5,
        0.999_984_74,
        1.0,
    ]);
    let second = audio_buffer(vec![1.0, 0.999_984_74, 0.5, 0.0, -0.0]);

    let scalar =
        mix_power_audio_buffers_with_backend(&[first.clone(), second.clone()], BackendKind::Scalar)
            .unwrap();
    let simd = mix_power_audio_buffers_with_backend(&[first, second], BackendKind::Simd).unwrap();

    assert_sample_bits_eq(simd.as_planar_f32(), scalar.as_planar_f32());
}

#[test]
fn mix_powered_audio_enters_effect_pipeline_before_effects() {
    let first = audio_buffer(vec![0.25, -0.5]);
    let second = audio_buffer(vec![0.75, 0.0, -0.25]);
    let scale = 1.0_f32 / 2.0_f32.sqrt();

    let actual = AudioFile::from_audio_buffers_mix_powered(&[first, second])
        .unwrap()
        .into_pipeline()
        .gain_db(6.0)
        .reverse()
        .into_audio_buffer()
        .unwrap();

    let multiplier = 10.0_f32.powf(6.0 / 20.0);
    assert_samples_close(
        actual.as_planar_f32(),
        &[
            (-0.25 * scale) * multiplier,
            (-0.5 * scale) * multiplier,
            (0.25 * scale + 0.75 * scale) * multiplier,
        ],
    );
}

#[test]
fn open_wavs_mix_powered_round_trips_through_file_boundary() {
    let tempdir = temp_dir();
    fs::create_dir(&tempdir).unwrap();
    let first = tempdir.join("first.wav");
    let second = tempdir.join("second.wav");
    let output = tempdir.join("output.wav");

    auralis_wav::encode_pcm16_path(&first, &audio_buffer(vec![0.25, -0.5])).unwrap();
    auralis_wav::encode_pcm16_path(&second, &audio_buffer(vec![0.75, 0.0, -0.25])).unwrap();

    AudioFile::open_wavs_mix_powered([&first, &second])
        .unwrap()
        .into_pipeline()
        .write_wav(&output)
        .unwrap();

    let scale = 1.0_f32 / 2.0_f32.sqrt();
    let decoded = auralis_wav::decode_pcm16_path(output).unwrap();
    assert_samples_close(
        decoded.as_planar_f32(),
        &[scale, -0.5 * scale, -0.25 * scale],
    );
    fs::remove_dir_all(tempdir).unwrap();
}

#[test]
fn merge_audio_buffers_turns_two_mono_inputs_into_stereo() {
    let first = audio_buffer(vec![0.25, -0.5]);
    let second = audio_buffer(vec![0.75, 0.0]);

    let actual = merge_audio_buffers(&[first, second]).unwrap();

    assert_eq!(actual.frames(), FrameCount::new(2));
    assert_eq!(actual.channels(), ChannelCount::new(2).unwrap());
    assert_eq!(actual.as_planar_f32(), &[0.25, -0.5, 0.75, 0.0]);
}

#[test]
fn merge_audio_buffers_preserves_multichannel_input_order() {
    let stereo = stereo_audio_buffer(vec![1.0, 2.0, -1.0, -2.0]);
    let mono = audio_buffer(vec![0.5, -0.5]);

    let actual = merge_audio_buffers(&[stereo, mono]).unwrap();

    assert_eq!(actual.frames(), FrameCount::new(2));
    assert_eq!(actual.channels(), ChannelCount::new(3).unwrap());
    assert_eq!(actual.as_planar_f32(), &[1.0, 2.0, -1.0, -2.0, 0.5, -0.5]);
}

#[test]
fn merge_audio_buffers_treats_mismatched_lengths_as_trailing_silence() {
    let first = audio_buffer(vec![0.25, -0.5]);
    let second = audio_buffer(vec![0.75, 0.0, -0.25]);

    let actual = merge_audio_buffers(&[first, second]).unwrap();

    assert_eq!(actual.frames(), FrameCount::new(3));
    assert_eq!(actual.channels(), ChannelCount::new(2).unwrap());
    assert_eq!(actual.as_planar_f32(), &[0.25, -0.5, 0.0, 0.75, 0.0, -0.25]);
}

#[test]
fn merge_audio_buffers_rejects_mismatched_sample_rate() {
    let first = audio_buffer(vec![0.25]);
    let second = audio_buffer_with_spec(vec![-0.25], 44_100, 1, SampleFormat::Float32);

    let error = merge_audio_buffers(&[first, second]).unwrap_err();

    assert_eq!(
        error,
        InputCombineError::MismatchedSampleRate {
            input_index: 1,
            expected: SampleRate::new(48_000).unwrap(),
            actual: SampleRate::new(44_100).unwrap(),
        }
    );
}

#[test]
fn merge_audio_buffers_rejects_mismatched_sample_format() {
    let first = audio_buffer(vec![0.25]);
    let second = audio_buffer_with_spec(vec![-0.25], 48_000, 1, SampleFormat::Pcm16);

    let error = merge_audio_buffers(&[first, second]).unwrap_err();

    assert_eq!(
        error,
        InputCombineError::MismatchedSampleFormat {
            input_index: 1,
            expected: SampleFormat::Float32,
            actual: SampleFormat::Pcm16,
        }
    );
}

#[test]
fn merged_audio_enters_effect_pipeline_before_effects() {
    let first = audio_buffer(vec![0.25, -0.5]);
    let second = audio_buffer(vec![0.75, 0.0, -0.25]);

    let actual = AudioFile::from_audio_buffers_merged(&[first, second])
        .unwrap()
        .into_pipeline()
        .gain_db(6.0)
        .reverse()
        .into_audio_buffer()
        .unwrap();

    let multiplier = 10.0_f32.powf(6.0 / 20.0);
    assert_samples_close(
        actual.as_planar_f32(),
        &[
            0.0,
            -0.5 * multiplier,
            0.25 * multiplier,
            -0.25 * multiplier,
            0.0,
            0.75 * multiplier,
        ],
    );
}

#[test]
fn open_wavs_merged_round_trips_through_file_boundary() {
    let tempdir = temp_dir();
    fs::create_dir(&tempdir).unwrap();
    let first = tempdir.join("first.wav");
    let second = tempdir.join("second.wav");
    let output = tempdir.join("output.wav");

    auralis_wav::encode_pcm16_path(&first, &audio_buffer(vec![0.25, -0.5])).unwrap();
    auralis_wav::encode_pcm16_path(&second, &audio_buffer(vec![0.75, 0.0, -0.25])).unwrap();

    AudioFile::open_wavs_merged([&first, &second])
        .unwrap()
        .into_pipeline()
        .write_wav(&output)
        .unwrap();

    let decoded = auralis_wav::decode_pcm16_path(output).unwrap();
    assert_eq!(decoded.frames(), FrameCount::new(3));
    assert_eq!(decoded.channels(), ChannelCount::new(2).unwrap());
    assert_samples_close(
        decoded.as_planar_f32(),
        &[0.25, -0.5, 0.0, 0.75, 0.0, -0.25],
    );
    fs::remove_dir_all(tempdir).unwrap();
}

#[test]
fn multiply_audio_buffers_multiplies_equal_length_mono_inputs() {
    let first = audio_buffer(vec![0.5, -0.5, 1.0]);
    let second = audio_buffer(vec![0.25, 1.0, -0.5]);

    let actual = multiply_audio_buffers(&[first, second]).unwrap();

    assert_eq!(actual.frames(), FrameCount::new(3));
    assert_eq!(actual.channels(), ChannelCount::new(1).unwrap());
    assert_sample_bits_eq(actual.as_planar_f32(), &[0.125, -0.5, -0.5]);
}

#[test]
fn multiply_audio_buffers_treats_mismatched_lengths_as_silence() {
    let first = audio_buffer(vec![0.5, -0.5]);
    let second = audio_buffer(vec![0.25, 1.0, -0.5]);

    let actual = multiply_audio_buffers(&[first, second]).unwrap();

    assert_eq!(actual.frames(), FrameCount::new(3));
    assert_sample_bits_eq(actual.as_planar_f32(), &[0.125, -0.5, 0.0]);
}

#[test]
fn multiply_audio_buffers_preserves_stereo_channel_grouping() {
    let first = stereo_audio_buffer(vec![0.5, -0.5, 1.0, -1.0]);
    let second = stereo_audio_buffer(vec![0.25, 1.0, -0.5, -0.5]);

    let actual = multiply_audio_buffers(&[first, second]).unwrap();

    assert_eq!(actual.frames(), FrameCount::new(2));
    assert_eq!(actual.channels(), ChannelCount::new(2).unwrap());
    assert_sample_bits_eq(actual.as_planar_f32(), &[0.125, -0.5, -0.5, 0.5]);
}

#[test]
fn multiply_audio_buffers_accepts_mismatched_channel_counts_with_silence() {
    let mono = audio_buffer(vec![0.5, -0.5]);
    let stereo = stereo_audio_buffer(vec![0.25, 1.0, -0.5, -0.5]);

    let actual = multiply_audio_buffers(&[mono, stereo]).unwrap();

    assert_eq!(actual.frames(), FrameCount::new(2));
    assert_eq!(actual.channels(), ChannelCount::new(2).unwrap());
    assert_sample_bits_eq(actual.as_planar_f32(), &[0.125, -0.5, 0.0, 0.0]);
}

#[test]
fn multiply_audio_buffers_single_input_is_identity() {
    let first = audio_buffer(vec![0.5, -0.5, 1.0]);

    let actual = multiply_audio_buffers(&[first]).unwrap();

    assert_sample_bits_eq(actual.as_planar_f32(), &[0.5, -0.5, 1.0]);
}

#[test]
fn multiply_audio_buffers_zero_sample_zeros_product() {
    let first = audio_buffer(vec![0.5, -0.5, 1.0]);
    let second = audio_buffer(vec![1.0, 0.0, 1.0]);

    let actual = multiply_audio_buffers(&[first, second]).unwrap();

    assert_sample_bits_eq(actual.as_planar_f32(), &[0.5, -0.0, 1.0]);
}

#[test]
fn multiply_audio_buffers_rejects_mismatched_sample_rate() {
    let first = audio_buffer(vec![0.25]);
    let second = audio_buffer_with_spec(vec![-0.25], 44_100, 1, SampleFormat::Float32);

    let error = multiply_audio_buffers(&[first, second]).unwrap_err();

    assert_eq!(
        error,
        InputCombineError::MismatchedSampleRate {
            input_index: 1,
            expected: SampleRate::new(48_000).unwrap(),
            actual: SampleRate::new(44_100).unwrap(),
        }
    );
}

#[test]
fn multiply_audio_buffers_rejects_mismatched_sample_format() {
    let first = audio_buffer(vec![0.25]);
    let second = audio_buffer_with_spec(vec![-0.25], 48_000, 1, SampleFormat::Pcm16);

    let error = multiply_audio_buffers(&[first, second]).unwrap_err();

    assert_eq!(
        error,
        InputCombineError::MismatchedSampleFormat {
            input_index: 1,
            expected: SampleFormat::Float32,
            actual: SampleFormat::Pcm16,
        }
    );
}

#[test]
fn multiply_audio_buffers_matches_under_forced_scalar_and_requested_simd() {
    let first = audio_buffer(vec![
        -1.0,
        -0.999_984_74,
        -0.5,
        -0.0,
        0.0,
        0.5,
        0.999_984_74,
        1.0,
    ]);
    let second = audio_buffer(vec![1.0, 0.999_984_74, 0.5, 0.0, -0.0]);

    let scalar =
        multiply_audio_buffers_with_backend(&[first.clone(), second.clone()], BackendKind::Scalar)
            .unwrap();
    let simd = multiply_audio_buffers_with_backend(&[first, second], BackendKind::Simd).unwrap();

    assert_sample_bits_eq(simd.as_planar_f32(), scalar.as_planar_f32());
}

#[test]
fn multiplied_audio_enters_effect_pipeline_before_effects() {
    let first = audio_buffer(vec![0.25, -0.5]);
    let second = audio_buffer(vec![0.75, 1.0, -0.25]);

    let actual = AudioFile::from_audio_buffers_multiplied(&[first, second])
        .unwrap()
        .into_pipeline()
        .gain_db(6.0)
        .reverse()
        .into_audio_buffer()
        .unwrap();

    let multiplier = 10.0_f32.powf(6.0 / 20.0);
    assert_samples_close(
        actual.as_planar_f32(),
        &[0.0, (-0.5 * 1.0) * multiplier, (0.25 * 0.75) * multiplier],
    );
}

#[test]
fn open_wavs_multiplied_round_trips_through_file_boundary() {
    let tempdir = temp_dir();
    fs::create_dir(&tempdir).unwrap();
    let first = tempdir.join("first.wav");
    let second = tempdir.join("second.wav");
    let output = tempdir.join("output.wav");

    auralis_wav::encode_pcm16_path(&first, &audio_buffer(vec![0.25, -0.5])).unwrap();
    auralis_wav::encode_pcm16_path(&second, &audio_buffer(vec![0.75, 1.0, -0.25])).unwrap();

    AudioFile::open_wavs_multiplied([&first, &second])
        .unwrap()
        .into_pipeline()
        .write_wav(&output)
        .unwrap();

    let decoded = auralis_wav::decode_pcm16_path(output).unwrap();
    assert_samples_close(decoded.as_planar_f32(), &[0.1875, -0.5, 0.0]);
    fs::remove_dir_all(tempdir).unwrap();
}

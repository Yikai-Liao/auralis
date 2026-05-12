//! WAV encode, round-trip, backend, and writer-boundary tests.

use std::{fs, io::Cursor};

use auralis_codec::{
    AudioEncoder, AudioWriter, CodecError, CodecKind, WavEncodeOptions, WavSampleFormat,
};
use auralis_simd::BackendKind;
use auralis_wav::{
    Pcm8WavEncoder, Pcm8WavWriter, Pcm16WavEncoder, Pcm16WavWriter, Pcm24WavEncoder,
    Pcm24WavWriter, WavError, decode_pcm16_path, encode_pcm16_path,
};

mod support;

use support::{
    assert_audio_bits_eq, audio_buffer, decode_path, decode_path_pcm8, decode_path_pcm24,
    encode_temp_wav, encode_temp_wav_pcm8, encode_temp_wav_pcm24, encode_temp_wav_with_backend,
    temp_path,
};

#[test]
fn encodes_mono_pcm8_from_planar_f32() {
    let audio = audio_buffer(1, 4, &[-1.0, 0.0, 0.5, 1.0]);
    let path = encode_temp_wav_pcm8("auralis-wav-encode-mono-pcm8", &audio);
    let decoded = decode_path_pcm8(&path);

    fs::remove_file(path).unwrap();
    assert_eq!(decoded.spec().sample_rate().as_u32(), 48_000);
    assert_eq!(decoded.spec().channels().as_u16(), 1);
    assert_eq!(decoded.frames().as_u64(), 4);
    assert_eq!(
        decoded.channel(0).unwrap(),
        &[-1.0, 0.0, 0.5, 127.0 / 128.0]
    );
}

#[test]
fn encodes_mono_pcm16_from_planar_f32() {
    let audio = audio_buffer(1, 4, &[-1.0, 0.0, 0.5, f32::from(32_767_i16) / 32768.0]);
    let path = encode_temp_wav("auralis-wav-encode-mono", &audio);
    let decoded = decode_path(&path);

    fs::remove_file(path).unwrap();
    assert_eq!(decoded.spec().sample_rate().as_u32(), 48_000);
    assert_eq!(decoded.spec().channels().as_u16(), 1);
    assert_eq!(decoded.frames().as_u64(), 4);
    assert_eq!(
        decoded.channel(0).unwrap(),
        &[-1.0, 0.0, 0.5, f32::from(32_767_i16) / 32768.0]
    );
}

#[test]
fn encodes_mono_pcm24_from_planar_f32() {
    let audio = audio_buffer(1, 4, &[-1.0, 0.0, 0.5, 8_388_607.0 / 8_388_608.0]);
    let path = encode_temp_wav_pcm24("auralis-wav-encode-mono-pcm24", &audio);
    let decoded = decode_path_pcm24(&path);

    fs::remove_file(path).unwrap();
    assert_eq!(decoded.spec().sample_rate().as_u32(), 48_000);
    assert_eq!(decoded.spec().channels().as_u16(), 1);
    assert_eq!(decoded.frames().as_u64(), 4);
    assert_eq!(
        decoded.channel(0).unwrap(),
        &[-1.0, 0.0, 0.5, 8_388_607.0 / 8_388_608.0]
    );
}

#[test]
fn encodes_stereo_pcm16_by_interleaving_frames() {
    let audio = audio_buffer(2, 3, &[-1.0, -0.5, 0.0, 0.999_969_5, 0.5, 0.25]);
    let path = encode_temp_wav("auralis-wav-encode-stereo", &audio);
    let decoded = decode_path(&path);

    fs::remove_file(path).unwrap();
    assert_eq!(decoded.spec().channels().as_u16(), 2);
    assert_eq!(decoded.frames().as_u64(), 3);
    assert_eq!(decoded.channel(0).unwrap(), &[-1.0, -0.5, 0.0]);
    assert_eq!(
        decoded.channel(1).unwrap(),
        &[f32::from(32_767_i16) / 32768.0, 0.5, 0.25]
    );
}

#[test]
fn encode_clips_samples_to_pcm16_range() {
    let audio = audio_buffer(1, 4, &[-2.0, -1.0, 1.0, 2.0]);
    let path = encode_temp_wav("auralis-wav-encode-clip", &audio);
    let decoded = decode_path(&path);

    fs::remove_file(path).unwrap();
    assert_eq!(
        decoded.channel(0).unwrap(),
        &[
            -1.0,
            -1.0,
            f32::from(32_767_i16) / 32768.0,
            f32::from(32_767_i16) / 32768.0,
        ]
    );
}

#[test]
fn encode_pcm16_matches_under_forced_scalar_and_requested_simd() {
    let audio = audio_buffer(
        2,
        5,
        &[
            -1.5,
            -1.0,
            -0.5,
            -0.5 / 32768.0,
            0.0,
            0.5 / 32768.0,
            0.25,
            0.999_984_74,
            1.0,
            1.5,
        ],
    );
    let scalar_path =
        encode_temp_wav_with_backend("auralis-wav-encode-scalar", &audio, BackendKind::Scalar);
    let simd_path =
        encode_temp_wav_with_backend("auralis-wav-encode-simd", &audio, BackendKind::Simd);
    let scalar = decode_pcm16_path(&scalar_path).unwrap();
    let simd = decode_pcm16_path(&simd_path).unwrap();

    fs::remove_file(scalar_path).unwrap();
    fs::remove_file(simd_path).unwrap();
    assert_audio_bits_eq(&simd, &scalar);
}

#[test]
fn encode_rejects_non_finite_samples() {
    let audio = audio_buffer(2, 2, &[0.0, 0.25, 0.5, f32::NAN]);
    let path = temp_path("auralis-wav-encode-nan", "wav");
    let error = encode_pcm16_path(&path, &audio).unwrap_err();

    let _ = fs::remove_file(path);
    assert_eq!(
        error,
        WavError::NonFiniteSample {
            channel_index: 1,
            frame_index: 1,
        }
    );
}

#[test]
fn round_trips_generated_pcm16_signal() {
    let input = audio_buffer(2, 4, &[-0.75, -0.25, 0.25, 0.75, 0.0, 0.125, -0.125, 0.5]);
    let first_path = encode_temp_wav("auralis-wav-roundtrip-first", &input);
    let decoded = decode_path(&first_path);
    let second_path = encode_temp_wav("auralis-wav-roundtrip-second", &decoded);
    let round_tripped = decode_path(&second_path);

    fs::remove_file(first_path).unwrap();
    fs::remove_file(second_path).unwrap();
    assert_eq!(round_tripped, decoded);
}

#[test]
fn implements_codec_writer_boundary() {
    let audio = audio_buffer(1, 1, &[0.0]);
    let mut writer = Pcm16WavWriter::new(Cursor::new(Vec::new()));

    writer.write_audio(&audio).unwrap();
    assert_eq!(writer.codec_kind(), CodecKind::Wav);
    assert!(matches!(
        writer.write_audio(&audio),
        Err(CodecError::EncodeFailed {
            kind: CodecKind::Wav,
            ..
        })
    ));
}

#[test]
fn implements_codec_encoder_boundary() {
    let audio = audio_buffer(1, 2, &[0.0, 0.25]);
    let encoder = Pcm16WavEncoder::new(WavEncodeOptions::default(), BackendKind::Scalar);
    let mut output = Cursor::new(Vec::new());

    let summary = encoder.encode(&audio, &mut output).unwrap();
    let decoded = auralis_wav::decode_pcm16(Cursor::new(output.into_inner())).unwrap();

    assert_eq!(summary.codec_kind(), CodecKind::Wav);
    assert_eq!(summary.spec(), audio.spec());
    assert_eq!(summary.frames(), audio.frames());
    assert_eq!(decoded, audio);
}

#[test]
fn implements_pcm8_codec_encoder_boundary() {
    let audio = audio_buffer(1, 3, &[-1.0, 0.0, 1.0]);
    let encoder = Pcm8WavEncoder::new(WavEncodeOptions::pcm8(), BackendKind::Scalar);
    let mut output = Cursor::new(Vec::new());

    let summary = encoder.encode(&audio, &mut output).unwrap();
    let decoded = auralis_wav::decode_pcm8(Cursor::new(output.into_inner())).unwrap();

    assert_eq!(summary.codec_kind(), CodecKind::Wav);
    assert_eq!(summary.spec(), audio.spec());
    assert_eq!(summary.frames(), audio.frames());
    assert_eq!(decoded.channel(0).unwrap(), &[-1.0, 0.0, 127.0 / 128.0]);
}

#[test]
fn implements_pcm24_codec_encoder_boundary() {
    let audio = audio_buffer(1, 3, &[-1.0, 0.0, 8_388_607.0 / 8_388_608.0]);
    let encoder = Pcm24WavEncoder::new(WavEncodeOptions::pcm24(), BackendKind::Scalar);
    let mut output = Cursor::new(Vec::new());

    let summary = encoder.encode(&audio, &mut output).unwrap();
    let decoded = auralis_wav::decode_pcm24(Cursor::new(output.into_inner())).unwrap();

    assert_eq!(summary.codec_kind(), CodecKind::Wav);
    assert_eq!(summary.spec(), audio.spec());
    assert_eq!(summary.frames(), audio.frames());
    assert_eq!(
        decoded.channel(0).unwrap(),
        &[-1.0, 0.0, 8_388_607.0 / 8_388_608.0]
    );
}

#[test]
fn pcm8_writer_boundary_is_one_shot() {
    let audio = audio_buffer(1, 1, &[0.0]);
    let mut writer = Pcm8WavWriter::new(Cursor::new(Vec::new()));

    writer.write_audio(&audio).unwrap();
    assert_eq!(writer.codec_kind(), CodecKind::Wav);
    assert!(matches!(
        writer.write_audio(&audio),
        Err(CodecError::EncodeFailed {
            kind: CodecKind::Wav,
            ..
        })
    ));
}

#[test]
fn pcm24_writer_boundary_is_one_shot() {
    let audio = audio_buffer(1, 1, &[0.0]);
    let mut writer = Pcm24WavWriter::new(Cursor::new(Vec::new()));

    writer.write_audio(&audio).unwrap();
    assert_eq!(writer.codec_kind(), CodecKind::Wav);
    assert!(matches!(
        writer.write_audio(&audio),
        Err(CodecError::EncodeFailed {
            kind: CodecKind::Wav,
            ..
        })
    ));
}

#[test]
fn wav_encode_options_default_to_pcm16() {
    assert_eq!(
        WavEncodeOptions::default().sample_format(),
        WavSampleFormat::Pcm16
    );
}

#[test]
fn wav_encode_options_can_target_pcm24() {
    assert_eq!(
        WavEncodeOptions::pcm24().sample_format(),
        WavSampleFormat::Pcm24
    );
}

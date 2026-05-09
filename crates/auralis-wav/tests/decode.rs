//! PCM16 WAV decode, format validation, and reader-boundary tests.

use std::{fs, io::Cursor};

use auralis_codec::{AudioReader, CodecKind};
use auralis_simd::BackendKind;
use auralis_wav::{
    Pcm16WavReader, WavError, WavSampleEncoding, decode_pcm16, decode_pcm16_path,
    decode_pcm16_with_backend,
};

mod support;

use support::{assert_audio_bits_eq, riff_header, wav_bytes, wav_bytes_with_bits, write_temp_wav};

#[test]
fn decodes_mono_pcm16_to_planar_f32() {
    let audio = decode_pcm16(Cursor::new(wav_bytes(1, &[-32768, 0, 16_384, 32_767]))).unwrap();

    assert_eq!(audio.spec().sample_rate().as_u32(), 48_000);
    assert_eq!(audio.spec().channels().as_u16(), 1);
    assert_eq!(audio.frames().as_u64(), 4);
    assert_eq!(
        audio.channel(0).unwrap(),
        &[-1.0, 0.0, 0.5, f32::from(32_767_i16) / 32768.0]
    );
}

#[test]
fn decodes_stereo_pcm16_to_channel_major_storage() {
    let audio = decode_pcm16(Cursor::new(wav_bytes(
        2,
        &[-32768, 32_767, -16_384, 16_384, 0, 8192],
    )))
    .unwrap();

    assert_eq!(audio.spec().channels().as_u16(), 2);
    assert_eq!(audio.frames().as_u64(), 3);
    assert_eq!(audio.channel(0).unwrap(), &[-1.0, -0.5, 0.0]);
    assert_eq!(
        audio.channel(1).unwrap(),
        &[f32::from(32_767_i16) / 32768.0, 0.5, 0.25]
    );
}

#[test]
fn decode_pcm16_matches_under_forced_scalar_and_requested_simd() {
    let bytes = wav_bytes(
        2,
        &[
            i16::MIN,
            i16::MAX,
            -16_384,
            16_384,
            -1,
            1,
            0,
            8192,
            -8192,
            1234,
        ],
    );

    let scalar =
        decode_pcm16_with_backend(Cursor::new(bytes.clone()), BackendKind::Scalar).unwrap();
    let simd = decode_pcm16_with_backend(Cursor::new(bytes), BackendKind::Simd).unwrap();

    assert_audio_bits_eq(&simd, &scalar);
}

#[test]
fn rejects_unsupported_bit_depth_with_typed_error() {
    let error = decode_pcm16(Cursor::new(wav_bytes_with_bits(1, 24, &[0, 0, 0]))).unwrap_err();

    assert_eq!(
        error,
        WavError::UnsupportedSampleFormat {
            bits_per_sample: 24,
            encoding: WavSampleEncoding::Integer,
        }
    );
}

#[test]
fn rejects_float_wav_with_typed_error() {
    let mut bytes = riff_header(1, 32, 3, 4);
    bytes.extend_from_slice(&0.0_f32.to_le_bytes());
    let error = decode_pcm16(Cursor::new(bytes)).unwrap_err();

    assert_eq!(
        error,
        WavError::UnsupportedSampleFormat {
            bits_per_sample: 32,
            encoding: WavSampleEncoding::Float,
        }
    );
}

#[test]
fn malformed_wav_returns_typed_error() {
    let error = decode_pcm16(Cursor::new(b"not a wav".to_vec())).unwrap_err();

    assert!(matches!(error, WavError::Malformed { .. }));
}

#[test]
fn path_decoder_preserves_sample_rate_and_channels() {
    let path = write_temp_wav("auralis-wav-path", 2, &[-1024, 1024]).unwrap();
    let audio = decode_pcm16_path(&path).unwrap();

    fs::remove_file(path).unwrap();
    assert_eq!(audio.spec().sample_rate().as_u32(), 48_000);
    assert_eq!(audio.spec().channels().as_u16(), 2);
}

#[test]
fn implements_codec_reader_boundary() {
    let mut reader = Pcm16WavReader::new(Cursor::new(wav_bytes(1, &[0]))).unwrap();
    let audio = reader.read_audio().unwrap();

    assert_eq!(reader.codec_kind(), CodecKind::Wav);
    assert_eq!(audio.sample(0, 0), Some(0.0));
}

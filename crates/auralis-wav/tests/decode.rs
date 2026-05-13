//! WAV decode, format validation, and reader-boundary tests.

use std::{fs, io::Cursor};

use auralis_codec::{AudioReader, CodecKind};
use auralis_simd::BackendKind;
use auralis_wav::{
    AnyPcmWavReader, Pcm16WavReader, WavError, WavSampleEncoding, decode_alaw, decode_float32,
    decode_float64, decode_pcm8, decode_pcm16, decode_pcm16_path, decode_pcm16_with_backend,
    decode_pcm24, decode_pcm24_path, decode_pcm32, decode_pcm32_path, decode_ulaw, decode_wav,
};

mod support;

use support::{
    assert_audio_bits_eq, wav_bytes, wav_bytes_alaw, wav_bytes_float32, wav_bytes_float64,
    wav_bytes_pcm8, wav_bytes_pcm24, wav_bytes_pcm32, wav_bytes_ulaw, wav_bytes_with_bits,
    write_temp_wav,
};

#[test]
fn decodes_mono_pcm8_to_planar_f32() {
    let audio = decode_pcm8(Cursor::new(wav_bytes_pcm8(1, &[-128, 0, 64, 127]))).unwrap();

    assert_eq!(audio.spec().sample_rate().as_u32(), 48_000);
    assert_eq!(audio.spec().channels().as_u16(), 1);
    assert_eq!(audio.frames().as_u64(), 4);
    assert_eq!(audio.channel(0).unwrap(), &[-1.0, 0.0, 0.5, 127.0 / 128.0]);
}

#[test]
fn generic_decoder_accepts_pcm8_and_pcm16() {
    let pcm8 = decode_wav(Cursor::new(wav_bytes_pcm8(1, &[-128, 127]))).unwrap();
    let pcm16 = decode_wav(Cursor::new(wav_bytes(1, &[i16::MIN, i16::MAX]))).unwrap();
    let pcm24 = decode_wav(Cursor::new(wav_bytes_pcm24(1, &[-8_388_608, 8_388_607]))).unwrap();
    let pcm32 = decode_wav(Cursor::new(wav_bytes_pcm32(1, &[i32::MIN, i32::MAX]))).unwrap();
    let float32 = decode_wav(Cursor::new(wav_bytes_float32(1, &[-1.25, 0.25]))).unwrap();
    let float64 = decode_wav(Cursor::new(wav_bytes_float64(1, &[-1.5, 0.75]))).unwrap();
    let ulaw = decode_wav(Cursor::new(wav_bytes_ulaw(1, &[0xff, 0x7f]))).unwrap();
    let alaw = decode_wav(Cursor::new(wav_bytes_alaw(1, &[0xd5, 0x55]))).unwrap();

    assert_eq!(pcm8.channel(0).unwrap(), &[-1.0, 127.0 / 128.0]);
    assert_eq!(
        pcm16.channel(0).unwrap(),
        &[-1.0, f32::from(i16::MAX) / 32768.0]
    );
    assert_eq!(
        pcm24.channel(0).unwrap(),
        &[-1.0, 8_388_607.0 / 8_388_608.0]
    );
    assert_eq!(pcm32.channel(0).unwrap(), &[-1.0, 1.0]);
    assert_eq!(float32.channel(0).unwrap(), &[-1.25, 0.25]);
    assert_eq!(float64.channel(0).unwrap(), &[-1.5, 0.75]);
    assert_eq!(ulaw.channel(0).unwrap(), &[0.0, 0.0]);
    assert_eq!(alaw.channel(0).unwrap(), &[8.0 / 32768.0, -8.0 / 32768.0]);
}

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
fn decodes_mono_pcm24_to_planar_f32() {
    let audio = decode_pcm24(Cursor::new(wav_bytes_pcm24(
        1,
        &[-8_388_608, 0, 4_194_304, 8_388_607],
    )))
    .unwrap();

    assert_eq!(audio.spec().sample_rate().as_u32(), 48_000);
    assert_eq!(audio.spec().channels().as_u16(), 1);
    assert_eq!(audio.frames().as_u64(), 4);
    assert_eq!(
        audio.channel(0).unwrap(),
        &[-1.0, 0.0, 0.5, 8_388_607.0 / 8_388_608.0]
    );
}

#[test]
fn decodes_mono_pcm32_to_planar_f32() {
    let audio = decode_pcm32(Cursor::new(wav_bytes_pcm32(
        1,
        &[i32::MIN, 0, 1_073_741_824, i32::MAX],
    )))
    .unwrap();

    assert_eq!(audio.spec().sample_rate().as_u32(), 48_000);
    assert_eq!(audio.spec().channels().as_u16(), 1);
    assert_eq!(audio.frames().as_u64(), 4);
    assert_eq!(audio.channel(0).unwrap(), &[-1.0, 0.0, 0.5, 1.0]);
}

#[test]
fn decodes_mono_float32_to_planar_f32() {
    let audio =
        decode_float32(Cursor::new(wav_bytes_float32(1, &[-1.25, 0.0, 0.5, 1.25]))).unwrap();

    assert_eq!(audio.spec().sample_rate().as_u32(), 48_000);
    assert_eq!(audio.spec().channels().as_u16(), 1);
    assert_eq!(audio.frames().as_u64(), 4);
    assert_eq!(audio.channel(0).unwrap(), &[-1.25, 0.0, 0.5, 1.25]);
}

#[test]
fn decodes_mono_float64_to_planar_f32() {
    let audio = decode_float64(Cursor::new(wav_bytes_float64(1, &[-1.5, 0.0, 0.5, 1.5]))).unwrap();

    assert_eq!(audio.spec().sample_rate().as_u32(), 48_000);
    assert_eq!(audio.spec().channels().as_u16(), 1);
    assert_eq!(audio.frames().as_u64(), 4);
    assert_eq!(audio.channel(0).unwrap(), &[-1.5, 0.0, 0.5, 1.5]);
}

#[test]
fn decodes_mono_ulaw_to_planar_f32() {
    let audio = decode_ulaw(Cursor::new(wav_bytes_ulaw(1, &[0xff, 0xbf, 0x7f]))).unwrap();

    assert_eq!(audio.spec().sample_rate().as_u32(), 48_000);
    assert_eq!(audio.spec().channels().as_u16(), 1);
    assert_eq!(audio.frames().as_u64(), 3);
    assert_eq!(audio.channel(0).unwrap(), &[0.0, 1980.0 / 32768.0, 0.0]);
}

#[test]
fn decodes_mono_alaw_to_planar_f32() {
    let audio = decode_alaw(Cursor::new(wav_bytes_alaw(1, &[0xd5, 0x55, 0xaa]))).unwrap();

    assert_eq!(audio.spec().sample_rate().as_u32(), 48_000);
    assert_eq!(audio.spec().channels().as_u16(), 1);
    assert_eq!(audio.frames().as_u64(), 3);
    assert_eq!(
        audio.channel(0).unwrap(),
        &[8.0 / 32768.0, -8.0 / 32768.0, 32256.0 / 32768.0]
    );
}

#[test]
fn decode_float32_rejects_non_finite_samples() {
    let error = decode_float32(Cursor::new(wav_bytes_float32(2, &[0.0, f32::NAN]))).unwrap_err();

    assert_eq!(
        error,
        WavError::NonFiniteSample {
            channel_index: 1,
            frame_index: 0,
        }
    );
}

#[test]
fn decode_float64_rejects_non_finite_samples() {
    let error =
        decode_float64(Cursor::new(wav_bytes_float64(2, &[0.0, f64::INFINITY]))).unwrap_err();

    assert_eq!(
        error,
        WavError::NonFiniteSample {
            channel_index: 1,
            frame_index: 0,
        }
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
fn pcm16_specific_decoder_rejects_pcm32_with_typed_error() {
    let error = decode_pcm16(Cursor::new(wav_bytes_pcm32(1, &[0]))).unwrap_err();

    assert_eq!(
        error,
        WavError::UnsupportedSampleFormat {
            bits_per_sample: 32,
            encoding: WavSampleEncoding::Integer,
        }
    );
}

#[test]
fn pcm16_specific_decoder_rejects_float32_with_typed_error() {
    let error = decode_pcm16(Cursor::new(wav_bytes_float32(1, &[0.0]))).unwrap_err();

    assert_eq!(
        error,
        WavError::UnsupportedSampleFormat {
            bits_per_sample: 32,
            encoding: WavSampleEncoding::Float,
        }
    );
}

#[test]
fn pcm16_specific_decoder_rejects_float64_with_typed_error() {
    let error = decode_pcm16(Cursor::new(wav_bytes_float64(1, &[0.0]))).unwrap_err();

    assert_eq!(
        error,
        WavError::UnsupportedSampleFormat {
            bits_per_sample: 64,
            encoding: WavSampleEncoding::Float,
        }
    );
}

#[test]
fn pcm16_specific_decoder_rejects_companded_with_typed_error() {
    let error = decode_pcm16(Cursor::new(wav_bytes_ulaw(1, &[0xff]))).unwrap_err();

    assert_eq!(
        error,
        WavError::UnsupportedSampleFormat {
            bits_per_sample: 8,
            encoding: WavSampleEncoding::Companded,
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
fn path_decoder_accepts_pcm24() {
    let path = support::temp_path("auralis-wav-path-pcm24", "wav");
    fs::write(&path, wav_bytes_pcm24(2, &[-8_388_608, 8_388_607])).unwrap();
    let audio = decode_pcm24_path(&path).unwrap();

    fs::remove_file(path).unwrap();
    assert_eq!(audio.spec().sample_rate().as_u32(), 48_000);
    assert_eq!(audio.spec().channels().as_u16(), 2);
    assert_eq!(audio.channel(0).unwrap(), &[-1.0]);
    assert_eq!(audio.channel(1).unwrap(), &[8_388_607.0 / 8_388_608.0]);
}

#[test]
fn path_decoder_accepts_pcm32() {
    let path = support::temp_path("auralis-wav-path-pcm32", "wav");
    fs::write(&path, wav_bytes_pcm32(2, &[i32::MIN, i32::MAX])).unwrap();
    let audio = decode_pcm32_path(&path).unwrap();

    fs::remove_file(path).unwrap();
    assert_eq!(audio.spec().sample_rate().as_u32(), 48_000);
    assert_eq!(audio.spec().channels().as_u16(), 2);
    assert_eq!(audio.channel(0).unwrap(), &[-1.0]);
    assert_eq!(audio.channel(1).unwrap(), &[1.0]);
}

#[test]
fn path_decoder_accepts_float32() {
    let path = support::temp_path("auralis-wav-path-float32", "wav");
    fs::write(&path, wav_bytes_float32(2, &[-1.25, 1.25])).unwrap();
    let audio = auralis_wav::decode_float32_path(&path).unwrap();

    fs::remove_file(path).unwrap();
    assert_eq!(audio.spec().sample_rate().as_u32(), 48_000);
    assert_eq!(audio.spec().channels().as_u16(), 2);
    assert_eq!(audio.channel(0).unwrap(), &[-1.25]);
    assert_eq!(audio.channel(1).unwrap(), &[1.25]);
}

#[test]
fn path_decoder_accepts_float64() {
    let path = support::temp_path("auralis-wav-path-float64", "wav");
    fs::write(&path, wav_bytes_float64(2, &[-1.5, 1.5])).unwrap();
    let audio = auralis_wav::decode_float64_path(&path).unwrap();

    fs::remove_file(path).unwrap();
    assert_eq!(audio.spec().sample_rate().as_u32(), 48_000);
    assert_eq!(audio.spec().channels().as_u16(), 2);
    assert_eq!(audio.channel(0).unwrap(), &[-1.5]);
    assert_eq!(audio.channel(1).unwrap(), &[1.5]);
}

#[test]
fn path_decoder_accepts_g711() {
    let ulaw_path = support::temp_path("auralis-wav-path-ulaw", "wav");
    let alaw_path = support::temp_path("auralis-wav-path-alaw", "wav");
    fs::write(&ulaw_path, wav_bytes_ulaw(2, &[0xff, 0xbf])).unwrap();
    fs::write(&alaw_path, wav_bytes_alaw(2, &[0xd5, 0x55])).unwrap();

    let ulaw = auralis_wav::decode_ulaw_path(&ulaw_path).unwrap();
    let alaw = auralis_wav::decode_alaw_path(&alaw_path).unwrap();

    fs::remove_file(ulaw_path).unwrap();
    fs::remove_file(alaw_path).unwrap();
    assert_eq!(ulaw.channel(0).unwrap(), &[0.0]);
    assert_eq!(ulaw.channel(1).unwrap(), &[1980.0 / 32768.0]);
    assert_eq!(alaw.channel(0).unwrap(), &[8.0 / 32768.0]);
    assert_eq!(alaw.channel(1).unwrap(), &[-8.0 / 32768.0]);
}

#[test]
fn implements_codec_reader_boundary() {
    let mut reader = Pcm16WavReader::new(Cursor::new(wav_bytes(1, &[0]))).unwrap();
    let audio = reader.read_audio().unwrap();

    assert_eq!(reader.codec_kind(), CodecKind::Wav);
    assert_eq!(audio.sample(0, 0), Some(0.0));
}

#[test]
fn generic_reader_boundary_decodes_pcm8() {
    let mut reader = AnyPcmWavReader::new(Cursor::new(wav_bytes_pcm8(1, &[0, 127]))).unwrap();
    let audio = reader.read_audio().unwrap();

    assert_eq!(reader.codec_kind(), CodecKind::Wav);
    assert_eq!(audio.channel(0).unwrap(), &[0.0, 127.0 / 128.0]);
}

#![allow(missing_docs)]

use std::{io::Cursor, path::PathBuf};

use auralis_codec::{AudioEncoder, CodecKind, FlacEncodeOptions};
use auralis_core::{AudioBuffer, AudioSpec, ChannelCount, FrameCount, SampleFormat, SampleRate};
use auralis_flac::{FlacEncoder, FlacError, decode_flac, decode_flac_path, encode_flac};

const POP_FLAC: &[u8] = &[
    0x66, 0x4c, 0x61, 0x43, 0x80, 0x00, 0x00, 0x22, 0x10, 0x00, 0x10, 0x00, 0x00, 0x00, 0x3f, 0x00,
    0x00, 0x3f, 0x0a, 0xc4, 0x40, 0xf0, 0x00, 0x00, 0x00, 0x64, 0x68, 0x46, 0x42, 0x88, 0xfa, 0x5e,
    0x19, 0x83, 0x55, 0x16, 0x97, 0x2d, 0xcf, 0x47, 0x22, 0x3c, 0xff, 0xf8, 0x69, 0x08, 0x00, 0x63,
    0x33, 0x18, 0x00, 0x00, 0x08, 0x04, 0x10, 0x01, 0x17, 0xee, 0x00, 0xa3, 0x2e, 0x4e, 0x8a, 0x65,
    0xda, 0x6a, 0x95, 0x2e, 0x62, 0x89, 0xe9, 0x2f, 0x32, 0xa4, 0xd7, 0x27, 0x49, 0xcc, 0x56, 0xde,
    0x37, 0xed, 0x66, 0x73, 0x14, 0x8e, 0xcb, 0xae, 0xf2, 0x59, 0x14, 0x87, 0x31, 0x53, 0xa3, 0xb2,
    0xcd, 0xe4, 0x8a, 0x47, 0xdb, 0xcd, 0x60, 0xb5, 0x75,
];

#[test]
fn decode_flac_reads_embedded_integer_stream() {
    let audio = decode_flac(Cursor::new(POP_FLAC)).unwrap();

    assert_eq!(audio.spec().sample_rate().as_u32(), 44_100);
    assert_eq!(audio.channels().as_u16(), 1);
    assert_eq!(audio.spec().sample_format(), SampleFormat::Float32);
    assert_eq!(audio.frames(), FrameCount::new(100));

    let expected = [
        0.0,
        2052.0 / 32768.0,
        4097.0 / 32768.0,
        6126.0 / 32768.0,
        8130.0 / 32768.0,
        10103.0 / 32768.0,
    ];
    assert_eq!(&audio.as_planar_f32()[..expected.len()], expected);
}

#[test]
fn decode_flac_path_reads_file() {
    let path = temp_path("auralis-flac-pop", "flac");
    std::fs::write(&path, POP_FLAC).unwrap();

    let audio = decode_flac_path(&path).unwrap();

    std::fs::remove_file(path).unwrap();
    assert_eq!(audio.frames(), FrameCount::new(100));
}

#[test]
fn decode_flac_rejects_malformed_input() {
    let error = decode_flac(Cursor::new(b"not a flac".as_slice())).unwrap_err();

    assert!(matches!(error, FlacError::Malformed { .. }));
}

#[test]
fn encode_flac_writes_decodable_pcm16_stream() {
    let audio = repeated_stereo_buffer([0.0, -0.5], [0.5, 1.0], 16);
    let mut bytes = Vec::new();

    encode_flac(&mut bytes, &audio, FlacEncodeOptions).unwrap();
    let decoded = decode_flac(Cursor::new(bytes)).unwrap();

    assert_eq!(decoded.spec(), audio.spec());
    assert_eq!(decoded.frames(), FrameCount::new(32));
    assert_eq!(&decoded.channel(0).unwrap()[..4], &[0.0, 0.5, 0.0, 0.5]);
    assert_eq!(
        &decoded.channel(1).unwrap()[..4],
        &[-0.5, 32_767.0 / 32_768.0, -0.5, 32_767.0 / 32_768.0]
    );
}

#[test]
fn flac_encoder_reports_codec_boundary_summary() {
    let audio = repeated_stereo_buffer([0.0, -0.25], [0.25, 0.75], 16);
    let encoder = FlacEncoder::new(FlacEncodeOptions);
    let mut output = Cursor::new(Vec::new());

    let summary = encoder.encode(&audio, &mut output).unwrap();
    let decoded = decode_flac(Cursor::new(output.into_inner())).unwrap();

    assert_eq!(encoder.codec_kind(), CodecKind::Flac);
    assert_eq!(summary.codec_kind(), CodecKind::Flac);
    assert_eq!(summary.spec(), audio.spec());
    assert_eq!(summary.frames(), audio.frames());
    assert_eq!(&decoded.channel(0).unwrap()[..4], &[0.0, 0.25, 0.0, 0.25]);
    assert_eq!(
        &decoded.channel(1).unwrap()[..4],
        &[-0.25, 0.75, -0.25, 0.75]
    );
}

#[test]
fn encode_flac_rejects_non_finite_samples() {
    let audio = mono_buffer(vec![
        0.0,
        f32::NAN,
        0.0,
        0.0,
        0.0,
        0.0,
        0.0,
        0.0,
        0.0,
        0.0,
        0.0,
        0.0,
        0.0,
        0.0,
        0.0,
        0.0,
    ]);
    let error = encode_flac(Vec::new(), &audio, FlacEncodeOptions).unwrap_err();

    assert!(matches!(
        error,
        FlacError::NonFiniteSample {
            channel_index: 0,
            frame_index: 1
        }
    ));
}

#[test]
fn encode_flac_rejects_short_buffers_before_writing_invalid_blocks() {
    let audio = mono_buffer(vec![0.0, 0.25]);
    let error = encode_flac(Vec::new(), &audio, FlacEncodeOptions).unwrap_err();

    assert!(matches!(error, FlacError::FrameCountTooSmall { frames: 2 }));
}

fn mono_buffer(samples: Vec<f32>) -> AudioBuffer {
    let spec = AudioSpec::new(
        SampleRate::new(44_100).unwrap(),
        ChannelCount::new(1).unwrap(),
        SampleFormat::Float32,
    );
    AudioBuffer::from_planar_f32(spec, FrameCount::new(samples.len() as u64), samples).unwrap()
}

fn stereo_buffer(interleaved: &[f32]) -> AudioBuffer {
    assert_eq!(interleaved.len() % 2, 0);
    let frames = interleaved.len() / 2;
    let mut planar = vec![0.0; interleaved.len()];
    for frame in 0..frames {
        planar[frame] = interleaved[frame * 2];
        planar[frames + frame] = interleaved[frame * 2 + 1];
    }
    let spec = AudioSpec::new(
        SampleRate::new(44_100).unwrap(),
        ChannelCount::new(2).unwrap(),
        SampleFormat::Float32,
    );
    AudioBuffer::from_planar_f32(spec, FrameCount::new(frames as u64), planar).unwrap()
}

fn repeated_stereo_buffer(first: [f32; 2], second: [f32; 2], repeats: usize) -> AudioBuffer {
    let mut interleaved = Vec::with_capacity(repeats * 4);
    for _ in 0..repeats {
        interleaved.extend_from_slice(&first);
        interleaved.extend_from_slice(&second);
    }
    stereo_buffer(&interleaved)
}

fn temp_path(prefix: &str, extension: &str) -> PathBuf {
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_nanos();

    std::env::temp_dir().join(format!("{prefix}-{nanos}.{extension}"))
}

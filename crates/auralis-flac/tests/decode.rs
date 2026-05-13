#![allow(missing_docs)]

use std::{io::Cursor, path::PathBuf};

use auralis_core::{FrameCount, SampleFormat};
use auralis_flac::{FlacError, decode_flac, decode_flac_path};

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

fn temp_path(prefix: &str, extension: &str) -> PathBuf {
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_nanos();

    std::env::temp_dir().join(format!("{prefix}-{nanos}.{extension}"))
}

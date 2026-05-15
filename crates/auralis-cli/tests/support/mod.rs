//! Shared helpers for CLI integration tests.

#![allow(dead_code)]

pub use std::{fs, process::Command};

use std::{
    path::{Path, PathBuf},
    time::{SystemTime, UNIX_EPOCH},
};

pub fn write_pcm16_wav(path: &Path, channels: u16, samples: &[i16]) {
    write_pcm16_wav_with_sample_rate(path, 48_000, channels, samples);
}

pub fn write_pcm16_wav_with_sample_rate(
    path: &Path,
    sample_rate: u32,
    channels: u16,
    samples: &[i16],
) {
    let mut bytes = riff_header(sample_rate, channels, 16, 1, samples.len() * 2);
    for sample in samples {
        bytes.extend_from_slice(&sample.to_le_bytes());
    }
    fs::write(path, bytes).unwrap();
}

pub fn write_pcm16_wav_with_metadata(path: &Path, channels: u16, samples: &[i16]) {
    let data_bytes = samples.len() * 2;
    let metadata = b"LIST\x04\0\0\0INFO";
    let mut bytes = riff_header_with_extra(48_000, channels, 16, 1, data_bytes, metadata.len());
    bytes.extend_from_slice(metadata);
    bytes.extend_from_slice(b"data");
    bytes.extend_from_slice(&u32::try_from(data_bytes).unwrap().to_le_bytes());
    for sample in samples {
        bytes.extend_from_slice(&sample.to_le_bytes());
    }
    fs::write(path, bytes).unwrap();
}

pub fn write_float_wav(path: &Path) {
    let mut bytes = riff_header(48_000, 1, 32, 3, 4);
    bytes.extend_from_slice(&0.0_f32.to_le_bytes());
    fs::write(path, bytes).unwrap();
}

fn riff_header(
    sample_rate: u32,
    channels: u16,
    bits_per_sample: u16,
    format_tag: u16,
    data_bytes: usize,
) -> Vec<u8> {
    let mut bytes = riff_header_with_extra(
        sample_rate,
        channels,
        bits_per_sample,
        format_tag,
        data_bytes,
        0,
    );

    bytes.extend_from_slice(b"data");
    bytes.extend_from_slice(&u32::try_from(data_bytes).unwrap().to_le_bytes());

    bytes
}

fn riff_header_with_extra(
    sample_rate: u32,
    channels: u16,
    bits_per_sample: u16,
    format_tag: u16,
    data_bytes: usize,
    extra_bytes: usize,
) -> Vec<u8> {
    let bytes_per_sample = u32::from(bits_per_sample) / 8;
    let byte_rate = sample_rate * u32::from(channels) * bytes_per_sample;
    let block_align = channels * (bits_per_sample / 8);
    let riff_size = 36 + u32::try_from(data_bytes).unwrap() + u32::try_from(extra_bytes).unwrap();
    let mut bytes = Vec::new();

    bytes.extend_from_slice(b"RIFF");
    bytes.extend_from_slice(&riff_size.to_le_bytes());
    bytes.extend_from_slice(b"WAVEfmt ");
    bytes.extend_from_slice(&16_u32.to_le_bytes());
    bytes.extend_from_slice(&format_tag.to_le_bytes());
    bytes.extend_from_slice(&channels.to_le_bytes());
    bytes.extend_from_slice(&sample_rate.to_le_bytes());
    bytes.extend_from_slice(&byte_rate.to_le_bytes());
    bytes.extend_from_slice(&block_align.to_le_bytes());
    bytes.extend_from_slice(&bits_per_sample.to_le_bytes());

    bytes
}

pub fn read_pcm16_wav(path: &Path) -> (u16, Vec<i16>) {
    let (_, channels, samples) = read_pcm16_wav_with_sample_rate(path);
    (channels, samples)
}

pub fn read_pcm16_wav_with_sample_rate(path: &Path) -> (u32, u16, Vec<i16>) {
    let bytes = fs::read(path).unwrap();
    assert_eq!(&bytes[0..4], b"RIFF");
    assert_eq!(&bytes[8..12], b"WAVE");
    let channels = u16::from_le_bytes(bytes[22..24].try_into().unwrap());
    let sample_rate = u32::from_le_bytes(bytes[24..28].try_into().unwrap());
    let data_offset = data_chunk_offset(&bytes);
    let data_len = u32::from_le_bytes(bytes[data_offset + 4..data_offset + 8].try_into().unwrap());
    let data_start = data_offset + 8;
    let data_end = data_start + usize::try_from(data_len).unwrap();
    let samples = bytes[data_start..data_end]
        .chunks_exact(2)
        .map(|sample| i16::from_le_bytes(sample.try_into().unwrap()))
        .collect();

    (sample_rate, channels, samples)
}

pub fn read_wav_bits_per_sample(path: &Path) -> u16 {
    let bytes = fs::read(path).unwrap();
    assert_eq!(&bytes[0..4], b"RIFF");
    assert_eq!(&bytes[8..12], b"WAVE");
    u16::from_le_bytes(bytes[34..36].try_into().unwrap())
}

pub fn metadata_chunk_is_absent(path: &Path) -> bool {
    !fs::read(path)
        .unwrap()
        .windows(4)
        .any(|window| window == b"LIST")
}

fn data_chunk_offset(bytes: &[u8]) -> usize {
    let mut offset = 12;

    while offset + 8 <= bytes.len() {
        let chunk_len = usize::try_from(u32::from_le_bytes(
            bytes[offset + 4..offset + 8].try_into().unwrap(),
        ))
        .unwrap();
        if &bytes[offset..offset + 4] == b"data" {
            return offset;
        }
        offset += 8 + chunk_len;
    }

    panic!("missing data chunk");
}

pub fn temp_path(prefix: &str, extension: &str) -> PathBuf {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();

    std::env::temp_dir().join(format!(
        "{prefix}-{}-{nanos}.{extension}",
        std::process::id()
    ))
}

pub fn stdout(output: &std::process::Output) -> String {
    String::from_utf8(output.stdout.clone()).unwrap()
}

pub fn stderr(output: &std::process::Output) -> String {
    String::from_utf8(output.stderr.clone()).unwrap()
}

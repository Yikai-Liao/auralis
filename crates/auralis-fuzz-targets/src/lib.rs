//! Reusable L7 fuzz target drivers.
//!
//! The functions in this crate are intentionally tiny adapters around public
//! parser and codec boundaries. They can be called from deterministic smoke
//! tests on stable Rust and from libFuzzer wrappers in the top-level `fuzz/`
//! package when `cargo-fuzz` is installed.

use std::{io::Cursor, str};

use auralis_effects::{parse_effect_command, parse_effects_file_str};
use auralis_testkit::golden::GoldenManifest;
use auralis_wav::decode_pcm16;

const MAX_TEXT_BYTES: usize = 16 * 1024;
const MAX_COMMAND_TOKENS: usize = 32;
const MAX_WAV_PAYLOAD_BYTES: usize = 4096;

/// Exercises PCM16 WAV parser behavior with arbitrary byte streams.
pub fn fuzz_wav_parser(input: &[u8]) {
    let _ = decode_pcm16(Cursor::new(input));
}

/// Exercises explicit WAV unsupported-format rejection paths.
///
/// The generated stream keeps the RIFF/WAVE structure recognizable while using
/// fuzz-controlled format tags, bit depths, channel counts, sample rates, and
/// payload bytes. Valid PCM16 variants are also allowed through so this target
/// covers the format gate without overfitting to only invalid headers.
pub fn fuzz_unsupported_wav_format(input: &[u8]) {
    let wav = wav_with_fuzzed_format(input);
    let _ = decode_pcm16(Cursor::new(wav));
}

/// Exercises typed effect command parsing with arbitrary UTF-8 token streams.
pub fn fuzz_effect_command(input: &[u8]) {
    let Some(source) = bounded_utf8(input) else {
        return;
    };
    let tokens = source
        .split_whitespace()
        .take(MAX_COMMAND_TOKENS)
        .collect::<Vec<_>>();

    let _ = parse_effect_command(&tokens);
}

/// Exercises effects-file tokenization, boundary handling, and command parsing.
pub fn fuzz_effects_file(input: &[u8]) {
    let Some(source) = bounded_utf8(input) else {
        return;
    };

    let _ = parse_effects_file_str(source);
}

/// Exercises TOML golden manifest parsing and validation.
///
/// Auralis does not yet have the future pipeline-manifest parser mentioned in
/// the 5.7.6 plan, so the current manifest fuzz target covers the checked-in
/// TOML golden-manifest boundary until a pipeline manifest exists.
pub fn fuzz_golden_manifest(input: &[u8]) {
    let Some(source) = bounded_utf8(input) else {
        return;
    };

    let _ = GoldenManifest::parse_toml(source);
}

fn bounded_utf8(input: &[u8]) -> Option<&str> {
    str::from_utf8(input.get(..input.len().min(MAX_TEXT_BYTES))?).ok()
}

fn wav_with_fuzzed_format(input: &[u8]) -> Vec<u8> {
    let format_tag = le_u16(input, 0);
    let channels = le_u16(input, 2);
    let sample_rate = le_u32(input, 4);
    let bits_per_sample = le_u16(input, 8);
    let bytes_per_sample = u32::from(bits_per_sample).div_ceil(8).max(1);
    let block_align = u32::from(channels).saturating_mul(bytes_per_sample);
    let byte_rate = sample_rate.saturating_mul(block_align);
    let payload = input
        .get(10..)
        .unwrap_or_default()
        .get(..input.len().saturating_sub(10).min(MAX_WAV_PAYLOAD_BYTES))
        .unwrap_or_default();
    let data_len = u32::try_from(payload.len()).unwrap_or(u32::MAX);

    let mut bytes = Vec::with_capacity(44 + payload.len());
    bytes.extend_from_slice(b"RIFF");
    bytes.extend_from_slice(&(36_u32.saturating_add(data_len)).to_le_bytes());
    bytes.extend_from_slice(b"WAVEfmt ");
    bytes.extend_from_slice(&16_u32.to_le_bytes());
    bytes.extend_from_slice(&format_tag.to_le_bytes());
    bytes.extend_from_slice(&channels.to_le_bytes());
    bytes.extend_from_slice(&sample_rate.to_le_bytes());
    bytes.extend_from_slice(&byte_rate.to_le_bytes());
    bytes.extend_from_slice(&u16::try_from(block_align).unwrap_or(u16::MAX).to_le_bytes());
    bytes.extend_from_slice(&bits_per_sample.to_le_bytes());
    bytes.extend_from_slice(b"data");
    bytes.extend_from_slice(&data_len.to_le_bytes());
    bytes.extend_from_slice(payload);
    bytes
}

fn le_u16(input: &[u8], offset: usize) -> u16 {
    let low = input.get(offset).copied().unwrap_or_default();
    let high = input.get(offset + 1).copied().unwrap_or_default();

    u16::from_le_bytes([low, high])
}

fn le_u32(input: &[u8], offset: usize) -> u32 {
    let b0 = input.get(offset).copied().unwrap_or_default();
    let b1 = input.get(offset + 1).copied().unwrap_or_default();
    let b2 = input.get(offset + 2).copied().unwrap_or_default();
    let b3 = input.get(offset + 3).copied().unwrap_or_default();

    u32::from_le_bytes([b0, b1, b2, b3])
}

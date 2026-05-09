//! Integration tests for the `auralis inspect` command.

use std::{
    fs,
    path::{Path, PathBuf},
    process::Command,
    time::{SystemTime, UNIX_EPOCH},
};

#[test]
fn inspect_reports_pcm16_wav_fields() {
    let path = temp_path("auralis-cli-inspect", "wav");
    write_pcm16_wav(&path, 2, &[-32768, 32767, 0, 8192, 16384, -16384]);

    let output = Command::new(env!("CARGO_BIN_EXE_auralis"))
        .args(["inspect", path.to_str().unwrap()])
        .output()
        .unwrap();

    fs::remove_file(path).unwrap();
    assert!(output.status.success(), "stderr: {}", stderr(&output));
    let stdout = stdout(&output);
    assert!(stdout.contains("format: wav"));
    assert!(stdout.contains("sample_rate: 48000"));
    assert!(stdout.contains("channels: 2"));
    assert!(stdout.contains("sample_format: pcm16"));
    assert!(stdout.contains("duration_frames: 3"));
    assert!(stdout.contains("duration_seconds: 0.000062500"));
}

#[test]
fn inspect_missing_path_returns_clear_error() {
    let path = temp_path("auralis-cli-inspect-missing", "wav");

    let output = Command::new(env!("CARGO_BIN_EXE_auralis"))
        .args(["inspect", path.to_str().unwrap()])
        .output()
        .unwrap();

    assert!(!output.status.success());
    let stderr = stderr(&output);
    assert!(
        stderr.contains("error: could not open WAV input"),
        "{stderr}"
    );
}

#[test]
fn inspect_unsupported_extension_returns_clear_error() {
    let path = temp_path("auralis-cli-inspect-unsupported", "flac");
    fs::write(&path, b"not a supported input").unwrap();

    let output = Command::new(env!("CARGO_BIN_EXE_auralis"))
        .args(["inspect", path.to_str().unwrap()])
        .output()
        .unwrap();

    fs::remove_file(path).unwrap();
    assert!(!output.status.success());
    let stderr = stderr(&output);
    assert!(
        stderr.contains("error: unsupported input format"),
        "{stderr}"
    );
    assert!(stderr.contains("only PCM16 WAV is supported"), "{stderr}");
}

#[test]
fn inspect_unsupported_wav_sample_format_returns_clear_error() {
    let path = temp_path("auralis-cli-inspect-float", "wav");
    write_float_wav(&path);

    let output = Command::new(env!("CARGO_BIN_EXE_auralis"))
        .args(["inspect", path.to_str().unwrap()])
        .output()
        .unwrap();

    fs::remove_file(path).unwrap();
    assert!(!output.status.success());
    let stderr = stderr(&output);
    assert!(
        stderr.contains("error: unsupported WAV sample format"),
        "{stderr}"
    );
}

fn write_pcm16_wav(path: &Path, channels: u16, samples: &[i16]) {
    let mut bytes = riff_header(channels, 16, 1, samples.len() * 2);
    for sample in samples {
        bytes.extend_from_slice(&sample.to_le_bytes());
    }
    fs::write(path, bytes).unwrap();
}

fn write_float_wav(path: &Path) {
    let mut bytes = riff_header(1, 32, 3, 4);
    bytes.extend_from_slice(&0.0_f32.to_le_bytes());
    fs::write(path, bytes).unwrap();
}

fn riff_header(channels: u16, bits_per_sample: u16, format_tag: u16, data_bytes: usize) -> Vec<u8> {
    let sample_rate = 48_000_u32;
    let bytes_per_sample = u32::from(bits_per_sample) / 8;
    let byte_rate = sample_rate * u32::from(channels) * bytes_per_sample;
    let block_align = channels * (bits_per_sample / 8);
    let riff_size = 36 + u32::try_from(data_bytes).unwrap();
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
    bytes.extend_from_slice(b"data");
    bytes.extend_from_slice(&u32::try_from(data_bytes).unwrap().to_le_bytes());

    bytes
}

fn temp_path(prefix: &str, extension: &str) -> PathBuf {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();

    std::env::temp_dir().join(format!(
        "{prefix}-{}-{nanos}.{extension}",
        std::process::id()
    ))
}

fn stdout(output: &std::process::Output) -> String {
    String::from_utf8(output.stdout.clone()).unwrap()
}

fn stderr(output: &std::process::Output) -> String {
    String::from_utf8(output.stderr.clone()).unwrap()
}

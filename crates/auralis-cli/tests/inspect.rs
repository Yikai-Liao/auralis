//! Integration tests for the `auralis inspect` command.

mod support;

use support::*;

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

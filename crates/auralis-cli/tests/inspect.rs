//! Integration tests for the `auralis inspect` command.

use std::{
    fs,
    path::{Path, PathBuf},
    process::Command,
    time::{SystemTime, UNIX_EPOCH},
};

use auralis::AudioFile;

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

#[test]
fn run_copies_mono_wav_through_decode_encode_pipeline() {
    let input = temp_path("auralis-cli-run-mono-input", "wav");
    let output = temp_path("auralis-cli-run-mono-output", "wav");
    let samples = [-32768, -1024, 0, 1024, 32767];
    write_pcm16_wav_with_metadata(&input, 1, &samples);

    let command_output = Command::new(env!("CARGO_BIN_EXE_auralis"))
        .args(["run", input.to_str().unwrap(), output.to_str().unwrap()])
        .output()
        .unwrap();

    assert!(
        command_output.status.success(),
        "stderr: {}",
        stderr(&command_output)
    );
    assert_eq!(read_pcm16_wav(&output), (1, samples.to_vec()));
    assert!(metadata_chunk_is_absent(&output));
    assert_ne!(fs::read(&input).unwrap(), fs::read(&output).unwrap());

    let inspect_output = Command::new(env!("CARGO_BIN_EXE_auralis"))
        .args(["inspect", output.to_str().unwrap()])
        .output()
        .unwrap();
    assert!(inspect_output.status.success());
    let stdout = stdout(&inspect_output);
    assert!(stdout.contains("sample_rate: 48000"));
    assert!(stdout.contains("channels: 1"));
    assert!(stdout.contains("duration_frames: 5"));

    fs::remove_file(input).unwrap();
    fs::remove_file(output).unwrap();
}

#[test]
fn run_copies_stereo_wav_samples_and_metadata() {
    let input = temp_path("auralis-cli-run-stereo-input", "wav");
    let output = temp_path("auralis-cli-run-stereo-output", "wav");
    let samples = [-32768, 32767, -12_000, 12_000, 0, 4096];
    write_pcm16_wav(&input, 2, &samples);

    let command_output = Command::new(env!("CARGO_BIN_EXE_auralis"))
        .args(["run", input.to_str().unwrap(), output.to_str().unwrap()])
        .output()
        .unwrap();

    fs::remove_file(input).unwrap();
    assert!(
        command_output.status.success(),
        "stderr: {}",
        stderr(&command_output)
    );
    assert_eq!(read_pcm16_wav(&output), (2, samples.to_vec()));

    let inspect_output = Command::new(env!("CARGO_BIN_EXE_auralis"))
        .args(["inspect", output.to_str().unwrap()])
        .output()
        .unwrap();
    fs::remove_file(output).unwrap();
    assert!(inspect_output.status.success());
    let stdout = stdout(&inspect_output);
    assert!(stdout.contains("channels: 2"));
    assert!(stdout.contains("duration_frames: 3"));
}

#[test]
fn run_channels_downmixes_stereo_to_mono() {
    let input = temp_path("auralis-cli-run-channels-downmix-input", "wav");
    let output = temp_path("auralis-cli-run-channels-downmix-output", "wav");
    write_pcm16_wav(&input, 2, &[8192, 24_576, -16_384, 16_384]);

    let command_output = Command::new(env!("CARGO_BIN_EXE_auralis"))
        .args([
            "run",
            input.to_str().unwrap(),
            output.to_str().unwrap(),
            "--channels",
            "1",
        ])
        .output()
        .unwrap();

    fs::remove_file(input).unwrap();
    assert!(
        command_output.status.success(),
        "stderr: {}",
        stderr(&command_output)
    );
    assert_eq!(read_pcm16_wav(&output), (1, vec![16_384, 0]));
    fs::remove_file(output).unwrap();
}

#[test]
fn run_channels_upmixes_mono_to_stereo() {
    let input = temp_path("auralis-cli-run-channels-upmix-input", "wav");
    let output = temp_path("auralis-cli-run-channels-upmix-output", "wav");
    write_pcm16_wav(&input, 1, &[8192, -16_384]);

    let command_output = Command::new(env!("CARGO_BIN_EXE_auralis"))
        .args([
            "run",
            input.to_str().unwrap(),
            output.to_str().unwrap(),
            "--channels",
            "2",
        ])
        .output()
        .unwrap();

    fs::remove_file(input).unwrap();
    assert!(
        command_output.status.success(),
        "stderr: {}",
        stderr(&command_output)
    );
    assert_eq!(
        read_pcm16_wav(&output),
        (2, vec![8192, 8192, -16_384, -16_384])
    );
    fs::remove_file(output).unwrap();
}

#[test]
fn run_no_auto_channels_rejects_mismatched_output_count() {
    let input = temp_path("auralis-cli-run-no-auto-channels-input", "wav");
    let output = temp_path("auralis-cli-run-no-auto-channels-output", "wav");
    write_pcm16_wav(&input, 2, &[1000, -1000]);

    let command_output = Command::new(env!("CARGO_BIN_EXE_auralis"))
        .args([
            "run",
            input.to_str().unwrap(),
            output.to_str().unwrap(),
            "--channels",
            "1",
            "--no-auto-channels",
        ])
        .output()
        .unwrap();

    fs::remove_file(input).unwrap();
    let _ = fs::remove_file(output);
    assert!(!command_output.status.success());
    let stderr = stderr(&command_output);
    assert!(
        stderr.contains("automatic channel conversion from 2 ch to 1 ch is disabled"),
        "{stderr}"
    );
}

#[test]
fn run_no_auto_channels_requires_output_channels() {
    let input = temp_path("auralis-cli-run-no-auto-channels-missing-input", "wav");
    let output = temp_path("auralis-cli-run-no-auto-channels-missing-output", "wav");
    write_pcm16_wav(&input, 1, &[1000]);

    let command_output = Command::new(env!("CARGO_BIN_EXE_auralis"))
        .args([
            "run",
            input.to_str().unwrap(),
            output.to_str().unwrap(),
            "--no-auto-channels",
        ])
        .output()
        .unwrap();

    fs::remove_file(input).unwrap();
    let _ = fs::remove_file(output);
    assert!(!command_output.status.success());
    let stderr = stderr(&command_output);
    assert!(
        stderr.contains("error: --no-auto-channels requires --channels"),
        "{stderr}"
    );
}

#[test]
fn run_rate_downsamples_output_sample_rate() {
    let input = temp_path("auralis-cli-run-rate-downsample-input", "wav");
    let output = temp_path("auralis-cli-run-rate-downsample-output", "wav");
    write_pcm16_wav(&input, 1, &[1000, 1000, 1000, 1000]);

    let command_output = Command::new(env!("CARGO_BIN_EXE_auralis"))
        .args([
            "run",
            input.to_str().unwrap(),
            output.to_str().unwrap(),
            "--rate",
            "24000",
        ])
        .output()
        .unwrap();

    fs::remove_file(input).unwrap();
    assert!(
        command_output.status.success(),
        "stderr: {}",
        stderr(&command_output)
    );
    assert_eq!(
        read_pcm16_wav_with_sample_rate(&output),
        (24_000, 1, vec![1000, 1000])
    );
    fs::remove_file(output).unwrap();
}

#[test]
fn run_rate_upsamples_output_sample_rate() {
    let input = temp_path("auralis-cli-run-rate-upsample-input", "wav");
    let output = temp_path("auralis-cli-run-rate-upsample-output", "wav");
    write_pcm16_wav(&input, 1, &[1000, 1000]);

    let command_output = Command::new(env!("CARGO_BIN_EXE_auralis"))
        .args([
            "run",
            input.to_str().unwrap(),
            output.to_str().unwrap(),
            "--rate",
            "96000",
        ])
        .output()
        .unwrap();

    fs::remove_file(input).unwrap();
    assert!(
        command_output.status.success(),
        "stderr: {}",
        stderr(&command_output)
    );
    assert_eq!(
        read_pcm16_wav_with_sample_rate(&output),
        (96_000, 1, vec![1000, 1000, 1000, 1000])
    );
    fs::remove_file(output).unwrap();
}

#[test]
fn run_no_auto_rate_rejects_mismatched_output_rate() {
    let input = temp_path("auralis-cli-run-no-auto-rate-input", "wav");
    let output = temp_path("auralis-cli-run-no-auto-rate-output", "wav");
    write_pcm16_wav(&input, 1, &[1000, -1000]);

    let command_output = Command::new(env!("CARGO_BIN_EXE_auralis"))
        .args([
            "run",
            input.to_str().unwrap(),
            output.to_str().unwrap(),
            "--rate",
            "24000",
            "--no-auto-rate",
        ])
        .output()
        .unwrap();

    fs::remove_file(input).unwrap();
    let _ = fs::remove_file(output);
    assert!(!command_output.status.success());
    let stderr = stderr(&command_output);
    assert!(
        stderr.contains("automatic sample-rate conversion from 48000 Hz to 24000 Hz is disabled"),
        "{stderr}"
    );
}

#[test]
fn run_no_auto_rate_requires_output_rate() {
    let input = temp_path("auralis-cli-run-no-auto-rate-missing-input", "wav");
    let output = temp_path("auralis-cli-run-no-auto-rate-missing-output", "wav");
    write_pcm16_wav(&input, 1, &[1000]);

    let command_output = Command::new(env!("CARGO_BIN_EXE_auralis"))
        .args([
            "run",
            input.to_str().unwrap(),
            output.to_str().unwrap(),
            "--no-auto-rate",
        ])
        .output()
        .unwrap();

    fs::remove_file(input).unwrap();
    let _ = fs::remove_file(output);
    assert!(!command_output.status.success());
    let stderr = stderr(&command_output);
    assert!(
        stderr.contains("error: --no-auto-rate requires --rate"),
        "{stderr}"
    );
}

#[test]
fn run_concatenate_accepts_mismatched_mono_lengths() {
    let first = temp_path("auralis-cli-run-concat-mono-first", "wav");
    let second = temp_path("auralis-cli-run-concat-mono-second", "wav");
    let output = temp_path("auralis-cli-run-concat-mono-output", "wav");
    write_pcm16_wav(&first, 1, &[1000, -1000]);
    write_pcm16_wav(&second, 1, &[500, 0, -500]);

    let command_output = Command::new(env!("CARGO_BIN_EXE_auralis"))
        .args([
            "run",
            first.to_str().unwrap(),
            output.to_str().unwrap(),
            "--combine",
            "concatenate",
            "--input",
            second.to_str().unwrap(),
        ])
        .output()
        .unwrap();

    assert!(
        command_output.status.success(),
        "stderr: {}",
        stderr(&command_output)
    );
    assert_eq!(
        read_pcm16_wav(&output),
        (1, vec![1000, -1000, 500, 0, -500])
    );

    fs::remove_file(first).unwrap();
    fs::remove_file(second).unwrap();
    fs::remove_file(output).unwrap();
}

#[test]
fn run_concatenate_combines_stereo_inputs_before_effects() {
    let first = temp_path("auralis-cli-run-concat-stereo-first", "wav");
    let second = temp_path("auralis-cli-run-concat-stereo-second", "wav");
    let output = temp_path("auralis-cli-run-concat-stereo-output", "wav");
    write_pcm16_wav(&first, 2, &[-1000, 1000, -2000, 2000]);
    write_pcm16_wav(&second, 2, &[-3000, 3000]);

    let command_output = Command::new(env!("CARGO_BIN_EXE_auralis"))
        .args([
            "run",
            first.to_str().unwrap(),
            output.to_str().unwrap(),
            "--combine",
            "concatenate",
            "--input",
            second.to_str().unwrap(),
            "--reverse",
        ])
        .output()
        .unwrap();

    assert!(
        command_output.status.success(),
        "stderr: {}",
        stderr(&command_output)
    );
    assert_eq!(
        read_pcm16_wav(&output),
        (2, vec![-3000, 3000, -2000, 2000, -1000, 1000])
    );

    fs::remove_file(first).unwrap();
    fs::remove_file(second).unwrap();
    fs::remove_file(output).unwrap();
}

#[test]
fn run_concatenate_rejects_mismatched_channel_count() {
    let first = temp_path("auralis-cli-run-concat-mismatch-first", "wav");
    let second = temp_path("auralis-cli-run-concat-mismatch-second", "wav");
    let output = temp_path("auralis-cli-run-concat-mismatch-output", "wav");
    write_pcm16_wav(&first, 1, &[1000, -1000]);
    write_pcm16_wav(&second, 2, &[500, -500]);

    let command_output = Command::new(env!("CARGO_BIN_EXE_auralis"))
        .args([
            "run",
            first.to_str().unwrap(),
            output.to_str().unwrap(),
            "--combine",
            "concatenate",
            "--input",
            second.to_str().unwrap(),
        ])
        .output()
        .unwrap();

    fs::remove_file(first).unwrap();
    fs::remove_file(second).unwrap();
    let _ = fs::remove_file(output);
    assert!(!command_output.status.success());
    let stderr = stderr(&command_output);
    assert!(stderr.contains("channel count"), "{stderr}");
    assert!(
        stderr.contains("does not match first input channel count"),
        "{stderr}"
    );
}

#[test]
fn run_sequence_accepts_mismatched_mono_lengths() {
    let first = temp_path("auralis-cli-run-sequence-mono-first", "wav");
    let second = temp_path("auralis-cli-run-sequence-mono-second", "wav");
    let output = temp_path("auralis-cli-run-sequence-mono-output", "wav");
    write_pcm16_wav(&first, 1, &[1000, -1000]);
    write_pcm16_wav(&second, 1, &[500, 0, -500]);

    let command_output = Command::new(env!("CARGO_BIN_EXE_auralis"))
        .args([
            "run",
            first.to_str().unwrap(),
            output.to_str().unwrap(),
            "--combine",
            "sequence",
            "--input",
            second.to_str().unwrap(),
        ])
        .output()
        .unwrap();

    assert!(
        command_output.status.success(),
        "stderr: {}",
        stderr(&command_output)
    );
    assert_eq!(
        read_pcm16_wav(&output),
        (1, vec![1000, -1000, 500, 0, -500])
    );

    fs::remove_file(first).unwrap();
    fs::remove_file(second).unwrap();
    fs::remove_file(output).unwrap();
}

#[test]
fn run_sequence_combines_stereo_inputs_before_effects() {
    let first = temp_path("auralis-cli-run-sequence-stereo-first", "wav");
    let second = temp_path("auralis-cli-run-sequence-stereo-second", "wav");
    let output = temp_path("auralis-cli-run-sequence-stereo-output", "wav");
    write_pcm16_wav(&first, 2, &[-1000, 1000, -2000, 2000]);
    write_pcm16_wav(&second, 2, &[-3000, 3000]);

    let command_output = Command::new(env!("CARGO_BIN_EXE_auralis"))
        .args([
            "run",
            first.to_str().unwrap(),
            output.to_str().unwrap(),
            "--combine",
            "sequence",
            "--input",
            second.to_str().unwrap(),
            "--reverse",
        ])
        .output()
        .unwrap();

    assert!(
        command_output.status.success(),
        "stderr: {}",
        stderr(&command_output)
    );
    assert_eq!(
        read_pcm16_wav(&output),
        (2, vec![-3000, 3000, -2000, 2000, -1000, 1000])
    );

    fs::remove_file(first).unwrap();
    fs::remove_file(second).unwrap();
    fs::remove_file(output).unwrap();
}

#[test]
fn run_sequence_rejects_unrepresentable_channel_boundary() {
    let first = temp_path("auralis-cli-run-sequence-mismatch-first", "wav");
    let second = temp_path("auralis-cli-run-sequence-mismatch-second", "wav");
    let output = temp_path("auralis-cli-run-sequence-mismatch-output", "wav");
    write_pcm16_wav(&first, 1, &[1000, -1000]);
    write_pcm16_wav(&second, 2, &[500, -500]);

    let command_output = Command::new(env!("CARGO_BIN_EXE_auralis"))
        .args([
            "run",
            first.to_str().unwrap(),
            output.to_str().unwrap(),
            "--combine",
            "sequence",
            "--input",
            second.to_str().unwrap(),
        ])
        .output()
        .unwrap();

    fs::remove_file(first).unwrap();
    fs::remove_file(second).unwrap();
    let _ = fs::remove_file(output);
    assert!(!command_output.status.success());
    let stderr = stderr(&command_output);
    assert!(
        stderr.contains("sequence boundary before input 1"),
        "{stderr}"
    );
    assert!(
        stderr.contains("cannot be represented in one output"),
        "{stderr}"
    );
}

#[test]
fn run_mix_averages_equal_length_mono_inputs() {
    let first = temp_path("auralis-cli-run-mix-mono-first", "wav");
    let second = temp_path("auralis-cli-run-mix-mono-second", "wav");
    let output = temp_path("auralis-cli-run-mix-mono-output", "wav");
    write_pcm16_wav(&first, 1, &[1000, -1000]);
    write_pcm16_wav(&second, 1, &[3000, 1000]);

    let command_output = Command::new(env!("CARGO_BIN_EXE_auralis"))
        .args([
            "run",
            first.to_str().unwrap(),
            output.to_str().unwrap(),
            "--combine",
            "mix",
            "--input",
            second.to_str().unwrap(),
        ])
        .output()
        .unwrap();

    assert!(
        command_output.status.success(),
        "stderr: {}",
        stderr(&command_output)
    );
    assert_eq!(read_pcm16_wav(&output), (1, vec![2000, 0]));

    fs::remove_file(first).unwrap();
    fs::remove_file(second).unwrap();
    fs::remove_file(output).unwrap();
}

#[test]
fn run_mix_treats_mismatched_lengths_as_trailing_silence() {
    let first = temp_path("auralis-cli-run-mix-length-first", "wav");
    let second = temp_path("auralis-cli-run-mix-length-second", "wav");
    let output = temp_path("auralis-cli-run-mix-length-output", "wav");
    write_pcm16_wav(&first, 1, &[1000, -1000]);
    write_pcm16_wav(&second, 1, &[500, 0, -500]);

    let command_output = Command::new(env!("CARGO_BIN_EXE_auralis"))
        .args([
            "run",
            first.to_str().unwrap(),
            output.to_str().unwrap(),
            "--combine",
            "mix",
            "--input",
            second.to_str().unwrap(),
        ])
        .output()
        .unwrap();

    assert!(
        command_output.status.success(),
        "stderr: {}",
        stderr(&command_output)
    );
    assert_eq!(read_pcm16_wav(&output), (1, vec![750, -500, -250]));

    fs::remove_file(first).unwrap();
    fs::remove_file(second).unwrap();
    fs::remove_file(output).unwrap();
}

#[test]
fn run_mix_combines_stereo_inputs_before_effects() {
    let first = temp_path("auralis-cli-run-mix-stereo-first", "wav");
    let second = temp_path("auralis-cli-run-mix-stereo-second", "wav");
    let output = temp_path("auralis-cli-run-mix-stereo-output", "wav");
    write_pcm16_wav(&first, 2, &[-1000, 1000, -2000, 2000]);
    write_pcm16_wav(&second, 2, &[3000, -1000, 1000, -3000]);

    let command_output = Command::new(env!("CARGO_BIN_EXE_auralis"))
        .args([
            "run",
            first.to_str().unwrap(),
            output.to_str().unwrap(),
            "--combine",
            "mix",
            "--input",
            second.to_str().unwrap(),
            "--reverse",
        ])
        .output()
        .unwrap();

    assert!(
        command_output.status.success(),
        "stderr: {}",
        stderr(&command_output)
    );
    assert_eq!(read_pcm16_wav(&output), (2, vec![-500, -500, 1000, 0]));

    fs::remove_file(first).unwrap();
    fs::remove_file(second).unwrap();
    fs::remove_file(output).unwrap();
}

#[test]
fn run_mix_accepts_mismatched_channel_counts() {
    let first = temp_path("auralis-cli-run-mix-channel-first", "wav");
    let second = temp_path("auralis-cli-run-mix-channel-second", "wav");
    let output = temp_path("auralis-cli-run-mix-channel-output", "wav");
    write_pcm16_wav(&first, 1, &[1000, -1000]);
    write_pcm16_wav(&second, 2, &[3000, 1000, -1000, 500]);

    let command_output = Command::new(env!("CARGO_BIN_EXE_auralis"))
        .args([
            "run",
            first.to_str().unwrap(),
            output.to_str().unwrap(),
            "--combine",
            "mix",
            "--input",
            second.to_str().unwrap(),
        ])
        .output()
        .unwrap();

    assert!(
        command_output.status.success(),
        "stderr: {}",
        stderr(&command_output)
    );
    assert_eq!(read_pcm16_wav(&output), (2, vec![2000, 500, -1000, 250]));

    fs::remove_file(first).unwrap();
    fs::remove_file(second).unwrap();
    fs::remove_file(output).unwrap();
}

#[test]
fn run_mix_backend_scalar_and_requested_simd_match() {
    let first = temp_path("auralis-cli-run-mix-backend-first", "wav");
    let second = temp_path("auralis-cli-run-mix-backend-second", "wav");
    let scalar_output = temp_path("auralis-cli-run-mix-backend-scalar-output", "wav");
    let simd_output = temp_path("auralis-cli-run-mix-backend-simd-output", "wav");
    write_pcm16_wav(&first, 1, &[-32768, -12345, 0, 12345, 32767]);
    write_pcm16_wav(&second, 1, &[32767, 12345, 0, -12345, -32768]);

    let scalar = Command::new(env!("CARGO_BIN_EXE_auralis"))
        .args([
            "run",
            first.to_str().unwrap(),
            scalar_output.to_str().unwrap(),
            "--backend",
            "scalar",
            "--combine",
            "mix",
            "--input",
            second.to_str().unwrap(),
        ])
        .output()
        .unwrap();
    let simd = Command::new(env!("CARGO_BIN_EXE_auralis"))
        .args([
            "run",
            first.to_str().unwrap(),
            simd_output.to_str().unwrap(),
            "--backend",
            "simd",
            "--combine",
            "mix",
            "--input",
            second.to_str().unwrap(),
        ])
        .output()
        .unwrap();

    assert!(scalar.status.success(), "stderr: {}", stderr(&scalar));
    assert!(simd.status.success(), "stderr: {}", stderr(&simd));
    assert_eq!(read_pcm16_wav(&simd_output), read_pcm16_wav(&scalar_output));

    fs::remove_file(first).unwrap();
    fs::remove_file(second).unwrap();
    fs::remove_file(scalar_output).unwrap();
    fs::remove_file(simd_output).unwrap();
}

#[test]
fn run_mix_power_scales_equal_length_mono_inputs() {
    let first = temp_path("auralis-cli-run-mix-power-mono-first", "wav");
    let second = temp_path("auralis-cli-run-mix-power-mono-second", "wav");
    let output = temp_path("auralis-cli-run-mix-power-mono-output", "wav");
    write_pcm16_wav(&first, 1, &[1000, -1000]);
    write_pcm16_wav(&second, 1, &[3000, 1000]);

    let command_output = Command::new(env!("CARGO_BIN_EXE_auralis"))
        .args([
            "run",
            first.to_str().unwrap(),
            output.to_str().unwrap(),
            "--combine",
            "mix-power",
            "--input",
            second.to_str().unwrap(),
        ])
        .output()
        .unwrap();

    assert!(
        command_output.status.success(),
        "stderr: {}",
        stderr(&command_output)
    );
    assert_eq!(read_pcm16_wav(&output), (1, vec![2828, 0]));

    fs::remove_file(first).unwrap();
    fs::remove_file(second).unwrap();
    fs::remove_file(output).unwrap();
}

#[test]
fn run_mix_power_treats_mismatched_lengths_as_trailing_silence() {
    let first = temp_path("auralis-cli-run-mix-power-length-first", "wav");
    let second = temp_path("auralis-cli-run-mix-power-length-second", "wav");
    let output = temp_path("auralis-cli-run-mix-power-length-output", "wav");
    write_pcm16_wav(&first, 1, &[1000, -1000]);
    write_pcm16_wav(&second, 1, &[500, 0, -500]);

    let command_output = Command::new(env!("CARGO_BIN_EXE_auralis"))
        .args([
            "run",
            first.to_str().unwrap(),
            output.to_str().unwrap(),
            "--combine",
            "mix-power",
            "--input",
            second.to_str().unwrap(),
        ])
        .output()
        .unwrap();

    assert!(
        command_output.status.success(),
        "stderr: {}",
        stderr(&command_output)
    );
    assert_eq!(read_pcm16_wav(&output), (1, vec![1061, -707, -354]));

    fs::remove_file(first).unwrap();
    fs::remove_file(second).unwrap();
    fs::remove_file(output).unwrap();
}

#[test]
fn run_mix_power_combines_stereo_inputs_before_effects() {
    let first = temp_path("auralis-cli-run-mix-power-stereo-first", "wav");
    let second = temp_path("auralis-cli-run-mix-power-stereo-second", "wav");
    let output = temp_path("auralis-cli-run-mix-power-stereo-output", "wav");
    write_pcm16_wav(&first, 2, &[-1000, 1000, -2000, 2000]);
    write_pcm16_wav(&second, 2, &[3000, -1000, 1000, -3000]);

    let command_output = Command::new(env!("CARGO_BIN_EXE_auralis"))
        .args([
            "run",
            first.to_str().unwrap(),
            output.to_str().unwrap(),
            "--combine",
            "mix-power",
            "--input",
            second.to_str().unwrap(),
            "--reverse",
        ])
        .output()
        .unwrap();

    assert!(
        command_output.status.success(),
        "stderr: {}",
        stderr(&command_output)
    );
    assert_eq!(read_pcm16_wav(&output), (2, vec![-707, -707, 1414, 0]));

    fs::remove_file(first).unwrap();
    fs::remove_file(second).unwrap();
    fs::remove_file(output).unwrap();
}

#[test]
fn run_mix_power_backend_scalar_and_requested_simd_match() {
    let first = temp_path("auralis-cli-run-mix-power-backend-first", "wav");
    let second = temp_path("auralis-cli-run-mix-power-backend-second", "wav");
    let scalar_output = temp_path("auralis-cli-run-mix-power-backend-scalar-output", "wav");
    let simd_output = temp_path("auralis-cli-run-mix-power-backend-simd-output", "wav");
    write_pcm16_wav(&first, 1, &[-32768, -12345, 0, 12345, 32767]);
    write_pcm16_wav(&second, 1, &[32767, 12345, 0, -12345, -32768]);

    let scalar = Command::new(env!("CARGO_BIN_EXE_auralis"))
        .args([
            "run",
            first.to_str().unwrap(),
            scalar_output.to_str().unwrap(),
            "--backend",
            "scalar",
            "--combine",
            "mix-power",
            "--input",
            second.to_str().unwrap(),
        ])
        .output()
        .unwrap();
    let simd = Command::new(env!("CARGO_BIN_EXE_auralis"))
        .args([
            "run",
            first.to_str().unwrap(),
            simd_output.to_str().unwrap(),
            "--backend",
            "simd",
            "--combine",
            "mix-power",
            "--input",
            second.to_str().unwrap(),
        ])
        .output()
        .unwrap();

    assert!(scalar.status.success(), "stderr: {}", stderr(&scalar));
    assert!(simd.status.success(), "stderr: {}", stderr(&simd));
    assert_eq!(read_pcm16_wav(&simd_output), read_pcm16_wav(&scalar_output));

    fs::remove_file(first).unwrap();
    fs::remove_file(second).unwrap();
    fs::remove_file(scalar_output).unwrap();
    fs::remove_file(simd_output).unwrap();
}

#[test]
fn run_merge_turns_two_mono_inputs_into_stereo() {
    let first = temp_path("auralis-cli-run-merge-mono-first", "wav");
    let second = temp_path("auralis-cli-run-merge-mono-second", "wav");
    let output = temp_path("auralis-cli-run-merge-mono-output", "wav");
    write_pcm16_wav(&first, 1, &[1000, -1000]);
    write_pcm16_wav(&second, 1, &[3000, 1000]);

    let command_output = Command::new(env!("CARGO_BIN_EXE_auralis"))
        .args([
            "run",
            first.to_str().unwrap(),
            output.to_str().unwrap(),
            "--combine",
            "merge",
            "--input",
            second.to_str().unwrap(),
        ])
        .output()
        .unwrap();

    assert!(
        command_output.status.success(),
        "stderr: {}",
        stderr(&command_output)
    );
    assert_eq!(read_pcm16_wav(&output), (2, vec![1000, 3000, -1000, 1000]));

    fs::remove_file(first).unwrap();
    fs::remove_file(second).unwrap();
    fs::remove_file(output).unwrap();
}

#[test]
fn run_merge_treats_mismatched_lengths_as_trailing_silence() {
    let first = temp_path("auralis-cli-run-merge-length-first", "wav");
    let second = temp_path("auralis-cli-run-merge-length-second", "wav");
    let output = temp_path("auralis-cli-run-merge-length-output", "wav");
    write_pcm16_wav(&first, 1, &[1000, -1000]);
    write_pcm16_wav(&second, 1, &[500, 0, -500]);

    let command_output = Command::new(env!("CARGO_BIN_EXE_auralis"))
        .args([
            "run",
            first.to_str().unwrap(),
            output.to_str().unwrap(),
            "--combine",
            "merge",
            "--input",
            second.to_str().unwrap(),
        ])
        .output()
        .unwrap();

    assert!(
        command_output.status.success(),
        "stderr: {}",
        stderr(&command_output)
    );
    assert_eq!(
        read_pcm16_wav(&output),
        (2, vec![1000, 500, -1000, 0, 0, -500])
    );

    fs::remove_file(first).unwrap();
    fs::remove_file(second).unwrap();
    fs::remove_file(output).unwrap();
}

#[test]
fn run_merge_combines_multichannel_inputs_before_effects() {
    let first = temp_path("auralis-cli-run-merge-stereo-first", "wav");
    let second = temp_path("auralis-cli-run-merge-mono-second", "wav");
    let output = temp_path("auralis-cli-run-merge-stereo-output", "wav");
    write_pcm16_wav(&first, 2, &[-1000, 1000, -2000, 2000]);
    write_pcm16_wav(&second, 1, &[3000, -3000]);

    let command_output = Command::new(env!("CARGO_BIN_EXE_auralis"))
        .args([
            "run",
            first.to_str().unwrap(),
            output.to_str().unwrap(),
            "--combine",
            "merge",
            "--input",
            second.to_str().unwrap(),
            "--reverse",
        ])
        .output()
        .unwrap();

    assert!(
        command_output.status.success(),
        "stderr: {}",
        stderr(&command_output)
    );
    assert_eq!(
        read_pcm16_wav(&output),
        (3, vec![-2000, 2000, -3000, -1000, 1000, 3000])
    );

    fs::remove_file(first).unwrap();
    fs::remove_file(second).unwrap();
    fs::remove_file(output).unwrap();
}

#[test]
fn run_multiply_multiplies_equal_length_mono_inputs() {
    let first = temp_path("auralis-cli-run-multiply-mono-first", "wav");
    let second = temp_path("auralis-cli-run-multiply-mono-second", "wav");
    let output = temp_path("auralis-cli-run-multiply-mono-output", "wav");
    write_pcm16_wav(&first, 1, &[16_384, -16_384]);
    write_pcm16_wav(&second, 1, &[8192, 16_384]);

    let command_output = Command::new(env!("CARGO_BIN_EXE_auralis"))
        .args([
            "run",
            first.to_str().unwrap(),
            output.to_str().unwrap(),
            "--combine",
            "multiply",
            "--input",
            second.to_str().unwrap(),
        ])
        .output()
        .unwrap();

    assert!(
        command_output.status.success(),
        "stderr: {}",
        stderr(&command_output)
    );
    assert_eq!(read_pcm16_wav(&output), (1, vec![4096, -8192]));

    fs::remove_file(first).unwrap();
    fs::remove_file(second).unwrap();
    fs::remove_file(output).unwrap();
}

#[test]
fn run_multiply_treats_mismatched_lengths_as_silence() {
    let first = temp_path("auralis-cli-run-multiply-length-first", "wav");
    let second = temp_path("auralis-cli-run-multiply-length-second", "wav");
    let output = temp_path("auralis-cli-run-multiply-length-output", "wav");
    write_pcm16_wav(&first, 1, &[16_384, -16_384]);
    write_pcm16_wav(&second, 1, &[16_384, 0, 8192]);

    let command_output = Command::new(env!("CARGO_BIN_EXE_auralis"))
        .args([
            "run",
            first.to_str().unwrap(),
            output.to_str().unwrap(),
            "--combine",
            "multiply",
            "--input",
            second.to_str().unwrap(),
        ])
        .output()
        .unwrap();

    assert!(
        command_output.status.success(),
        "stderr: {}",
        stderr(&command_output)
    );
    assert_eq!(read_pcm16_wav(&output), (1, vec![8192, 0, 0]));

    fs::remove_file(first).unwrap();
    fs::remove_file(second).unwrap();
    fs::remove_file(output).unwrap();
}

#[test]
fn run_multiply_accepts_mismatched_channel_counts_with_silence() {
    let first = temp_path("auralis-cli-run-multiply-channel-first", "wav");
    let second = temp_path("auralis-cli-run-multiply-channel-second", "wav");
    let output = temp_path("auralis-cli-run-multiply-channel-output", "wav");
    write_pcm16_wav(&first, 1, &[16_384, -16_384]);
    write_pcm16_wav(&second, 2, &[16_384, 8192, -16_384, 8192]);

    let command_output = Command::new(env!("CARGO_BIN_EXE_auralis"))
        .args([
            "run",
            first.to_str().unwrap(),
            output.to_str().unwrap(),
            "--combine",
            "multiply",
            "--input",
            second.to_str().unwrap(),
        ])
        .output()
        .unwrap();

    assert!(
        command_output.status.success(),
        "stderr: {}",
        stderr(&command_output)
    );
    assert_eq!(read_pcm16_wav(&output), (2, vec![8192, 0, 8192, 0]));

    fs::remove_file(first).unwrap();
    fs::remove_file(second).unwrap();
    fs::remove_file(output).unwrap();
}

#[test]
fn run_multiply_combines_stereo_inputs_before_effects() {
    let first = temp_path("auralis-cli-run-multiply-stereo-first", "wav");
    let second = temp_path("auralis-cli-run-multiply-stereo-second", "wav");
    let output = temp_path("auralis-cli-run-multiply-stereo-output", "wav");
    write_pcm16_wav(&first, 2, &[-16_384, 16_384, -8192, 8192]);
    write_pcm16_wav(&second, 2, &[8192, -8192, 0, 16_384]);

    let command_output = Command::new(env!("CARGO_BIN_EXE_auralis"))
        .args([
            "run",
            first.to_str().unwrap(),
            output.to_str().unwrap(),
            "--combine",
            "multiply",
            "--input",
            second.to_str().unwrap(),
            "--reverse",
        ])
        .output()
        .unwrap();

    assert!(
        command_output.status.success(),
        "stderr: {}",
        stderr(&command_output)
    );
    assert_eq!(read_pcm16_wav(&output), (2, vec![0, 4096, -4096, -4096]));

    fs::remove_file(first).unwrap();
    fs::remove_file(second).unwrap();
    fs::remove_file(output).unwrap();
}

#[test]
fn run_multiply_backend_scalar_and_requested_simd_match() {
    let first = temp_path("auralis-cli-run-multiply-backend-first", "wav");
    let second = temp_path("auralis-cli-run-multiply-backend-second", "wav");
    let scalar_output = temp_path("auralis-cli-run-multiply-backend-scalar-output", "wav");
    let simd_output = temp_path("auralis-cli-run-multiply-backend-simd-output", "wav");
    write_pcm16_wav(&first, 1, &[-32768, -12345, 0, 12345, 32767]);
    write_pcm16_wav(&second, 1, &[32767, 12345, 0, -12345, -32768]);

    let scalar = Command::new(env!("CARGO_BIN_EXE_auralis"))
        .args([
            "run",
            first.to_str().unwrap(),
            scalar_output.to_str().unwrap(),
            "--backend",
            "scalar",
            "--combine",
            "multiply",
            "--input",
            second.to_str().unwrap(),
        ])
        .output()
        .unwrap();
    let simd = Command::new(env!("CARGO_BIN_EXE_auralis"))
        .args([
            "run",
            first.to_str().unwrap(),
            simd_output.to_str().unwrap(),
            "--backend",
            "simd",
            "--combine",
            "multiply",
            "--input",
            second.to_str().unwrap(),
        ])
        .output()
        .unwrap();

    assert!(scalar.status.success(), "stderr: {}", stderr(&scalar));
    assert!(simd.status.success(), "stderr: {}", stderr(&simd));
    assert_eq!(read_pcm16_wav(&simd_output), read_pcm16_wav(&scalar_output));

    fs::remove_file(first).unwrap();
    fs::remove_file(second).unwrap();
    fs::remove_file(scalar_output).unwrap();
    fs::remove_file(simd_output).unwrap();
}

#[test]
fn run_gain_output_matches_library_pipeline() {
    let input = temp_path("auralis-cli-run-gain-input", "wav");
    let cli_output = temp_path("auralis-cli-run-gain-cli-output", "wav");
    let library_output = temp_path("auralis-cli-run-gain-library-output", "wav");
    write_pcm16_wav(&input, 1, &[-16_384, -8_192, 0, 8_192, 16_384]);

    AudioFile::open_wav(&input)
        .unwrap()
        .into_pipeline()
        .gain_db(-6.0)
        .write_wav(&library_output)
        .unwrap();

    let command_output = Command::new(env!("CARGO_BIN_EXE_auralis"))
        .args([
            "run",
            input.to_str().unwrap(),
            cli_output.to_str().unwrap(),
            "--gain-db",
            "-6",
        ])
        .output()
        .unwrap();

    assert!(
        command_output.status.success(),
        "stderr: {}",
        stderr(&command_output)
    );
    assert_eq!(read_pcm16_wav(&cli_output), read_pcm16_wav(&library_output));

    fs::remove_file(input).unwrap();
    fs::remove_file(cli_output).unwrap();
    fs::remove_file(library_output).unwrap();
}

#[test]
fn run_gain_output_matches_under_forced_scalar_and_requested_simd() {
    let input = temp_path("auralis-cli-run-gain-backend-input", "wav");
    let scalar_output = temp_path("auralis-cli-run-gain-scalar-output", "wav");
    let simd_output = temp_path("auralis-cli-run-gain-simd-output", "wav");
    write_pcm16_wav(
        &input,
        1,
        &[
            i16::MIN,
            -32_767,
            -16_384,
            -1,
            0,
            1,
            16_384,
            32_766,
            i16::MAX,
        ],
    );

    let scalar_command_output = Command::new(env!("CARGO_BIN_EXE_auralis"))
        .args([
            "run",
            input.to_str().unwrap(),
            scalar_output.to_str().unwrap(),
            "--backend",
            "scalar",
            "--gain-db",
            "-3",
        ])
        .output()
        .unwrap();
    let simd_command_output = Command::new(env!("CARGO_BIN_EXE_auralis"))
        .args([
            "run",
            input.to_str().unwrap(),
            simd_output.to_str().unwrap(),
            "--backend",
            "simd",
            "--gain-db",
            "-3",
        ])
        .output()
        .unwrap();

    assert!(
        scalar_command_output.status.success(),
        "stderr: {}",
        stderr(&scalar_command_output)
    );
    assert!(
        simd_command_output.status.success(),
        "stderr: {}",
        stderr(&simd_command_output)
    );
    assert_eq!(read_pcm16_wav(&simd_output), read_pcm16_wav(&scalar_output));

    fs::remove_file(input).unwrap();
    fs::remove_file(scalar_output).unwrap();
    fs::remove_file(simd_output).unwrap();
}

#[test]
fn run_dc_shift_output_matches_library_pipeline_and_clips_at_wav_boundary() {
    let input = temp_path("auralis-cli-run-dc-shift-input", "wav");
    let cli_output = temp_path("auralis-cli-run-dc-shift-cli-output", "wav");
    let library_output = temp_path("auralis-cli-run-dc-shift-library-output", "wav");
    write_pcm16_wav(&input, 2, &[-32768, 0, 8192, 30_000]);

    AudioFile::open_wav(&input)
        .unwrap()
        .into_pipeline()
        .dc_shift(0.25)
        .write_wav(&library_output)
        .unwrap();

    let command_output = Command::new(env!("CARGO_BIN_EXE_auralis"))
        .args([
            "run",
            input.to_str().unwrap(),
            cli_output.to_str().unwrap(),
            "--dc-shift",
            "0.25",
        ])
        .output()
        .unwrap();

    assert!(
        command_output.status.success(),
        "stderr: {}",
        stderr(&command_output)
    );
    assert_eq!(read_pcm16_wav(&cli_output), read_pcm16_wav(&library_output));
    assert_eq!(
        read_pcm16_wav(&cli_output),
        (2, vec![-24576, 8192, 16384, 32767])
    );

    fs::remove_file(input).unwrap();
    fs::remove_file(cli_output).unwrap();
    fs::remove_file(library_output).unwrap();
}

#[test]
fn run_dc_shift_output_matches_under_forced_scalar_and_requested_simd() {
    let input = temp_path("auralis-cli-run-dc-shift-backend-input", "wav");
    let scalar_output = temp_path("auralis-cli-run-dc-shift-scalar-output", "wav");
    let simd_output = temp_path("auralis-cli-run-dc-shift-simd-output", "wav");
    write_pcm16_wav(
        &input,
        1,
        &[
            i16::MIN,
            -32_767,
            -16_384,
            -1,
            0,
            1,
            16_384,
            32_766,
            i16::MAX,
        ],
    );

    let scalar_command_output = Command::new(env!("CARGO_BIN_EXE_auralis"))
        .args([
            "run",
            input.to_str().unwrap(),
            scalar_output.to_str().unwrap(),
            "--backend",
            "scalar",
            "--dc-shift",
            "0.125",
        ])
        .output()
        .unwrap();
    let simd_command_output = Command::new(env!("CARGO_BIN_EXE_auralis"))
        .args([
            "run",
            input.to_str().unwrap(),
            simd_output.to_str().unwrap(),
            "--backend",
            "simd",
            "--dc-shift",
            "0.125",
        ])
        .output()
        .unwrap();

    assert!(
        scalar_command_output.status.success(),
        "stderr: {}",
        stderr(&scalar_command_output)
    );
    assert!(
        simd_command_output.status.success(),
        "stderr: {}",
        stderr(&simd_command_output)
    );
    assert_eq!(read_pcm16_wav(&simd_output), read_pcm16_wav(&scalar_output));

    fs::remove_file(input).unwrap();
    fs::remove_file(scalar_output).unwrap();
    fs::remove_file(simd_output).unwrap();
}

#[test]
fn run_trim_frames_output_matches_library_pipeline() {
    let input = temp_path("auralis-cli-run-trim-frame-input", "wav");
    let cli_output = temp_path("auralis-cli-run-trim-frame-cli-output", "wav");
    let library_output = temp_path("auralis-cli-run-trim-frame-library-output", "wav");
    write_pcm16_wav(
        &input,
        2,
        &[-1000, 1000, -2000, 2000, -3000, 3000, -4000, 4000],
    );

    AudioFile::open_wav(&input)
        .unwrap()
        .into_pipeline()
        .trim_frames(1, 3)
        .write_wav(&library_output)
        .unwrap();

    let command_output = Command::new(env!("CARGO_BIN_EXE_auralis"))
        .args([
            "run",
            input.to_str().unwrap(),
            cli_output.to_str().unwrap(),
            "--trim-start-frame",
            "1",
            "--trim-end-frame",
            "3",
        ])
        .output()
        .unwrap();

    assert!(
        command_output.status.success(),
        "stderr: {}",
        stderr(&command_output)
    );
    assert_eq!(read_pcm16_wav(&cli_output), read_pcm16_wav(&library_output));
    assert_eq!(
        read_pcm16_wav(&cli_output),
        (2, vec![-2000, 2000, -3000, 3000])
    );

    fs::remove_file(input).unwrap();
    fs::remove_file(cli_output).unwrap();
    fs::remove_file(library_output).unwrap();
}

#[test]
fn run_trim_seconds_output_matches_library_pipeline() {
    let input = temp_path("auralis-cli-run-trim-seconds-input", "wav");
    let cli_output = temp_path("auralis-cli-run-trim-seconds-cli-output", "wav");
    let library_output = temp_path("auralis-cli-run-trim-seconds-library-output", "wav");
    write_pcm16_wav(&input, 1, &[-1000, -500, 0, 500, 1000]);

    AudioFile::open_wav(&input)
        .unwrap()
        .into_pipeline()
        .trim_seconds(1.0 / 48_000.0, 4.0 / 48_000.0)
        .write_wav(&library_output)
        .unwrap();

    let command_output = Command::new(env!("CARGO_BIN_EXE_auralis"))
        .args([
            "run",
            input.to_str().unwrap(),
            cli_output.to_str().unwrap(),
            "--trim-start-seconds",
            "0.000020833333333333333",
            "--trim-end-seconds",
            "0.000083333333333333333",
        ])
        .output()
        .unwrap();

    assert!(
        command_output.status.success(),
        "stderr: {}",
        stderr(&command_output)
    );
    assert_eq!(read_pcm16_wav(&cli_output), read_pcm16_wav(&library_output));
    assert_eq!(read_pcm16_wav(&cli_output), (1, vec![-500, 0, 500]));

    fs::remove_file(input).unwrap();
    fs::remove_file(cli_output).unwrap();
    fs::remove_file(library_output).unwrap();
}

#[test]
fn run_pad_frames_output_matches_library_pipeline() {
    let input = temp_path("auralis-cli-run-pad-frame-input", "wav");
    let cli_output = temp_path("auralis-cli-run-pad-frame-cli-output", "wav");
    let library_output = temp_path("auralis-cli-run-pad-frame-library-output", "wav");
    write_pcm16_wav(&input, 2, &[-1000, 1000, -2000, 2000]);

    AudioFile::open_wav(&input)
        .unwrap()
        .into_pipeline()
        .pad_frames(1, 2)
        .write_wav(&library_output)
        .unwrap();

    let command_output = Command::new(env!("CARGO_BIN_EXE_auralis"))
        .args([
            "run",
            input.to_str().unwrap(),
            cli_output.to_str().unwrap(),
            "--pad-start-frame",
            "1",
            "--pad-end-frame",
            "2",
        ])
        .output()
        .unwrap();

    assert!(
        command_output.status.success(),
        "stderr: {}",
        stderr(&command_output)
    );
    assert_eq!(read_pcm16_wav(&cli_output), read_pcm16_wav(&library_output));
    assert_eq!(
        read_pcm16_wav(&cli_output),
        (2, vec![0, 0, -1000, 1000, -2000, 2000, 0, 0, 0, 0])
    );

    fs::remove_file(input).unwrap();
    fs::remove_file(cli_output).unwrap();
    fs::remove_file(library_output).unwrap();
}

#[test]
fn run_single_sided_pad_defaults_other_side_to_zero() {
    let input = temp_path("auralis-cli-run-pad-single-input", "wav");
    let output = temp_path("auralis-cli-run-pad-single-output", "wav");
    write_pcm16_wav(&input, 1, &[1000, -1000]);

    let command_output = Command::new(env!("CARGO_BIN_EXE_auralis"))
        .args([
            "run",
            input.to_str().unwrap(),
            output.to_str().unwrap(),
            "--pad-end-frame",
            "2",
        ])
        .output()
        .unwrap();

    assert!(
        command_output.status.success(),
        "stderr: {}",
        stderr(&command_output)
    );
    assert_eq!(read_pcm16_wav(&output), (1, vec![1000, -1000, 0, 0]));

    fs::remove_file(input).unwrap();
    fs::remove_file(output).unwrap();
}

#[test]
fn run_reverse_output_matches_library_pipeline() {
    let input = temp_path("auralis-cli-run-reverse-input", "wav");
    let cli_output = temp_path("auralis-cli-run-reverse-cli-output", "wav");
    let library_output = temp_path("auralis-cli-run-reverse-library-output", "wav");
    write_pcm16_wav(&input, 2, &[-1000, 1000, -2000, 2000, -3000, 3000]);

    AudioFile::open_wav(&input)
        .unwrap()
        .into_pipeline()
        .reverse()
        .write_wav(&library_output)
        .unwrap();

    let command_output = Command::new(env!("CARGO_BIN_EXE_auralis"))
        .args([
            "run",
            input.to_str().unwrap(),
            cli_output.to_str().unwrap(),
            "--reverse",
        ])
        .output()
        .unwrap();

    assert!(
        command_output.status.success(),
        "stderr: {}",
        stderr(&command_output)
    );
    assert_eq!(read_pcm16_wav(&cli_output), read_pcm16_wav(&library_output));
    assert_eq!(
        read_pcm16_wav(&cli_output),
        (2, vec![-3000, 3000, -2000, 2000, -1000, 1000])
    );

    fs::remove_file(input).unwrap();
    fs::remove_file(cli_output).unwrap();
    fs::remove_file(library_output).unwrap();
}

#[test]
fn run_fade_frames_output_matches_library_pipeline() {
    let input = temp_path("auralis-cli-run-fade-frame-input", "wav");
    let cli_output = temp_path("auralis-cli-run-fade-frame-cli-output", "wav");
    let library_output = temp_path("auralis-cli-run-fade-frame-library-output", "wav");
    write_pcm16_wav(
        &input,
        2,
        &[-10000, 10000, -20000, 20000, -30000, 30000, -4000, 4000],
    );

    AudioFile::open_wav(&input)
        .unwrap()
        .into_pipeline()
        .fade_frames(2, 2)
        .write_wav(&library_output)
        .unwrap();

    let command_output = Command::new(env!("CARGO_BIN_EXE_auralis"))
        .args([
            "run",
            input.to_str().unwrap(),
            cli_output.to_str().unwrap(),
            "--fade-in-frame",
            "2",
            "--fade-out-frame",
            "2",
        ])
        .output()
        .unwrap();

    assert!(
        command_output.status.success(),
        "stderr: {}",
        stderr(&command_output)
    );
    assert_eq!(read_pcm16_wav(&cli_output), read_pcm16_wav(&library_output));
    assert_eq!(
        read_pcm16_wav(&cli_output),
        (2, vec![0, 0, -10000, 10000, -15000, 15000, 0, 0])
    );

    fs::remove_file(input).unwrap();
    fs::remove_file(cli_output).unwrap();
    fs::remove_file(library_output).unwrap();
}

#[test]
fn run_fade_frames_output_matches_under_forced_scalar_and_requested_simd() {
    let input = temp_path("auralis-cli-run-fade-frame-backend-input", "wav");
    let scalar_output = temp_path("auralis-cli-run-fade-frame-scalar-output", "wav");
    let simd_output = temp_path("auralis-cli-run-fade-frame-simd-output", "wav");
    write_pcm16_wav(
        &input,
        2,
        &[
            -32768, 32767, -32767, 32766, -16384, 16384, -1, 1, 0, 0, 1, -1, 16384, -16384, 32766,
            -32767,
        ],
    );

    let scalar_command_output = Command::new(env!("CARGO_BIN_EXE_auralis"))
        .args([
            "run",
            input.to_str().unwrap(),
            scalar_output.to_str().unwrap(),
            "--backend",
            "scalar",
            "--fade-in-frame",
            "5",
            "--fade-out-frame",
            "7",
        ])
        .output()
        .unwrap();
    let simd_command_output = Command::new(env!("CARGO_BIN_EXE_auralis"))
        .args([
            "run",
            input.to_str().unwrap(),
            simd_output.to_str().unwrap(),
            "--backend",
            "simd",
            "--fade-in-frame",
            "5",
            "--fade-out-frame",
            "7",
        ])
        .output()
        .unwrap();

    assert!(
        scalar_command_output.status.success(),
        "stderr: {}",
        stderr(&scalar_command_output)
    );
    assert!(
        simd_command_output.status.success(),
        "stderr: {}",
        stderr(&simd_command_output)
    );
    assert_eq!(read_pcm16_wav(&simd_output), read_pcm16_wav(&scalar_output));

    fs::remove_file(input).unwrap();
    fs::remove_file(scalar_output).unwrap();
    fs::remove_file(simd_output).unwrap();
}

#[test]
fn run_positional_effect_chain_output_matches_in_memory_chain() {
    let input = temp_path("auralis-cli-run-chain-input", "wav");
    let cli_output = temp_path("auralis-cli-run-chain-cli-output", "wav");
    let library_output = temp_path("auralis-cli-run-chain-library-output", "wav");
    write_pcm16_wav(&input, 2, &[-16_384, 16_384, -8_192, 8_192, 0, 4096]);
    let chain_tokens = ["gain", "-6", "dcshift", "0.125", "reverse"];
    let chain = auralis::parse_effect_chain(&chain_tokens).unwrap();

    AudioFile::open_wav(&input)
        .unwrap()
        .into_pipeline()
        .apply_effect_chain(&chain)
        .write_wav(&library_output)
        .unwrap();

    let command_output = Command::new(env!("CARGO_BIN_EXE_auralis"))
        .args([
            "run",
            input.to_str().unwrap(),
            cli_output.to_str().unwrap(),
            "gain",
            "-6",
            "dcshift",
            "0.125",
            "reverse",
        ])
        .output()
        .unwrap();

    assert!(
        command_output.status.success(),
        "stderr: {}",
        stderr(&command_output)
    );
    assert_eq!(read_pcm16_wav(&cli_output), read_pcm16_wav(&library_output));

    fs::remove_file(input).unwrap();
    fs::remove_file(cli_output).unwrap();
    fs::remove_file(library_output).unwrap();
}

#[test]
fn run_effects_file_output_matches_equivalent_positional_chain() {
    let input = temp_path("auralis-cli-run-effects-file-input", "wav");
    let effects_file = temp_path("auralis-cli-run-effects-file", "effects");
    let positional_output = temp_path("auralis-cli-run-effects-file-positional-output", "wav");
    let file_output = temp_path("auralis-cli-run-effects-file-output", "wav");
    write_pcm16_wav(&input, 2, &[-16_384, 16_384, -8_192, 8_192, 0, 4096]);
    fs::write(
        &effects_file,
        "# level then edit\n\
         gain -6\n\
         dcshift 0.125\n\
         reverse\n",
    )
    .unwrap();

    let positional_command_output = Command::new(env!("CARGO_BIN_EXE_auralis"))
        .args([
            "run",
            input.to_str().unwrap(),
            positional_output.to_str().unwrap(),
            "gain",
            "-6",
            "dcshift",
            "0.125",
            "reverse",
        ])
        .output()
        .unwrap();
    let file_command_output = Command::new(env!("CARGO_BIN_EXE_auralis"))
        .args([
            "run",
            input.to_str().unwrap(),
            file_output.to_str().unwrap(),
            "--effects-file",
            effects_file.to_str().unwrap(),
        ])
        .output()
        .unwrap();

    assert!(
        positional_command_output.status.success(),
        "stderr: {}",
        stderr(&positional_command_output)
    );
    assert!(
        file_command_output.status.success(),
        "stderr: {}",
        stderr(&file_command_output)
    );
    assert_eq!(
        read_pcm16_wav(&file_output),
        read_pcm16_wav(&positional_output)
    );

    fs::remove_file(input).unwrap();
    fs::remove_file(effects_file).unwrap();
    fs::remove_file(positional_output).unwrap();
    fs::remove_file(file_output).unwrap();
}

#[test]
fn run_positional_effect_chain_preserves_user_order() {
    let input = temp_path("auralis-cli-run-chain-order-input", "wav");
    let gain_then_shift = temp_path("auralis-cli-run-chain-gain-shift-output", "wav");
    let shift_then_gain = temp_path("auralis-cli-run-chain-shift-gain-output", "wav");
    write_pcm16_wav(&input, 1, &[4096, 8192, 12_288]);

    let gain_then_shift_output = Command::new(env!("CARGO_BIN_EXE_auralis"))
        .args([
            "run",
            input.to_str().unwrap(),
            gain_then_shift.to_str().unwrap(),
            "gain",
            "-6",
            "dcshift",
            "0.125",
        ])
        .output()
        .unwrap();
    let shift_then_gain_output = Command::new(env!("CARGO_BIN_EXE_auralis"))
        .args([
            "run",
            input.to_str().unwrap(),
            shift_then_gain.to_str().unwrap(),
            "dcshift",
            "0.125",
            "gain",
            "-6",
        ])
        .output()
        .unwrap();

    assert!(
        gain_then_shift_output.status.success(),
        "stderr: {}",
        stderr(&gain_then_shift_output)
    );
    assert!(
        shift_then_gain_output.status.success(),
        "stderr: {}",
        stderr(&shift_then_gain_output)
    );
    assert_ne!(
        read_pcm16_wav(&gain_then_shift),
        read_pcm16_wav(&shift_then_gain)
    );

    fs::remove_file(input).unwrap();
    fs::remove_file(gain_then_shift).unwrap();
    fs::remove_file(shift_then_gain).unwrap();
}

#[test]
fn run_accepts_boundary_separator_in_positional_chain_and_effects_file() {
    let input = temp_path("auralis-cli-run-chain-boundary-input", "wav");
    let flat_output = temp_path("auralis-cli-run-chain-boundary-flat-output", "wav");
    let boundary_output = temp_path("auralis-cli-run-chain-boundary-output", "wav");
    let file_output = temp_path("auralis-cli-run-chain-boundary-file-output", "wav");
    let effects_file = temp_path("auralis-cli-run-chain-boundary", "effects");
    write_pcm16_wav(&input, 1, &[4096, -8192, 12_288, -16_384]);
    fs::write(&effects_file, "gain -6 :\ndcshift 0.125 reverse\n").unwrap();

    let flat_command_output = Command::new(env!("CARGO_BIN_EXE_auralis"))
        .args([
            "run",
            input.to_str().unwrap(),
            flat_output.to_str().unwrap(),
            "gain",
            "-6",
            "dcshift",
            "0.125",
            "reverse",
        ])
        .output()
        .unwrap();
    let boundary_command_output = Command::new(env!("CARGO_BIN_EXE_auralis"))
        .args([
            "run",
            input.to_str().unwrap(),
            boundary_output.to_str().unwrap(),
            "gain",
            "-6",
            ":",
            "dcshift",
            "0.125",
            "reverse",
        ])
        .output()
        .unwrap();
    let file_command_output = Command::new(env!("CARGO_BIN_EXE_auralis"))
        .args([
            "run",
            input.to_str().unwrap(),
            file_output.to_str().unwrap(),
            "--effects-file",
            effects_file.to_str().unwrap(),
        ])
        .output()
        .unwrap();

    assert!(
        flat_command_output.status.success(),
        "stderr: {}",
        stderr(&flat_command_output)
    );
    assert!(
        boundary_command_output.status.success(),
        "stderr: {}",
        stderr(&boundary_command_output)
    );
    assert!(
        file_command_output.status.success(),
        "stderr: {}",
        stderr(&file_command_output)
    );
    assert_eq!(
        read_pcm16_wav(&boundary_output),
        read_pcm16_wav(&flat_output)
    );
    assert_eq!(read_pcm16_wav(&file_output), read_pcm16_wav(&flat_output));

    fs::remove_file(input).unwrap();
    fs::remove_file(flat_output).unwrap();
    fs::remove_file(boundary_output).unwrap();
    fs::remove_file(file_output).unwrap();
    fs::remove_file(effects_file).unwrap();
}

#[test]
fn run_boundary_control_returns_clear_error() {
    let input = temp_path("auralis-cli-run-chain-boundary-control-input", "wav");
    let output = temp_path("auralis-cli-run-chain-boundary-control-output", "wav");
    write_pcm16_wav(&input, 1, &[0, 1, 2]);

    let command_output = Command::new(env!("CARGO_BIN_EXE_auralis"))
        .args([
            "run",
            input.to_str().unwrap(),
            output.to_str().unwrap(),
            "gain",
            "-3",
            ":",
            "newfile",
        ])
        .output()
        .unwrap();

    fs::remove_file(input).unwrap();
    let _ = fs::remove_file(output);
    assert!(!command_output.status.success());
    let stderr = stderr(&command_output);
    assert!(stderr.contains("unsupported boundary control"), "{stderr}");
    assert!(
        stderr.contains("`newfile` and `restart` semantics are not implemented"),
        "{stderr}"
    );
}

#[test]
fn run_missing_effects_file_returns_clear_error() {
    let input = temp_path("auralis-cli-run-missing-effects-file-input", "wav");
    let effects_file = temp_path("auralis-cli-run-missing-effects-file", "effects");
    let output = temp_path("auralis-cli-run-missing-effects-file-output", "wav");
    write_pcm16_wav(&input, 1, &[0, 1, 2]);

    let command_output = Command::new(env!("CARGO_BIN_EXE_auralis"))
        .args([
            "run",
            input.to_str().unwrap(),
            output.to_str().unwrap(),
            "--effects-file",
            effects_file.to_str().unwrap(),
        ])
        .output()
        .unwrap();

    fs::remove_file(input).unwrap();
    let _ = fs::remove_file(output);
    assert!(!command_output.status.success());
    let stderr = stderr(&command_output);
    assert!(stderr.contains("failed to read effects file"), "{stderr}");
    assert!(stderr.contains(effects_file.to_str().unwrap()), "{stderr}");
}

#[test]
fn run_unreadable_effects_file_returns_clear_error() {
    let input = temp_path("auralis-cli-run-unreadable-effects-file-input", "wav");
    let effects_dir = temp_path("auralis-cli-run-unreadable-effects-file", "effects");
    let output = temp_path("auralis-cli-run-unreadable-effects-file-output", "wav");
    write_pcm16_wav(&input, 1, &[0, 1, 2]);
    fs::create_dir(&effects_dir).unwrap();

    let command_output = Command::new(env!("CARGO_BIN_EXE_auralis"))
        .args([
            "run",
            input.to_str().unwrap(),
            output.to_str().unwrap(),
            "--effects-file",
            effects_dir.to_str().unwrap(),
        ])
        .output()
        .unwrap();

    fs::remove_file(input).unwrap();
    fs::remove_dir(effects_dir).unwrap();
    let _ = fs::remove_file(output);
    assert!(!command_output.status.success());
    let stderr = stderr(&command_output);
    assert!(stderr.contains("failed to read effects file"), "{stderr}");
}

#[test]
fn run_rejects_effects_file_with_positional_chain() {
    let input = temp_path("auralis-cli-run-effects-file-mixed-chain-input", "wav");
    let effects_file = temp_path("auralis-cli-run-effects-file-mixed-chain", "effects");
    let output = temp_path("auralis-cli-run-effects-file-mixed-chain-output", "wav");
    write_pcm16_wav(&input, 1, &[0, 1, 2]);
    fs::write(&effects_file, "gain -3\n").unwrap();

    let command_output = Command::new(env!("CARGO_BIN_EXE_auralis"))
        .args([
            "run",
            input.to_str().unwrap(),
            output.to_str().unwrap(),
            "--effects-file",
            effects_file.to_str().unwrap(),
            "reverse",
        ])
        .output()
        .unwrap();

    fs::remove_file(input).unwrap();
    fs::remove_file(effects_file).unwrap();
    let _ = fs::remove_file(output);
    assert!(!command_output.status.success());
    let stderr = stderr(&command_output);
    assert!(
        stderr.contains("effects files cannot be combined with positional effect chain tokens"),
        "{stderr}"
    );
}

#[test]
fn run_rejects_effects_file_with_legacy_effect_flags() {
    let input = temp_path("auralis-cli-run-effects-file-mixed-legacy-input", "wav");
    let effects_file = temp_path("auralis-cli-run-effects-file-mixed-legacy", "effects");
    let output = temp_path("auralis-cli-run-effects-file-mixed-legacy-output", "wav");
    write_pcm16_wav(&input, 1, &[0, 1, 2]);
    fs::write(&effects_file, "reverse\n").unwrap();

    let command_output = Command::new(env!("CARGO_BIN_EXE_auralis"))
        .args([
            "run",
            input.to_str().unwrap(),
            output.to_str().unwrap(),
            "--effects-file",
            effects_file.to_str().unwrap(),
            "--gain-db",
            "-3",
        ])
        .output()
        .unwrap();

    fs::remove_file(input).unwrap();
    fs::remove_file(effects_file).unwrap();
    let _ = fs::remove_file(output);
    assert!(!command_output.status.success());
    let stderr = stderr(&command_output);
    assert!(
        stderr.contains("effects files cannot be combined with legacy effect flags"),
        "{stderr}"
    );
}

#[test]
fn run_invalid_positional_chain_reports_failing_effect_and_argument() {
    let input = temp_path("auralis-cli-run-invalid-chain-input", "wav");
    let output = temp_path("auralis-cli-run-invalid-chain-output", "wav");
    write_pcm16_wav(&input, 1, &[0, 1, 2]);

    let command_output = Command::new(env!("CARGO_BIN_EXE_auralis"))
        .args([
            "run",
            input.to_str().unwrap(),
            output.to_str().unwrap(),
            "gain",
            "-3",
            "trim",
            "1",
            "reverse",
        ])
        .output()
        .unwrap();

    fs::remove_file(input).unwrap();
    let _ = fs::remove_file(output);
    assert!(!command_output.status.success());
    let stderr = stderr(&command_output);
    assert!(
        stderr.contains("effect chain command 1 (`trim 1`) failed to parse"),
        "{stderr}"
    );
    assert!(
        stderr.contains("effect `trim` requires argument `end-frame`"),
        "{stderr}"
    );
}

#[test]
fn run_rejects_mixed_positional_chain_and_legacy_effect_flags() {
    let input = temp_path("auralis-cli-run-mixed-chain-input", "wav");
    let output = temp_path("auralis-cli-run-mixed-chain-output", "wav");
    write_pcm16_wav(&input, 1, &[0, 1, 2]);

    let command_output = Command::new(env!("CARGO_BIN_EXE_auralis"))
        .args([
            "run",
            input.to_str().unwrap(),
            output.to_str().unwrap(),
            "--gain-db",
            "-3",
            "reverse",
        ])
        .output()
        .unwrap();

    fs::remove_file(input).unwrap();
    let _ = fs::remove_file(output);
    assert!(!command_output.status.success());
    let stderr = stderr(&command_output);
    assert!(
        stderr.contains("positional effect chains cannot be combined with legacy effect flags"),
        "{stderr}"
    );
}

#[test]
fn run_invalid_gain_argument_returns_clear_error() {
    let input = temp_path("auralis-cli-run-invalid-gain-input", "wav");
    let output = temp_path("auralis-cli-run-invalid-gain-output", "wav");
    write_pcm16_wav(&input, 1, &[0]);

    let command_output = Command::new(env!("CARGO_BIN_EXE_auralis"))
        .args([
            "run",
            input.to_str().unwrap(),
            output.to_str().unwrap(),
            "--gain-db",
            "NaN",
        ])
        .output()
        .unwrap();

    fs::remove_file(input).unwrap();
    let _ = fs::remove_file(output);
    assert!(!command_output.status.success());
    let stderr = stderr(&command_output);
    assert!(
        stderr.contains("error: decibels must be finite"),
        "{stderr}"
    );
}

#[test]
fn run_invalid_dc_shift_argument_returns_clear_error() {
    let input = temp_path("auralis-cli-run-invalid-dc-shift-input", "wav");
    let output = temp_path("auralis-cli-run-invalid-dc-shift-output", "wav");
    write_pcm16_wav(&input, 1, &[0]);

    let command_output = Command::new(env!("CARGO_BIN_EXE_auralis"))
        .args([
            "run",
            input.to_str().unwrap(),
            output.to_str().unwrap(),
            "--dc-shift",
            "2.1",
        ])
        .output()
        .unwrap();

    fs::remove_file(input).unwrap();
    let _ = fs::remove_file(output);
    assert!(!command_output.status.success());
    let stderr = stderr(&command_output);
    assert!(
        stderr.contains("error: dc shift must be finite and in the range -2.0..=2.0"),
        "{stderr}"
    );
}

#[test]
fn run_invalid_trim_range_returns_clear_error() {
    let input = temp_path("auralis-cli-run-invalid-trim-input", "wav");
    let output = temp_path("auralis-cli-run-invalid-trim-output", "wav");
    write_pcm16_wav(&input, 1, &[0, 1, 2]);

    let command_output = Command::new(env!("CARGO_BIN_EXE_auralis"))
        .args([
            "run",
            input.to_str().unwrap(),
            output.to_str().unwrap(),
            "--trim-start-frame",
            "3",
            "--trim-end-frame",
            "1",
        ])
        .output()
        .unwrap();

    fs::remove_file(input).unwrap();
    let _ = fs::remove_file(output);
    assert!(!command_output.status.success());
    let stderr = stderr(&command_output);
    assert!(
        stderr.contains("error: trim start frame must be less than or equal"),
        "{stderr}"
    );
}

#[test]
fn run_incomplete_trim_range_returns_clear_error() {
    let input = temp_path("auralis-cli-run-incomplete-trim-input", "wav");
    let output = temp_path("auralis-cli-run-incomplete-trim-output", "wav");
    write_pcm16_wav(&input, 1, &[0, 1, 2]);

    let command_output = Command::new(env!("CARGO_BIN_EXE_auralis"))
        .args([
            "run",
            input.to_str().unwrap(),
            output.to_str().unwrap(),
            "--trim-start-frame",
            "1",
        ])
        .output()
        .unwrap();

    fs::remove_file(input).unwrap();
    let _ = fs::remove_file(output);
    assert!(!command_output.status.success());
    let stderr = stderr(&command_output);
    assert!(
        stderr.contains("error: frame trim requires both --trim-start-frame and --trim-end-frame"),
        "{stderr}"
    );
}

#[test]
fn run_help_documents_gain_and_trim_units() {
    let command_output = Command::new(env!("CARGO_BIN_EXE_auralis"))
        .args(["run", "--help"])
        .output()
        .unwrap();

    assert!(command_output.status.success());
    let stdout = stdout(&command_output);
    assert!(stdout.contains("--gain-db <DB>"), "{stdout}");
    assert!(
        stdout.contains("Constant gain to apply, in decibels"),
        "{stdout}"
    );
    assert!(stdout.contains("--backend <BACKEND>"), "{stdout}");
    assert!(
        stdout.contains("Sample-processing backend to request"),
        "{stdout}"
    );
    assert!(stdout.contains("--combine <METHOD>"), "{stdout}");
    assert!(
        stdout.contains("Input-combiner method to apply before effects"),
        "{stdout}"
    );
    assert!(stdout.contains("--input <FILE>"), "{stdout}");
    assert!(
        stdout.contains("Additional PCM16 WAV input files to combine"),
        "{stdout}"
    );
    assert!(stdout.contains("--channels <CHANNELS>"), "{stdout}");
    assert!(
        stdout.contains("Output channel count; inserts SoX-ng-style channel conversion"),
        "{stdout}"
    );
    assert!(stdout.contains("--no-auto-channels"), "{stdout}");
    assert!(
        stdout.contains("Fail instead of automatically converting channels"),
        "{stdout}"
    );
    assert!(stdout.contains("--rate <RATE>"), "{stdout}");
    assert!(
        stdout.contains("Output sample rate; inserts deterministic rate conversion"),
        "{stdout}"
    );
    assert!(stdout.contains("--no-auto-rate"), "{stdout}");
    assert!(
        stdout.contains("Fail instead of automatically converting sample rate"),
        "{stdout}"
    );
    assert!(stdout.contains("--dc-shift <SHIFT>"), "{stdout}");
    assert!(
        stdout.contains("Constant normalized DC offset to add"),
        "{stdout}"
    );
    assert!(stdout.contains("--trim-start-frame <FRAME>"), "{stdout}");
    assert!(stdout.contains("--trim-end-frame <FRAME>"), "{stdout}");
    assert!(
        stdout.contains("--trim-start-seconds <SECONDS>"),
        "{stdout}"
    );
    assert!(stdout.contains("--trim-end-seconds <SECONDS>"), "{stdout}");
    assert!(stdout.contains("--pad-start-frame <FRAMES>"), "{stdout}");
    assert!(stdout.contains("--pad-end-frame <FRAMES>"), "{stdout}");
    assert!(stdout.contains("--fade-in-frame <FRAMES>"), "{stdout}");
    assert!(stdout.contains("--fade-out-frame <FRAMES>"), "{stdout}");
    assert!(stdout.contains("--reverse"), "{stdout}");
    assert!(
        stdout.contains("Reverse frame order within each channel"),
        "{stdout}"
    );
    assert!(stdout.contains("--effects-file <FILE>"), "{stdout}");
    assert!(
        stdout.contains("Read the effect chain from a SoX-ng-style effects file"),
        "{stdout}"
    );
    assert!(stdout.contains("[EFFECT]..."), "{stdout}");
    assert!(
        stdout.contains("Positional SoX-ng-style effect chain tokens"),
        "{stdout}"
    );
}

#[test]
fn run_unsupported_input_extension_returns_clear_error() {
    let input = temp_path("auralis-cli-run-input-unsupported", "flac");
    let output = temp_path("auralis-cli-run-output", "wav");
    fs::write(&input, b"not a supported input").unwrap();

    let command_output = Command::new(env!("CARGO_BIN_EXE_auralis"))
        .args(["run", input.to_str().unwrap(), output.to_str().unwrap()])
        .output()
        .unwrap();

    fs::remove_file(input).unwrap();
    let _ = fs::remove_file(output);
    assert!(!command_output.status.success());
    let stderr = stderr(&command_output);
    assert!(
        stderr.contains("error: unsupported input format"),
        "{stderr}"
    );
    assert!(stderr.contains("only PCM16 WAV is supported"), "{stderr}");
}

#[test]
fn run_unsupported_output_extension_returns_clear_error() {
    let input = temp_path("auralis-cli-run-input", "wav");
    let output = temp_path("auralis-cli-run-output-unsupported", "flac");
    write_pcm16_wav(&input, 1, &[0]);

    let command_output = Command::new(env!("CARGO_BIN_EXE_auralis"))
        .args(["run", input.to_str().unwrap(), output.to_str().unwrap()])
        .output()
        .unwrap();

    fs::remove_file(input).unwrap();
    let _ = fs::remove_file(output);
    assert!(!command_output.status.success());
    let stderr = stderr(&command_output);
    assert!(
        stderr.contains("error: unsupported output format"),
        "{stderr}"
    );
    assert!(stderr.contains("only PCM16 WAV is supported"), "{stderr}");
}

fn write_pcm16_wav(path: &Path, channels: u16, samples: &[i16]) {
    let mut bytes = riff_header(channels, 16, 1, samples.len() * 2);
    for sample in samples {
        bytes.extend_from_slice(&sample.to_le_bytes());
    }
    fs::write(path, bytes).unwrap();
}

fn write_pcm16_wav_with_metadata(path: &Path, channels: u16, samples: &[i16]) {
    let data_bytes = samples.len() * 2;
    let metadata = b"LIST\x04\0\0\0INFO";
    let mut bytes = riff_header_with_extra(channels, 16, 1, data_bytes, metadata.len());
    bytes.extend_from_slice(metadata);
    bytes.extend_from_slice(b"data");
    bytes.extend_from_slice(&u32::try_from(data_bytes).unwrap().to_le_bytes());
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
    let mut bytes = riff_header_with_extra(channels, bits_per_sample, format_tag, data_bytes, 0);

    bytes.extend_from_slice(b"data");
    bytes.extend_from_slice(&u32::try_from(data_bytes).unwrap().to_le_bytes());

    bytes
}

fn riff_header_with_extra(
    channels: u16,
    bits_per_sample: u16,
    format_tag: u16,
    data_bytes: usize,
    extra_bytes: usize,
) -> Vec<u8> {
    let sample_rate = 48_000_u32;
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

fn read_pcm16_wav(path: &Path) -> (u16, Vec<i16>) {
    let (_, channels, samples) = read_pcm16_wav_with_sample_rate(path);
    (channels, samples)
}

fn read_pcm16_wav_with_sample_rate(path: &Path) -> (u32, u16, Vec<i16>) {
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

fn metadata_chunk_is_absent(path: &Path) -> bool {
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

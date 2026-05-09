//! CLI integration tests for automatic output channel and rate policies.

mod support;

use support::*;

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

//! CLI integration tests for automatic output channel and rate policies.

mod support;

use support::*;

#[test]
fn render_channels_downmixes_stereo_to_mono() {
    let input = temp_path("auralis-cli-render-channels-downmix-input", "wav");
    let output = temp_path("auralis-cli-render-channels-downmix-output", "wav");
    write_pcm16_wav(&input, 2, &[8192, 24_576, -16_384, 16_384]);

    let command_output = Command::new(env!("CARGO_BIN_EXE_auralis"))
        .args([
            "render",
            input.to_str().unwrap(),
            "-o",
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
fn render_channels_upmixes_mono_to_stereo() {
    let input = temp_path("auralis-cli-render-channels-upmix-input", "wav");
    let output = temp_path("auralis-cli-render-channels-upmix-output", "wav");
    write_pcm16_wav(&input, 1, &[8192, -16_384]);

    let command_output = Command::new(env!("CARGO_BIN_EXE_auralis"))
        .args([
            "render",
            input.to_str().unwrap(),
            "-o",
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
fn render_no_auto_channels_rejects_mismatched_output_count() {
    let input = temp_path("auralis-cli-render-no-auto-channels-input", "wav");
    let output = temp_path("auralis-cli-render-no-auto-channels-output", "wav");
    write_pcm16_wav(&input, 2, &[1000, -1000]);

    let command_output = Command::new(env!("CARGO_BIN_EXE_auralis"))
        .args([
            "render",
            input.to_str().unwrap(),
            "-o",
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
fn render_no_auto_channels_requires_output_channels() {
    let input = temp_path("auralis-cli-render-no-auto-channels-missing-input", "wav");
    let output = temp_path("auralis-cli-render-no-auto-channels-missing-output", "wav");
    write_pcm16_wav(&input, 1, &[1000]);

    let command_output = Command::new(env!("CARGO_BIN_EXE_auralis"))
        .args([
            "render",
            input.to_str().unwrap(),
            "-o",
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
fn render_rate_downsamples_output_sample_rate() {
    let input = temp_path("auralis-cli-render-rate-downsample-input", "wav");
    let output = temp_path("auralis-cli-render-rate-downsample-output", "wav");
    write_pcm16_wav(&input, 1, &[1000, 1000, 1000, 1000]);

    let command_output = Command::new(env!("CARGO_BIN_EXE_auralis"))
        .args([
            "render",
            input.to_str().unwrap(),
            "-o",
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
fn render_rate_upsamples_output_sample_rate() {
    let input = temp_path("auralis-cli-render-rate-upsample-input", "wav");
    let output = temp_path("auralis-cli-render-rate-upsample-output", "wav");
    write_pcm16_wav(&input, 1, &[1000, 1000]);

    let command_output = Command::new(env!("CARGO_BIN_EXE_auralis"))
        .args([
            "render",
            input.to_str().unwrap(),
            "-o",
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
fn render_no_auto_rate_rejects_mismatched_output_rate() {
    let input = temp_path("auralis-cli-render-no-auto-rate-input", "wav");
    let output = temp_path("auralis-cli-render-no-auto-rate-output", "wav");
    write_pcm16_wav(&input, 1, &[1000, -1000]);

    let command_output = Command::new(env!("CARGO_BIN_EXE_auralis"))
        .args([
            "render",
            input.to_str().unwrap(),
            "-o",
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
fn render_no_auto_rate_requires_output_rate() {
    let input = temp_path("auralis-cli-render-no-auto-rate-missing-input", "wav");
    let output = temp_path("auralis-cli-render-no-auto-rate-missing-output", "wav");
    write_pcm16_wav(&input, 1, &[1000]);

    let command_output = Command::new(env!("CARGO_BIN_EXE_auralis"))
        .args([
            "render",
            input.to_str().unwrap(),
            "-o",
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
fn render_wav_sample_format_writes_pcm24_header() {
    let input = temp_path("auralis-cli-render-sample-input", "wav");
    let output = temp_path("auralis-cli-render-sample-output", "wav");
    write_pcm16_wav(&input, 1, &[1000, -1000, 2000, -2000]);

    let command_output = Command::new(env!("CARGO_BIN_EXE_auralis"))
        .args([
            "render",
            input.to_str().unwrap(),
            "-o",
            output.to_str().unwrap(),
            "--sample",
            "pcm24",
        ])
        .output()
        .unwrap();

    fs::remove_file(input).unwrap();
    assert!(
        command_output.status.success(),
        "stderr: {}",
        stderr(&command_output)
    );
    assert_eq!(read_wav_bits_per_sample(&output), 24);
    fs::remove_file(output).unwrap();
}

#[test]
fn render_container_overrides_output_extension() {
    let input = temp_path("auralis-cli-render-container-input", "wav");
    let output = temp_path("auralis-cli-render-container-output", "audio");
    write_pcm16_wav(
        &input,
        1,
        &[
            -16_384, -14_336, -12_288, -10_240, -8_192, -6_144, -4_096, -2_048, 0, 2_048, 4_096,
            6_144, 8_192, 10_240, 12_288, 14_336,
        ],
    );

    let command_output = Command::new(env!("CARGO_BIN_EXE_auralis"))
        .args([
            "render",
            input.to_str().unwrap(),
            "-o",
            output.to_str().unwrap(),
            "--container",
            "flac",
            "--fx",
            "gain -3",
        ])
        .output()
        .unwrap();

    fs::remove_file(input).unwrap();
    assert!(
        command_output.status.success(),
        "stderr: {}",
        stderr(&command_output)
    );
    let bytes = fs::read(&output).unwrap();
    assert_eq!(&bytes[..4], b"fLaC");
    fs::remove_file(output).unwrap();
}

#[test]
fn render_rejects_wav_sample_for_non_wav_output() {
    let input = temp_path("auralis-cli-render-sample-non-wav-input", "wav");
    let output = temp_path("auralis-cli-render-sample-non-wav-output", "flac");
    write_pcm16_wav(&input, 1, &[0]);

    let command_output = Command::new(env!("CARGO_BIN_EXE_auralis"))
        .args([
            "render",
            input.to_str().unwrap(),
            "-o",
            output.to_str().unwrap(),
            "--sample",
            "pcm24",
        ])
        .output()
        .unwrap();

    fs::remove_file(input).unwrap();
    let _ = fs::remove_file(output);
    assert!(!command_output.status.success());
    let stderr = stderr(&command_output);
    assert!(
        stderr.contains("error: --sample is supported only for WAV output"),
        "{stderr}"
    );
}

#[test]
fn render_dither_is_explicit_and_repeatable() {
    let input = temp_path("auralis-cli-render-dither-input", "wav");
    let plain_output = temp_path("auralis-cli-render-dither-plain-output", "wav");
    let first_output = temp_path("auralis-cli-render-dither-first-output", "wav");
    let second_output = temp_path("auralis-cli-render-dither-second-output", "wav");
    write_pcm16_wav(&input, 1, &[1000, -1000, 2000, -2000, 3000, -3000]);

    let plain = Command::new(env!("CARGO_BIN_EXE_auralis"))
        .args([
            "render",
            input.to_str().unwrap(),
            "-o",
            plain_output.to_str().unwrap(),
            "--fx",
            "gain -0.1",
        ])
        .output()
        .unwrap();
    let first = Command::new(env!("CARGO_BIN_EXE_auralis"))
        .args([
            "render",
            input.to_str().unwrap(),
            "-o",
            first_output.to_str().unwrap(),
            "--fx",
            "gain -0.1",
            "--dither",
            "--dither-seed",
            "0",
        ])
        .output()
        .unwrap();
    let second = Command::new(env!("CARGO_BIN_EXE_auralis"))
        .args([
            "render",
            input.to_str().unwrap(),
            "-o",
            second_output.to_str().unwrap(),
            "--fx",
            "gain -0.1",
            "--dither",
            "--dither-seed",
            "0",
        ])
        .output()
        .unwrap();

    fs::remove_file(input).unwrap();
    assert!(plain.status.success(), "stderr: {}", stderr(&plain));
    assert!(first.status.success(), "stderr: {}", stderr(&first));
    assert!(second.status.success(), "stderr: {}", stderr(&second));
    assert_eq!(
        read_pcm16_wav(&first_output),
        read_pcm16_wav(&second_output)
    );
    assert_ne!(read_pcm16_wav(&plain_output), read_pcm16_wav(&first_output));
    fs::remove_file(plain_output).unwrap();
    fs::remove_file(first_output).unwrap();
    fs::remove_file(second_output).unwrap();
}

#[test]
fn render_dither_seed_requires_dither() {
    let input = temp_path("auralis-cli-render-dither-seed-missing-input", "wav");
    let output = temp_path("auralis-cli-render-dither-seed-missing-output", "wav");
    write_pcm16_wav(&input, 1, &[1000]);

    let command_output = Command::new(env!("CARGO_BIN_EXE_auralis"))
        .args([
            "render",
            input.to_str().unwrap(),
            "-o",
            output.to_str().unwrap(),
            "--dither-seed",
            "0",
        ])
        .output()
        .unwrap();

    fs::remove_file(input).unwrap();
    let _ = fs::remove_file(output);
    assert!(!command_output.status.success());
    let stderr = stderr(&command_output);
    assert!(
        stderr.contains("error: --dither-seed requires --dither"),
        "{stderr}"
    );
}

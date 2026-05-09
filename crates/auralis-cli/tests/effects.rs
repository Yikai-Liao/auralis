//! CLI integration tests for implemented effects and level policies.

mod support;

use auralis::AudioFile;
use support::*;

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
fn run_guard_attenuates_output_that_would_clip() {
    let input = temp_path("auralis-cli-run-guard-input", "wav");
    let output = temp_path("auralis-cli-run-guard-output", "wav");
    write_pcm16_wav(&input, 1, &[24_576, 8_192]);

    let command_output = Command::new(env!("CARGO_BIN_EXE_auralis"))
        .args([
            "run",
            input.to_str().unwrap(),
            output.to_str().unwrap(),
            "--guard",
            "--gain-db",
            "6",
        ])
        .output()
        .unwrap();

    assert!(
        command_output.status.success(),
        "stderr: {}",
        stderr(&command_output)
    );
    assert_eq!(read_pcm16_wav(&output), (1, vec![32_767, 10_923]));

    fs::remove_file(input).unwrap();
    fs::remove_file(output).unwrap();
}

#[test]
fn run_norm_without_value_normalizes_output_to_full_scale() {
    let input = temp_path("auralis-cli-run-norm-input", "wav");
    let output = temp_path("auralis-cli-run-norm-output", "wav");
    write_pcm16_wav(&input, 1, &[8_192, -16_384]);

    let command_output = Command::new(env!("CARGO_BIN_EXE_auralis"))
        .args([
            "run",
            input.to_str().unwrap(),
            output.to_str().unwrap(),
            "--norm",
        ])
        .output()
        .unwrap();

    assert!(
        command_output.status.success(),
        "stderr: {}",
        stderr(&command_output)
    );
    assert_eq!(read_pcm16_wav(&output), (1, vec![16_384, -32_768]));

    fs::remove_file(input).unwrap();
    fs::remove_file(output).unwrap();
}

#[test]
fn run_rejects_mixed_guard_and_norm() {
    let input = temp_path("auralis-cli-run-mixed-guard-norm-input", "wav");
    let output = temp_path("auralis-cli-run-mixed-guard-norm-output", "wav");
    write_pcm16_wav(&input, 1, &[0]);

    let command_output = Command::new(env!("CARGO_BIN_EXE_auralis"))
        .args([
            "run",
            input.to_str().unwrap(),
            output.to_str().unwrap(),
            "--guard",
            "--norm",
        ])
        .output()
        .unwrap();

    fs::remove_file(input).unwrap();
    let _ = fs::remove_file(output);
    assert!(!command_output.status.success());
    let stderr = stderr(&command_output);
    assert!(
        stderr.contains("error: --guard cannot be combined with --norm"),
        "{stderr}"
    );
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

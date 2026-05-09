//! CLI integration tests for merge and multiply combiners.

mod support;

use support::*;

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

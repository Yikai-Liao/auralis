//! CLI integration tests for mix and mix-power combiners.

mod support;

use support::*;

#[test]
fn render_mix_averages_equal_length_mono_inputs() {
    let first = temp_path("auralis-cli-render-mix-mono-first", "wav");
    let second = temp_path("auralis-cli-render-mix-mono-second", "wav");
    let output = temp_path("auralis-cli-render-mix-mono-output", "wav");
    write_pcm16_wav(&first, 1, &[1000, -1000]);
    write_pcm16_wav(&second, 1, &[3000, 1000]);

    let command_output = Command::new(env!("CARGO_BIN_EXE_auralis"))
        .args([
            "render",
            first.to_str().unwrap(),
            "-o",
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
fn render_mix_treats_mismatched_lengths_as_trailing_silence() {
    let first = temp_path("auralis-cli-render-mix-length-first", "wav");
    let second = temp_path("auralis-cli-render-mix-length-second", "wav");
    let output = temp_path("auralis-cli-render-mix-length-output", "wav");
    write_pcm16_wav(&first, 1, &[1000, -1000]);
    write_pcm16_wav(&second, 1, &[500, 0, -500]);

    let command_output = Command::new(env!("CARGO_BIN_EXE_auralis"))
        .args([
            "render",
            first.to_str().unwrap(),
            "-o",
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
fn render_mix_combines_stereo_inputs_before_effects() {
    let first = temp_path("auralis-cli-render-mix-stereo-first", "wav");
    let second = temp_path("auralis-cli-render-mix-stereo-second", "wav");
    let output = temp_path("auralis-cli-render-mix-stereo-output", "wav");
    write_pcm16_wav(&first, 2, &[-1000, 1000, -2000, 2000]);
    write_pcm16_wav(&second, 2, &[3000, -1000, 1000, -3000]);

    let command_output = Command::new(env!("CARGO_BIN_EXE_auralis"))
        .args([
            "render",
            first.to_str().unwrap(),
            "-o",
            output.to_str().unwrap(),
            "--combine",
            "mix",
            "--input",
            second.to_str().unwrap(),
            "--chain",
            "reverse",
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
fn render_mix_accepts_mismatched_channel_counts() {
    let first = temp_path("auralis-cli-render-mix-channel-first", "wav");
    let second = temp_path("auralis-cli-render-mix-channel-second", "wav");
    let output = temp_path("auralis-cli-render-mix-channel-output", "wav");
    write_pcm16_wav(&first, 1, &[1000, -1000]);
    write_pcm16_wav(&second, 2, &[3000, 1000, -1000, 500]);

    let command_output = Command::new(env!("CARGO_BIN_EXE_auralis"))
        .args([
            "render",
            first.to_str().unwrap(),
            "-o",
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
fn render_mix_backend_scalar_and_requested_simd_match() {
    let first = temp_path("auralis-cli-render-mix-backend-first", "wav");
    let second = temp_path("auralis-cli-render-mix-backend-second", "wav");
    let scalar_output = temp_path("auralis-cli-render-mix-backend-scalar-output", "wav");
    let simd_output = temp_path("auralis-cli-render-mix-backend-simd-output", "wav");
    write_pcm16_wav(&first, 1, &[-32768, -12345, 0, 12345, 32767]);
    write_pcm16_wav(&second, 1, &[32767, 12345, 0, -12345, -32768]);

    let scalar = Command::new(env!("CARGO_BIN_EXE_auralis"))
        .args([
            "render",
            first.to_str().unwrap(),
            "-o",
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
            "render",
            first.to_str().unwrap(),
            "-o",
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
fn render_mix_power_scales_equal_length_mono_inputs() {
    let first = temp_path("auralis-cli-render-mix-power-mono-first", "wav");
    let second = temp_path("auralis-cli-render-mix-power-mono-second", "wav");
    let output = temp_path("auralis-cli-render-mix-power-mono-output", "wav");
    write_pcm16_wav(&first, 1, &[1000, -1000]);
    write_pcm16_wav(&second, 1, &[3000, 1000]);

    let command_output = Command::new(env!("CARGO_BIN_EXE_auralis"))
        .args([
            "render",
            first.to_str().unwrap(),
            "-o",
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
fn render_mix_power_treats_mismatched_lengths_as_trailing_silence() {
    let first = temp_path("auralis-cli-render-mix-power-length-first", "wav");
    let second = temp_path("auralis-cli-render-mix-power-length-second", "wav");
    let output = temp_path("auralis-cli-render-mix-power-length-output", "wav");
    write_pcm16_wav(&first, 1, &[1000, -1000]);
    write_pcm16_wav(&second, 1, &[500, 0, -500]);

    let command_output = Command::new(env!("CARGO_BIN_EXE_auralis"))
        .args([
            "render",
            first.to_str().unwrap(),
            "-o",
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
fn render_mix_power_combines_stereo_inputs_before_effects() {
    let first = temp_path("auralis-cli-render-mix-power-stereo-first", "wav");
    let second = temp_path("auralis-cli-render-mix-power-stereo-second", "wav");
    let output = temp_path("auralis-cli-render-mix-power-stereo-output", "wav");
    write_pcm16_wav(&first, 2, &[-1000, 1000, -2000, 2000]);
    write_pcm16_wav(&second, 2, &[3000, -1000, 1000, -3000]);

    let command_output = Command::new(env!("CARGO_BIN_EXE_auralis"))
        .args([
            "render",
            first.to_str().unwrap(),
            "-o",
            output.to_str().unwrap(),
            "--combine",
            "mix-power",
            "--input",
            second.to_str().unwrap(),
            "--chain",
            "reverse",
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
fn render_mix_power_backend_scalar_and_requested_simd_match() {
    let first = temp_path("auralis-cli-render-mix-power-backend-first", "wav");
    let second = temp_path("auralis-cli-render-mix-power-backend-second", "wav");
    let scalar_output = temp_path("auralis-cli-render-mix-power-backend-scalar-output", "wav");
    let simd_output = temp_path("auralis-cli-render-mix-power-backend-simd-output", "wav");
    write_pcm16_wav(&first, 1, &[-32768, -12345, 0, 12345, 32767]);
    write_pcm16_wav(&second, 1, &[32767, 12345, 0, -12345, -32768]);

    let scalar = Command::new(env!("CARGO_BIN_EXE_auralis"))
        .args([
            "render",
            first.to_str().unwrap(),
            "-o",
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
            "render",
            first.to_str().unwrap(),
            "-o",
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

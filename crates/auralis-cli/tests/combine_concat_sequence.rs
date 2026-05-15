//! CLI integration tests for concatenate and sequence combiners.

mod support;

use support::*;

#[test]
fn render_concatenate_accepts_mismatched_mono_lengths() {
    let first = temp_path("auralis-cli-render-concat-mono-first", "wav");
    let second = temp_path("auralis-cli-render-concat-mono-second", "wav");
    let output = temp_path("auralis-cli-render-concat-mono-output", "wav");
    write_pcm16_wav(&first, 1, &[1000, -1000]);
    write_pcm16_wav(&second, 1, &[500, 0, -500]);

    let command_output = Command::new(env!("CARGO_BIN_EXE_auralis"))
        .args([
            "render",
            first.to_str().unwrap(),
            "-o",
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
fn render_concatenate_combines_stereo_inputs_before_effects() {
    let first = temp_path("auralis-cli-render-concat-stereo-first", "wav");
    let second = temp_path("auralis-cli-render-concat-stereo-second", "wav");
    let output = temp_path("auralis-cli-render-concat-stereo-output", "wav");
    write_pcm16_wav(&first, 2, &[-1000, 1000, -2000, 2000]);
    write_pcm16_wav(&second, 2, &[-3000, 3000]);

    let command_output = Command::new(env!("CARGO_BIN_EXE_auralis"))
        .args([
            "render",
            first.to_str().unwrap(),
            "-o",
            output.to_str().unwrap(),
            "--combine",
            "concatenate",
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
    assert_eq!(
        read_pcm16_wav(&output),
        (2, vec![-3000, 3000, -2000, 2000, -1000, 1000])
    );

    fs::remove_file(first).unwrap();
    fs::remove_file(second).unwrap();
    fs::remove_file(output).unwrap();
}

#[test]
fn render_concatenate_rejects_mismatched_channel_count() {
    let first = temp_path("auralis-cli-render-concat-mismatch-first", "wav");
    let second = temp_path("auralis-cli-render-concat-mismatch-second", "wav");
    let output = temp_path("auralis-cli-render-concat-mismatch-output", "wav");
    write_pcm16_wav(&first, 1, &[1000, -1000]);
    write_pcm16_wav(&second, 2, &[500, -500]);

    let command_output = Command::new(env!("CARGO_BIN_EXE_auralis"))
        .args([
            "render",
            first.to_str().unwrap(),
            "-o",
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
fn render_sequence_accepts_mismatched_mono_lengths() {
    let first = temp_path("auralis-cli-render-sequence-mono-first", "wav");
    let second = temp_path("auralis-cli-render-sequence-mono-second", "wav");
    let output = temp_path("auralis-cli-render-sequence-mono-output", "wav");
    write_pcm16_wav(&first, 1, &[1000, -1000]);
    write_pcm16_wav(&second, 1, &[500, 0, -500]);

    let command_output = Command::new(env!("CARGO_BIN_EXE_auralis"))
        .args([
            "render",
            first.to_str().unwrap(),
            "-o",
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
fn render_sequence_combines_stereo_inputs_before_effects() {
    let first = temp_path("auralis-cli-render-sequence-stereo-first", "wav");
    let second = temp_path("auralis-cli-render-sequence-stereo-second", "wav");
    let output = temp_path("auralis-cli-render-sequence-stereo-output", "wav");
    write_pcm16_wav(&first, 2, &[-1000, 1000, -2000, 2000]);
    write_pcm16_wav(&second, 2, &[-3000, 3000]);

    let command_output = Command::new(env!("CARGO_BIN_EXE_auralis"))
        .args([
            "render",
            first.to_str().unwrap(),
            "-o",
            output.to_str().unwrap(),
            "--combine",
            "sequence",
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
    assert_eq!(
        read_pcm16_wav(&output),
        (2, vec![-3000, 3000, -2000, 2000, -1000, 1000])
    );

    fs::remove_file(first).unwrap();
    fs::remove_file(second).unwrap();
    fs::remove_file(output).unwrap();
}

#[test]
fn render_sequence_rejects_unrepresentable_channel_boundary() {
    let first = temp_path("auralis-cli-render-sequence-mismatch-first", "wav");
    let second = temp_path("auralis-cli-render-sequence-mismatch-second", "wav");
    let output = temp_path("auralis-cli-render-sequence-mismatch-output", "wav");
    write_pcm16_wav(&first, 1, &[1000, -1000]);
    write_pcm16_wav(&second, 2, &[500, -500]);

    let command_output = Command::new(env!("CARGO_BIN_EXE_auralis"))
        .args([
            "render",
            first.to_str().unwrap(),
            "-o",
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

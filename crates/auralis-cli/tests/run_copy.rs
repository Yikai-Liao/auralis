//! CLI integration tests for decode/encode copy workflows.

mod support;

use support::*;

#[test]
fn render_copies_mono_wav_through_decode_encode_pipeline() {
    let input = temp_path("auralis-cli-render-mono-input", "wav");
    let output = temp_path("auralis-cli-render-mono-output", "wav");
    let samples = [-32768, -1024, 0, 1024, 32767];
    write_pcm16_wav_with_metadata(&input, 1, &samples);

    let command_output = Command::new(env!("CARGO_BIN_EXE_auralis"))
        .args([
            "render",
            input.to_str().unwrap(),
            "-o",
            output.to_str().unwrap(),
        ])
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
fn render_copies_stereo_wav_samples_and_metadata() {
    let input = temp_path("auralis-cli-render-stereo-input", "wav");
    let output = temp_path("auralis-cli-render-stereo-output", "wav");
    let samples = [-32768, 32767, -12_000, 12_000, 0, 4096];
    write_pcm16_wav(&input, 2, &samples);

    let command_output = Command::new(env!("CARGO_BIN_EXE_auralis"))
        .args([
            "render",
            input.to_str().unwrap(),
            "-o",
            output.to_str().unwrap(),
        ])
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

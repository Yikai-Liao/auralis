//! CLI integration tests for implemented effects and level policies.

mod support;

use auralis::AudioFile;
use support::*;

#[test]
fn render_gain_output_matches_library_pipeline() {
    let input = temp_path("auralis-cli-render-gain-input", "wav");
    let cli_output = temp_path("auralis-cli-render-gain-cli-output", "wav");
    let library_output = temp_path("auralis-cli-render-gain-library-output", "wav");
    write_pcm16_wav(&input, 1, &[-16_384, -8_192, 0, 8_192, 16_384]);

    AudioFile::open_wav(&input)
        .unwrap()
        .into_pipeline()
        .gain_db(-6.0)
        .write_wav(&library_output)
        .unwrap();

    let command_output = Command::new(env!("CARGO_BIN_EXE_auralis"))
        .args([
            "render",
            input.to_str().unwrap(),
            "-o",
            cli_output.to_str().unwrap(),
            "--fx",
            "gain -6",
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
fn render_gain_output_matches_under_forced_scalar_and_requested_simd() {
    let input = temp_path("auralis-cli-render-gain-backend-input", "wav");
    let scalar_output = temp_path("auralis-cli-render-gain-scalar-output", "wav");
    let simd_output = temp_path("auralis-cli-render-gain-simd-output", "wav");
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
            "render",
            input.to_str().unwrap(),
            "-o",
            scalar_output.to_str().unwrap(),
            "--backend",
            "scalar",
            "--fx",
            "gain -3",
        ])
        .output()
        .unwrap();
    let simd_command_output = Command::new(env!("CARGO_BIN_EXE_auralis"))
        .args([
            "render",
            input.to_str().unwrap(),
            "-o",
            simd_output.to_str().unwrap(),
            "--backend",
            "simd",
            "--fx",
            "gain -3",
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
fn render_dc_shift_output_matches_library_pipeline_and_clips_at_wav_boundary() {
    let input = temp_path("auralis-cli-render-dc-shift-input", "wav");
    let cli_output = temp_path("auralis-cli-render-dc-shift-cli-output", "wav");
    let library_output = temp_path("auralis-cli-render-dc-shift-library-output", "wav");
    write_pcm16_wav(&input, 2, &[-32768, 0, 8192, 30_000]);

    AudioFile::open_wav(&input)
        .unwrap()
        .into_pipeline()
        .dc_shift(0.25)
        .write_wav(&library_output)
        .unwrap();

    let command_output = Command::new(env!("CARGO_BIN_EXE_auralis"))
        .args([
            "render",
            input.to_str().unwrap(),
            "-o",
            cli_output.to_str().unwrap(),
            "--fx",
            "dcshift 0.25",
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
fn render_dc_shift_output_matches_under_forced_scalar_and_requested_simd() {
    let input = temp_path("auralis-cli-render-dc-shift-backend-input", "wav");
    let scalar_output = temp_path("auralis-cli-render-dc-shift-scalar-output", "wav");
    let simd_output = temp_path("auralis-cli-render-dc-shift-simd-output", "wav");
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
            "render",
            input.to_str().unwrap(),
            "-o",
            scalar_output.to_str().unwrap(),
            "--backend",
            "scalar",
            "--fx",
            "dcshift 0.125",
        ])
        .output()
        .unwrap();
    let simd_command_output = Command::new(env!("CARGO_BIN_EXE_auralis"))
        .args([
            "render",
            input.to_str().unwrap(),
            "-o",
            simd_output.to_str().unwrap(),
            "--backend",
            "simd",
            "--fx",
            "dcshift 0.125",
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
fn render_trim_frames_output_matches_library_pipeline() {
    let input = temp_path("auralis-cli-render-trim-frame-input", "wav");
    let cli_output = temp_path("auralis-cli-render-trim-frame-cli-output", "wav");
    let library_output = temp_path("auralis-cli-render-trim-frame-library-output", "wav");
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
            "render",
            input.to_str().unwrap(),
            "-o",
            cli_output.to_str().unwrap(),
            "--fx",
            "trim 1 =3",
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
fn render_trim_seconds_output_matches_library_pipeline() {
    let input = temp_path("auralis-cli-render-trim-seconds-input", "wav");
    let cli_output = temp_path("auralis-cli-render-trim-seconds-cli-output", "wav");
    let library_output = temp_path("auralis-cli-render-trim-seconds-library-output", "wav");
    write_pcm16_wav(&input, 1, &[-1000, -500, 0, 500, 1000]);

    AudioFile::open_wav(&input)
        .unwrap()
        .into_pipeline()
        .trim_seconds(1.0 / 48_000.0, 4.0 / 48_000.0)
        .write_wav(&library_output)
        .unwrap();

    let command_output = Command::new(env!("CARGO_BIN_EXE_auralis"))
        .args([
            "render",
            input.to_str().unwrap(),
            "-o",
            cli_output.to_str().unwrap(),
            "--fx",
            "trim 1 =4",
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
fn render_pad_frames_output_matches_library_pipeline() {
    let input = temp_path("auralis-cli-render-pad-frame-input", "wav");
    let cli_output = temp_path("auralis-cli-render-pad-frame-cli-output", "wav");
    let library_output = temp_path("auralis-cli-render-pad-frame-library-output", "wav");
    write_pcm16_wav(&input, 2, &[-1000, 1000, -2000, 2000]);

    AudioFile::open_wav(&input)
        .unwrap()
        .into_pipeline()
        .pad_frames(1, 2)
        .write_wav(&library_output)
        .unwrap();

    let command_output = Command::new(env!("CARGO_BIN_EXE_auralis"))
        .args([
            "render",
            input.to_str().unwrap(),
            "-o",
            cli_output.to_str().unwrap(),
            "--fx",
            "pad 1 2",
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
fn render_single_sided_pad_defaults_other_side_to_zero() {
    let input = temp_path("auralis-cli-render-pad-single-input", "wav");
    let output = temp_path("auralis-cli-render-pad-single-output", "wav");
    write_pcm16_wav(&input, 1, &[1000, -1000]);

    let command_output = Command::new(env!("CARGO_BIN_EXE_auralis"))
        .args([
            "render",
            input.to_str().unwrap(),
            "-o",
            output.to_str().unwrap(),
            "--fx",
            "pad 0 2",
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
fn render_guard_attenuates_output_that_would_clip() {
    let input = temp_path("auralis-cli-render-guard-input", "wav");
    let output = temp_path("auralis-cli-render-guard-output", "wav");
    write_pcm16_wav(&input, 1, &[24_576, 8_192]);

    let command_output = Command::new(env!("CARGO_BIN_EXE_auralis"))
        .args([
            "render",
            input.to_str().unwrap(),
            "-o",
            output.to_str().unwrap(),
            "--guard",
            "--fx",
            "gain 6",
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
fn render_norm_without_value_normalizes_output_to_full_scale() {
    let input = temp_path("auralis-cli-render-norm-input", "wav");
    let output = temp_path("auralis-cli-render-norm-output", "wav");
    write_pcm16_wav(&input, 1, &[8_192, -16_384]);

    let command_output = Command::new(env!("CARGO_BIN_EXE_auralis"))
        .args([
            "render",
            input.to_str().unwrap(),
            "-o",
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
fn render_rejects_mixed_guard_and_norm() {
    let input = temp_path("auralis-cli-render-mixed-guard-norm-input", "wav");
    let output = temp_path("auralis-cli-render-mixed-guard-norm-output", "wav");
    write_pcm16_wav(&input, 1, &[0]);

    let command_output = Command::new(env!("CARGO_BIN_EXE_auralis"))
        .args([
            "render",
            input.to_str().unwrap(),
            "-o",
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
fn render_reverse_output_matches_library_pipeline() {
    let input = temp_path("auralis-cli-render-reverse-input", "wav");
    let cli_output = temp_path("auralis-cli-render-reverse-cli-output", "wav");
    let library_output = temp_path("auralis-cli-render-reverse-library-output", "wav");
    write_pcm16_wav(&input, 2, &[-1000, 1000, -2000, 2000, -3000, 3000]);

    AudioFile::open_wav(&input)
        .unwrap()
        .into_pipeline()
        .reverse()
        .write_wav(&library_output)
        .unwrap();

    let command_output = Command::new(env!("CARGO_BIN_EXE_auralis"))
        .args([
            "render",
            input.to_str().unwrap(),
            "-o",
            cli_output.to_str().unwrap(),
            "--fx",
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
    assert_eq!(
        read_pcm16_wav(&cli_output),
        (2, vec![-3000, 3000, -2000, 2000, -1000, 1000])
    );

    fs::remove_file(input).unwrap();
    fs::remove_file(cli_output).unwrap();
    fs::remove_file(library_output).unwrap();
}

#[test]
fn render_fade_frames_output_matches_library_pipeline() {
    let input = temp_path("auralis-cli-render-fade-frame-input", "wav");
    let cli_output = temp_path("auralis-cli-render-fade-frame-cli-output", "wav");
    let library_output = temp_path("auralis-cli-render-fade-frame-library-output", "wav");
    write_pcm16_wav(
        &input,
        2,
        &[-10000, 10000, -20000, 20000, -30000, 30000, -4000, 4000],
    );

    let chain_tokens = ["fade", "t", "2", "4", "2"];
    let chain = auralis::parse_effect_chain(&chain_tokens).unwrap();
    AudioFile::open_wav(&input)
        .unwrap()
        .into_pipeline()
        .apply_effect_chain(&chain)
        .write_wav(&library_output)
        .unwrap();

    let command_output = Command::new(env!("CARGO_BIN_EXE_auralis"))
        .args([
            "render",
            input.to_str().unwrap(),
            "-o",
            cli_output.to_str().unwrap(),
            "--fx",
            "fade t 2 4 2",
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
        (2, vec![0, 0, -10000, 10000, -30000, 30000, -2000, 2000])
    );

    fs::remove_file(input).unwrap();
    fs::remove_file(cli_output).unwrap();
    fs::remove_file(library_output).unwrap();
}

#[test]
fn render_fade_frames_output_matches_under_forced_scalar_and_requested_simd() {
    let input = temp_path("auralis-cli-render-fade-frame-backend-input", "wav");
    let scalar_output = temp_path("auralis-cli-render-fade-frame-scalar-output", "wav");
    let simd_output = temp_path("auralis-cli-render-fade-frame-simd-output", "wav");
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
            "render",
            input.to_str().unwrap(),
            "-o",
            scalar_output.to_str().unwrap(),
            "--backend",
            "scalar",
            "--fx",
            "fade t 2 8 2",
        ])
        .output()
        .unwrap();
    let simd_command_output = Command::new(env!("CARGO_BIN_EXE_auralis"))
        .args([
            "render",
            input.to_str().unwrap(),
            "-o",
            simd_output.to_str().unwrap(),
            "--backend",
            "simd",
            "--fx",
            "fade t 2 8 2",
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

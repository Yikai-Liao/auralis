//! CLI integration tests for diagnostics and help text.

mod support;

use support::*;

#[test]
fn render_invalid_gain_argument_returns_clear_error() {
    let input = temp_path("auralis-cli-render-invalid-gain-input", "wav");
    let output = temp_path("auralis-cli-render-invalid-gain-output", "wav");
    write_pcm16_wav(&input, 1, &[0]);

    let command_output = Command::new(env!("CARGO_BIN_EXE_auralis"))
        .args([
            "render",
            input.to_str().unwrap(),
            "-o",
            output.to_str().unwrap(),
            "--fx",
            "gain NaN",
        ])
        .output()
        .unwrap();

    fs::remove_file(input).unwrap();
    let _ = fs::remove_file(output);
    assert!(!command_output.status.success());
    let stderr = stderr(&command_output);
    assert!(stderr.contains("decibels must be finite"), "{stderr}");
}

#[test]
fn render_invalid_dc_shift_argument_returns_clear_error() {
    let input = temp_path("auralis-cli-render-invalid-dc-shift-input", "wav");
    let output = temp_path("auralis-cli-render-invalid-dc-shift-output", "wav");
    write_pcm16_wav(&input, 1, &[0]);

    let command_output = Command::new(env!("CARGO_BIN_EXE_auralis"))
        .args([
            "render",
            input.to_str().unwrap(),
            "-o",
            output.to_str().unwrap(),
            "--fx",
            "dcshift 2.1",
        ])
        .output()
        .unwrap();

    fs::remove_file(input).unwrap();
    let _ = fs::remove_file(output);
    assert!(!command_output.status.success());
    let stderr = stderr(&command_output);
    assert!(
        stderr.contains("dc shift must be finite and in the range -2.0..=2.0"),
        "{stderr}"
    );
}

#[test]
fn render_invalid_trim_range_returns_clear_error() {
    let input = temp_path("auralis-cli-render-invalid-trim-input", "wav");
    let output = temp_path("auralis-cli-render-invalid-trim-output", "wav");
    write_pcm16_wav(&input, 1, &[0, 1, 2]);

    let command_output = Command::new(env!("CARGO_BIN_EXE_auralis"))
        .args([
            "render",
            input.to_str().unwrap(),
            "-o",
            output.to_str().unwrap(),
            "--fx",
            "trim 3 =1",
        ])
        .output()
        .unwrap();

    fs::remove_file(input).unwrap();
    let _ = fs::remove_file(output);
    assert!(!command_output.status.success());
    let stderr = stderr(&command_output);
    assert!(
        stderr.contains("trim start frame must be less than or equal"),
        "{stderr}"
    );
}

#[test]
fn check_missing_trim_position_returns_clear_error() {
    let command_output = Command::new(env!("CARGO_BIN_EXE_auralis"))
        .args(["check", "--fx", "trim"])
        .output()
        .unwrap();

    assert!(!command_output.status.success());
    let stderr = stderr(&command_output);
    assert!(
        stderr.contains("error: effect chain command 0 (`trim`) failed to parse: effect `trim` requires argument `position`"),
        "{stderr}"
    );
}

#[test]
fn render_help_documents_modern_effect_inputs() {
    let command_output = Command::new(env!("CARGO_BIN_EXE_auralis"))
        .args(["render", "--help"])
        .output()
        .unwrap();

    assert!(command_output.status.success());
    let stdout = stdout(&command_output);
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
    assert!(stdout.contains("--guard"), "{stdout}");
    assert!(
        stdout.contains("Attenuate final output only if it would clip"),
        "{stdout}"
    );
    assert!(stdout.contains("--norm [<DB>]"), "{stdout}");
    assert!(
        stdout.contains("Normalize final output to a peak level"),
        "{stdout}"
    );
    assert!(stdout.contains("--dither"), "{stdout}");
    assert!(
        stdout.contains("Apply deterministic TPDF dither before PCM16 encoding"),
        "{stdout}"
    );
    assert!(stdout.contains("--dither-seed <SEED>"), "{stdout}");
    assert!(stdout.contains("--fx <EFFECT>"), "{stdout}");
    assert!(
        stdout.contains("One typed effect command per flag"),
        "{stdout}"
    );
    assert!(stdout.contains("--chain <CHAIN>"), "{stdout}");
    assert!(stdout.contains("Compact ordered effect chain"), "{stdout}");
    assert!(stdout.contains("--effects-file <FILE>"), "{stdout}");
    assert!(
        stdout.contains("Read the effect chain from a SoX-ng-style effects file"),
        "{stdout}"
    );
}

#[test]
fn render_unsupported_input_extension_returns_clear_error() {
    let input = temp_path("auralis-cli-render-input-unsupported", "flac");
    let output = temp_path("auralis-cli-render-output", "wav");
    fs::write(&input, b"not a supported input").unwrap();

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
fn render_unsupported_output_extension_returns_clear_error() {
    let input = temp_path("auralis-cli-render-input", "wav");
    let output = temp_path("auralis-cli-render-output-unsupported", "flac");
    write_pcm16_wav(&input, 1, &[0]);

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
    let _ = fs::remove_file(output);
    assert!(!command_output.status.success());
    let stderr = stderr(&command_output);
    assert!(
        stderr.contains("error: unsupported output format"),
        "{stderr}"
    );
    assert!(stderr.contains("only PCM16 WAV is supported"), "{stderr}");
}

//! CLI integration tests for diagnostics and help text.

mod support;

use support::*;

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

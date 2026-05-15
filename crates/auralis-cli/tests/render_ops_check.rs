//! Integration tests for `render`, `ops`, and `check`.

mod support;

use support::*;

#[test]
fn render_fx_output_matches_positional_run_chain() {
    let input = temp_path("auralis-cli-render-fx-input", "wav");
    let render_output = temp_path("auralis-cli-render-fx-output", "wav");
    let run_output = temp_path("auralis-cli-render-fx-run-output", "wav");
    write_pcm16_wav(&input, 1, &[-16_384, -8_192, 0, 8_192, 16_384]);

    let render = Command::new(env!("CARGO_BIN_EXE_auralis"))
        .args([
            "render",
            input.to_str().unwrap(),
            "-o",
            render_output.to_str().unwrap(),
            "--fx",
            "gain -6",
            "--fx",
            "reverse",
        ])
        .output()
        .unwrap();
    let run = Command::new(env!("CARGO_BIN_EXE_auralis"))
        .args([
            "run",
            input.to_str().unwrap(),
            run_output.to_str().unwrap(),
            "gain",
            "-6",
            "reverse",
        ])
        .output()
        .unwrap();

    assert!(render.status.success(), "stderr: {}", stderr(&render));
    assert!(run.status.success(), "stderr: {}", stderr(&run));
    assert_eq!(read_pcm16_wav(&render_output), read_pcm16_wav(&run_output));

    fs::remove_file(input).unwrap();
    fs::remove_file(render_output).unwrap();
    fs::remove_file(run_output).unwrap();
}

#[test]
fn check_fx_reports_ok_summary() {
    let command_output = Command::new(env!("CARGO_BIN_EXE_auralis"))
        .args(["check", "--fx", "gain -3", "--fx", "reverse"])
        .output()
        .unwrap();

    assert!(
        command_output.status.success(),
        "stderr: {}",
        stderr(&command_output)
    );
    let stdout = stdout(&command_output);
    assert!(stdout.contains("status: ok"), "{stdout}");
    assert!(stdout.contains("commands: 2"), "{stdout}");
    assert!(stdout.contains("boundaries: 0"), "{stdout}");
}

#[test]
fn check_requires_fx_or_effects_file() {
    let command_output = Command::new(env!("CARGO_BIN_EXE_auralis"))
        .args(["check"])
        .output()
        .unwrap();

    assert!(!command_output.status.success());
    let stderr = stderr(&command_output);
    assert!(
        stderr.contains("error: one of --fx or --effects-file is required"),
        "{stderr}"
    );
}

#[test]
fn check_rejects_unmatched_fx_quoting() {
    let command_output = Command::new(env!("CARGO_BIN_EXE_auralis"))
        .args(["check", "--fx", "'gain -3"])
        .output()
        .unwrap();

    assert!(!command_output.status.success());
    let stderr = stderr(&command_output);
    assert!(
        stderr.contains("error: effect string `'gain -3` contains unmatched shell quoting"),
        "{stderr}"
    );
}

#[test]
fn ops_lists_gain_and_reverse() {
    let command_output = Command::new(env!("CARGO_BIN_EXE_auralis"))
        .args(["ops"])
        .output()
        .unwrap();

    assert!(
        command_output.status.success(),
        "stderr: {}",
        stderr(&command_output)
    );
    let stdout = stdout(&command_output);
    assert!(stdout.contains("gain"), "{stdout}");
    assert!(stdout.contains("reverse"), "{stdout}");
}

#[test]
fn ops_effect_alias_resolves_to_descriptor() {
    let command_output = Command::new(env!("CARGO_BIN_EXE_auralis"))
        .args(["ops", "dc-shift"])
        .output()
        .unwrap();

    assert!(
        command_output.status.success(),
        "stderr: {}",
        stderr(&command_output)
    );
    let stdout = stdout(&command_output);
    assert!(stdout.contains("name: dcshift"), "{stdout}");
    assert!(stdout.contains("typed_api: DcShift"), "{stdout}");
    assert!(stdout.contains("aliases: dc-shift"), "{stdout}");
}

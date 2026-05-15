//! Integration tests for the `auralis man` command.

mod support;

use support::*;

#[test]
fn top_level_man_page_lists_modern_commands() {
    let output = Command::new(env!("CARGO_BIN_EXE_auralis"))
        .args(["man"])
        .output()
        .unwrap();

    assert!(output.status.success(), "stderr: {}", stderr(&output));
    let stdout = stdout(&output);
    assert!(stdout.contains("NAME"), "{stdout}");
    assert!(
        stdout.contains("auralis - modern deterministic audio processing CLI"),
        "{stdout}"
    );
    assert!(stdout.contains("trim"), "{stdout}");
    assert!(stdout.contains("normalize"), "{stdout}");
    assert!(stdout.contains("render"), "{stdout}");
    assert!(stdout.contains("plan"), "{stdout}");
    assert!(stdout.contains("completions"), "{stdout}");
    assert!(stdout.contains("man"), "{stdout}");
}

#[test]
fn render_man_page_includes_core_options() {
    let output = Command::new(env!("CARGO_BIN_EXE_auralis"))
        .args(["man", "render"])
        .output()
        .unwrap();

    assert!(output.status.success(), "stderr: {}", stderr(&output));
    let stdout = stdout(&output);
    assert!(stdout.contains("NAME"), "{stdout}");
    assert!(
        stdout.contains("render - run one ordered DSP pipeline"),
        "{stdout}"
    );
    assert!(stdout.contains("--fx EFFECT"), "{stdout}");
    assert!(stdout.contains("--chain CHAIN"), "{stdout}");
    assert!(stdout.contains("--combine METHOD"), "{stdout}");
}

#[test]
fn normalize_man_page_includes_peak_option() {
    let output = Command::new(env!("CARGO_BIN_EXE_auralis"))
        .args(["man", "normalize"])
        .output()
        .unwrap();

    assert!(output.status.success(), "stderr: {}", stderr(&output));
    let stdout = stdout(&output);
    assert!(
        stdout.contains("normalize - normalize to a peak level"),
        "{stdout}"
    );
    assert!(stdout.contains("--peak DBFS"), "{stdout}");
}

#[test]
fn man_rejects_unknown_topic() {
    let output = Command::new(env!("CARGO_BIN_EXE_auralis"))
        .args(["man", "missing"])
        .output()
        .unwrap();

    assert!(!output.status.success());
    let stderr = stderr(&output);
    assert!(
        stderr.contains("no built-in manual page for `missing`"),
        "{stderr}"
    );
}

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
    assert!(stdout.contains("gain"), "{stdout}");
    assert!(stdout.contains("reverse"), "{stdout}");
    assert!(stdout.contains("fade"), "{stdout}");
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
fn reverse_man_page_describes_recipe_lowering() {
    let output = Command::new(env!("CARGO_BIN_EXE_auralis"))
        .args(["man", "reverse"])
        .output()
        .unwrap();

    assert!(output.status.success(), "stderr: {}", stderr(&output));
    let stdout = stdout(&output);
    assert!(
        stdout.contains("reverse - reverse one audio file"),
        "{stdout}"
    );
    assert!(stdout.contains("render --fx reverse"), "{stdout}");
    assert!(stdout.contains("-o, --output FILE"), "{stdout}");
}

#[test]
fn gain_man_page_describes_recipe_lowering() {
    let output = Command::new(env!("CARGO_BIN_EXE_auralis"))
        .args(["man", "gain"])
        .output()
        .unwrap();

    assert!(output.status.success(), "stderr: {}", stderr(&output));
    let stdout = stdout(&output);
    assert!(
        stdout.contains("gain - adjust one audio file by gain"),
        "{stdout}"
    );
    assert!(stdout.contains("render --fx 'gain ...'"), "{stdout}");
    assert!(stdout.contains("-o, --output FILE"), "{stdout}");
}

#[test]
fn fade_man_page_describes_recipe_lowering() {
    let output = Command::new(env!("CARGO_BIN_EXE_auralis"))
        .args(["man", "fade"])
        .output()
        .unwrap();

    assert!(output.status.success(), "stderr: {}", stderr(&output));
    let stdout = stdout(&output);
    assert!(
        stdout.contains("fade - fade one audio file in or out"),
        "{stdout}"
    );
    assert!(stdout.contains("render --fx 'fade ...'"), "{stdout}");
    assert!(stdout.contains("--in FRAMES"), "{stdout}");
    assert!(stdout.contains("--out FRAMES"), "{stdout}");
    assert!(stdout.contains("--curve CURVE"), "{stdout}");
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

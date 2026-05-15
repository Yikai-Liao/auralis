//! Integration tests for the `auralis fmt` command.

mod support;

use support::*;

#[test]
fn fmt_rewrites_graph_spec_into_stable_pretty_toml() {
    let spec = temp_path("auralis-cli-fmt-spec", "toml");
    fs::write(
        &spec,
        r#"version = "auralis.graph/v1"
name = "episode"
[[sources]]
id = "voice"
path = "input/voice.wav"
[[chains]]
id = "voice_clean"
input = "voice.audio"
steps = [{ op = "trim", range = "10s..30s" }]
[[sinks]]
id = "wav"
input = "voice_clean.audio"
path = "build/out.wav"
"#,
    )
    .unwrap();

    let output = Command::new(env!("CARGO_BIN_EXE_auralis"))
        .args(["fmt", spec.to_str().unwrap()])
        .output()
        .unwrap();

    assert!(output.status.success(), "stderr: {}", stderr(&output));
    let formatted = fs::read_to_string(&spec).unwrap();
    fs::remove_file(spec).unwrap();
    assert!(formatted.contains("version = \"auralis.graph/v1\""));
    assert!(formatted.contains("[[chains.steps]]"));
    assert!(formatted.contains("range = \"10s..30s\""));
}

#[test]
fn fmt_check_reports_unformatted_spec() {
    let spec = temp_path("auralis-cli-fmt-check-spec", "toml");
    fs::write(
        &spec,
        r#"version = "auralis.graph/v1"
[[sources]]
id = "voice"
path = "input/voice.wav"
[[sinks]]
id = "wav"
input = "voice.audio"
path = "build/out.wav"
"#,
    )
    .unwrap();

    let output = Command::new(env!("CARGO_BIN_EXE_auralis"))
        .args(["fmt", spec.to_str().unwrap(), "--check"])
        .output()
        .unwrap();

    fs::remove_file(spec).unwrap();
    assert!(!output.status.success());
    let stderr = stderr(&output);
    assert!(stderr.contains("is not formatted"), "{stderr}");
    assert!(stderr.contains("run `auralis fmt"), "{stderr}");
}

#[test]
fn fmt_check_accepts_formatted_spec() {
    let spec = temp_path("auralis-cli-fmt-check-formatted", "toml");
    fs::write(
        &spec,
        r#"version = "auralis.graph/v1"

[[sources]]
id = "voice"
path = "input/voice.wav"

[[sinks]]
id = "wav"
input = "voice.audio"
path = "build/out.wav"
"#,
    )
    .unwrap();

    let output = Command::new(env!("CARGO_BIN_EXE_auralis"))
        .args(["fmt", spec.to_str().unwrap(), "--check"])
        .output()
        .unwrap();

    fs::remove_file(spec).unwrap();
    assert!(output.status.success(), "stderr: {}", stderr(&output));
}

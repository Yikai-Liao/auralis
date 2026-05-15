//! Integration tests for `auralis check --json`.

mod support;

use support::*;

#[test]
fn check_fx_json_reports_effect_summary() {
    let command_output = Command::new(env!("CARGO_BIN_EXE_auralis"))
        .args(["check", "--json", "--fx", "gain -3", "--fx", "reverse"])
        .output()
        .unwrap();

    assert!(
        command_output.status.success(),
        "stderr: {}",
        stderr(&command_output)
    );
    let check: serde_json::Value = serde_json::from_str(&stdout(&command_output)).unwrap();
    assert_eq!(check["status"], "ok");
    assert_eq!(check["kind"], "effects");
    assert_eq!(check["commands"], 2);
    assert_eq!(check["boundaries"], 0);
}

#[test]
fn check_graph_json_reports_spec_summary() {
    let spec_dir = temp_path("auralis-cli-check-json", "dir");
    fs::create_dir(&spec_dir).unwrap();
    let spec = spec_dir.join("Auralis.toml");
    fs::write(
        &spec,
        r#"
version = "auralis.graph/v1"

[[sources]]
id = "input"
path = "input.wav"

[[chains]]
id = "main"
input = "input.audio"
steps = [
  { op = "gain", by = "-3" },
]

[[sinks]]
id = "out"
input = "main.audio"
path = "out.wav"
"#,
    )
    .unwrap();

    let command_output = Command::new(env!("CARGO_BIN_EXE_auralis"))
        .args(["check", "--json", spec.to_str().unwrap()])
        .output()
        .unwrap();

    assert!(
        command_output.status.success(),
        "stderr: {}",
        stderr(&command_output)
    );
    let check: serde_json::Value = serde_json::from_str(&stdout(&command_output)).unwrap();
    assert_eq!(check["status"], "ok");
    assert_eq!(check["kind"], "graph");
    assert_eq!(check["sources"], 1);
    assert_eq!(check["chains"], 1);
    assert_eq!(check["nodes"], 0);
    assert_eq!(check["sinks"], 1);
    assert_eq!(check["expanded_steps"], 1);

    fs::remove_file(spec).unwrap();
    fs::remove_file(spec_dir.join("Auralis.lock")).unwrap();
    fs::remove_dir(spec_dir).unwrap();
}

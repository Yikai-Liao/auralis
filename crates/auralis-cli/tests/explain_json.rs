//! Integration tests for machine-readable `auralis explain` output.

mod support;

use support::*;

#[test]
fn explain_chain_json_reports_steps_modes_and_downstream() {
    let spec = temp_path("auralis-cli-explain-json-chain", "toml");
    fs::write(
        &spec,
        r#"version = "auralis.graph/v1"

[[sources]]
id = "voice"
path = "input/voice.wav"

[[chains]]
id = "voice_fx"
input = "voice.audio"
steps = [
  { id = "cut", op = "trim" },
  { op = "reverse" },
]

[[sinks]]
id = "wav"
input = "voice_fx.audio"
path = "build/out.wav"
"#,
    )
    .unwrap();

    let command_output = Command::new(env!("CARGO_BIN_EXE_auralis"))
        .args(["explain", spec.to_str().unwrap(), "voice_fx", "--json"])
        .output()
        .unwrap();

    fs::remove_file(spec).unwrap();
    assert!(
        command_output.status.success(),
        "{}",
        stderr(&command_output)
    );
    let explain: serde_json::Value = serde_json::from_str(&stdout(&command_output)).unwrap();
    assert_eq!(explain["id"], "voice_fx");
    assert_eq!(explain["kind"], "chain");
    assert_eq!(explain["input"], "voice.audio");
    assert_eq!(explain["output_port"], "voice_fx.audio");
    assert_eq!(explain["downstream"], serde_json::json!(["wav"]));
    assert_eq!(explain["steps"][0]["id"], "cut");
    assert_eq!(explain["steps"][0]["mode"], "streaming");
    assert_eq!(explain["steps"][1]["id"], "voice_fx/02-reverse");
    assert_eq!(explain["steps"][1]["mode"], "whole-buffer barrier");
}

#[test]
fn explain_node_json_reports_op_mode_inputs_and_downstream() {
    let spec = temp_path("auralis-cli-explain-json-node", "toml");
    fs::write(
        &spec,
        r#"version = "auralis.graph/v1"

[[sources]]
id = "voice"
path = "input/voice.wav"

[[nodes]]
id = "quiet"
op = "gain"
input = "voice.audio"
by = "-6dB"

[[sinks]]
id = "wav"
input = "quiet.audio"
path = "build/out.wav"
"#,
    )
    .unwrap();

    let command_output = Command::new(env!("CARGO_BIN_EXE_auralis"))
        .args(["explain", spec.to_str().unwrap(), "quiet", "--json"])
        .output()
        .unwrap();

    fs::remove_file(spec).unwrap();
    assert!(
        command_output.status.success(),
        "{}",
        stderr(&command_output)
    );
    let explain: serde_json::Value = serde_json::from_str(&stdout(&command_output)).unwrap();
    assert_eq!(explain["id"], "quiet");
    assert_eq!(explain["kind"], "node");
    assert_eq!(explain["op"], "gain -6dB");
    assert_eq!(explain["mode"], "streaming");
    assert_eq!(explain["inputs"], serde_json::json!(["voice.audio"]));
    assert_eq!(explain["downstream"], serde_json::json!(["wav"]));
}

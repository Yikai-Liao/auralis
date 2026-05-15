//! Snapshot tests for modern human-readable CLI output.

mod support;

use support::*;

#[test]
fn ops_gain_human_output_snapshot() {
    let output = Command::new(env!("CARGO_BIN_EXE_auralis"))
        .args(["ops", "gain"])
        .output()
        .unwrap();

    assert!(output.status.success(), "stderr: {}", stderr(&output));
    assert_eq!(
        stdout(&output),
        r#"name: gain
kind: Gain
summary: apply gain with optional SoX-ng level management
typed_api: Gain
sox_ng_syntax: gain [options] [gain-dB]
aliases: gain-db, gain_db
"#
    );
}

#[test]
fn plan_human_output_snapshot() {
    let spec = temp_path("auralis-cli-plan-snapshot", "toml");
    fs::write(
        &spec,
        r#"version = "auralis.graph/v1"
name = "episode-42"

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

[[sinks]]
id = "preview"
input = "voice_fx.audio"
path = "build/preview.wav"
"#,
    )
    .unwrap();

    let output = Command::new(env!("CARGO_BIN_EXE_auralis"))
        .args(["plan", spec.to_str().unwrap()])
        .output()
        .unwrap();

    fs::remove_file(&spec).unwrap();
    assert!(output.status.success(), "stderr: {}", stderr(&output));
    let stdout = stdout(&output).replace(spec.to_str().unwrap(), "<SPEC>");
    assert_eq!(
        stdout,
        r#"Pipeline: episode-42
Spec: <SPEC>

Inputs:
  voice  input/voice.wav

Outputs:
  wav  build/out.wav
  preview  build/preview.wav

Graph:
  sources: 1
  chains: 1
  nodes: 0
  sinks: 2
  expanded steps: 2
  streaming segments: 1
  whole-buffer barriers: 1
  fanout points: 1

Execution:
  read voice
  chain voice_fx <- voice.audio
    step cut
    step voice_fx/02-reverse
  write wav <- voice_fx.audio
  write preview <- voice_fx.audio

Segments:
  S1  voice.read -> cut
      mode: streaming
  B1  voice_fx/02-reverse
      mode: whole-buffer barrier
      reason: reverse requires a full-buffer materialization

Fanout:
  voice_fx.audio -> sink wav, sink preview
"#
    );
}

#[test]
fn explain_human_output_snapshot() {
    let spec = temp_path("auralis-cli-explain-snapshot", "toml");
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

    let output = Command::new(env!("CARGO_BIN_EXE_auralis"))
        .args(["explain", spec.to_str().unwrap(), "voice_fx"])
        .output()
        .unwrap();

    fs::remove_file(&spec).unwrap();
    assert!(output.status.success(), "stderr: {}", stderr(&output));
    assert_eq!(
        stdout(&output),
        r#"Node: voice_fx
Kind: chain
Input:
  voice.audio
Expanded steps:
  cut
  voice_fx/02-reverse
Execution mode:
  cut                      streaming
  voice_fx/02-reverse      whole-buffer barrier (reverse requires a full-buffer materialization)
Downstream:
  wav
"#
    );
}

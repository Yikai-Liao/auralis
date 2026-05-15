//! Integration tests for planning `auralis convert`.

mod support;

use support::*;

#[test]
fn plan_convert_lowers_format_recipe_to_graph_plan() {
    let input = temp_path("auralis-cli-plan-convert-input", "wav");
    let output = temp_path("auralis-cli-plan-convert-output", "flac");

    let command_output = Command::new(env!("CARGO_BIN_EXE_auralis"))
        .args([
            "plan",
            "convert",
            input.to_str().unwrap(),
            "-o",
            output.to_str().unwrap(),
            "--rate",
            "48000",
            "--channels",
            "2",
            "--guard",
            "--container",
            "flac",
        ])
        .output()
        .unwrap();

    assert!(
        command_output.status.success(),
        "stderr: {}",
        stderr(&command_output)
    );
    let stdout = stdout(&command_output);
    assert!(stdout.contains("Pipeline: convert"), "{stdout}");
    assert!(stdout.contains("Spec: command:convert"), "{stdout}");
    assert!(stdout.contains("step convert/01-channels-2"), "{stdout}");
    assert!(stdout.contains("step convert/02-rate-48000"), "{stdout}");
    assert!(stdout.contains("step convert/03-guard"), "{stdout}");
    assert!(
        stdout.contains("step convert/04-container-flac"),
        "{stdout}"
    );
}

#[test]
fn plan_convert_json_uses_graph_plan_contract() {
    let input = temp_path("auralis-cli-plan-convert-json-input", "wav");
    let output = temp_path("auralis-cli-plan-convert-json-output", "wav");

    let command_output = Command::new(env!("CARGO_BIN_EXE_auralis"))
        .args([
            "plan",
            "--json",
            "convert",
            input.to_str().unwrap(),
            "-o",
            output.to_str().unwrap(),
            "--sample",
            "pcm24",
        ])
        .output()
        .unwrap();

    assert!(
        command_output.status.success(),
        "stderr: {}",
        stderr(&command_output)
    );
    let plan: serde_json::Value = serde_json::from_str(&stdout(&command_output)).unwrap();
    assert_eq!(plan["pipeline"], "convert");
    assert_eq!(plan["spec"], "command:convert");
    assert_eq!(plan["graph"]["sources"], 1);
    assert_eq!(plan["graph"]["chains"], 1);
    assert_eq!(plan["execution"][1]["action"], "chain");
    assert_eq!(
        plan["execution"][1]["steps"],
        serde_json::json!(["convert/01-sample-pcm24"])
    );
}

#[test]
fn plan_convert_rejects_runtime_output_policy_errors() {
    let input = temp_path("auralis-cli-plan-convert-policy-input", "wav");
    let output = temp_path("auralis-cli-plan-convert-policy-output", "wav");

    let command_output = Command::new(env!("CARGO_BIN_EXE_auralis"))
        .args([
            "plan",
            "convert",
            input.to_str().unwrap(),
            "-o",
            output.to_str().unwrap(),
            "--container",
            "flac",
            "--sample",
            "pcm24",
        ])
        .output()
        .unwrap();

    assert!(!command_output.status.success());
    assert!(stderr(&command_output).contains("--sample is supported only for WAV output"));
}

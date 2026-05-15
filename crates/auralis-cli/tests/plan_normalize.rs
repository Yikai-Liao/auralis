//! Integration tests for planning `auralis normalize`.

mod support;

use support::*;

#[test]
fn plan_normalize_lowers_recipe_to_graph_plan() {
    let input = temp_path("auralis-cli-plan-normalize-input", "wav");
    let output = temp_path("auralis-cli-plan-normalize-output", "wav");

    let command_output = Command::new(env!("CARGO_BIN_EXE_auralis"))
        .args([
            "plan",
            "--json",
            "normalize",
            input.to_str().unwrap(),
            "-o",
            output.to_str().unwrap(),
            "--peak",
            "-3dBFS",
        ])
        .output()
        .unwrap();

    assert!(
        command_output.status.success(),
        "stderr: {}",
        stderr(&command_output)
    );
    let plan: serde_json::Value = serde_json::from_str(&stdout(&command_output)).unwrap();
    assert_eq!(plan["pipeline"], "normalize");
    assert_eq!(plan["spec"], "command:normalize");
    assert_eq!(plan["graph"]["sources"], 1);
    assert_eq!(plan["graph"]["chains"], 1);
    assert_eq!(plan["outputs"][0]["path"], output.to_str().unwrap());
    assert_eq!(plan["execution"][1]["action"], "chain");
    assert_eq!(
        plan["execution"][1]["steps"],
        serde_json::json!(["normalize/01-norm--3"])
    );
}

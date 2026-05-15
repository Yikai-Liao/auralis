//! Integration tests for `auralis plan render`.

mod support;

use support::*;

#[test]
fn plan_render_effects_file_expands_typed_effect_steps() {
    let input = temp_path("auralis-cli-plan-render-effects-file-input", "wav");
    let output = temp_path("auralis-cli-plan-render-effects-file-output", "wav");
    let effects_file = temp_path("auralis-cli-plan-render-effects-file", "effects");
    fs::write(&effects_file, "gain -3\nreverse\n").unwrap();

    let command_output = Command::new(env!("CARGO_BIN_EXE_auralis"))
        .args([
            "plan",
            "--json",
            "render",
            input.to_str().unwrap(),
            "-o",
            output.to_str().unwrap(),
            "--effects-file",
            effects_file.to_str().unwrap(),
        ])
        .output()
        .unwrap();

    assert!(
        command_output.status.success(),
        "stderr: {}",
        stderr(&command_output)
    );
    let plan: serde_json::Value = serde_json::from_str(&stdout(&command_output)).unwrap();
    assert_eq!(
        plan["execution"][1]["steps"],
        serde_json::json!(["render/01-gain--3", "render/02-reverse"])
    );

    fs::remove_file(effects_file).unwrap();
}

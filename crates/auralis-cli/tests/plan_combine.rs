//! Integration tests for multi-input recipe planning.

mod support;

use support::*;

#[test]
fn plan_combine_recipes_use_graph_plan_contract() {
    for (name, op) in [
        ("mix", "combine.mix"),
        ("concat", "combine.concatenate"),
        ("mix-power", "combine.mix-power"),
        ("merge", "combine.merge"),
        ("multiply", "combine.multiply"),
    ] {
        let first = temp_path(&format!("auralis-cli-plan-{name}-first"), "wav");
        let second = temp_path(&format!("auralis-cli-plan-{name}-second"), "wav");
        let output = temp_path(&format!("auralis-cli-plan-{name}-output"), "wav");

        let command_output = Command::new(env!("CARGO_BIN_EXE_auralis"))
            .args([
                "plan",
                "--json",
                name,
                first.to_str().unwrap(),
                second.to_str().unwrap(),
                "-o",
                output.to_str().unwrap(),
            ])
            .output()
            .unwrap();

        assert!(
            command_output.status.success(),
            "stderr: {}",
            stderr(&command_output)
        );
        let plan: serde_json::Value = serde_json::from_str(&stdout(&command_output)).unwrap();
        assert_eq!(plan["pipeline"], name);
        assert_eq!(plan["spec"], format!("command:{name}"));
        assert_eq!(plan["graph"]["sources"], 2);
        assert_eq!(plan["graph"]["chains"], 0);
        assert_eq!(plan["graph"]["nodes"], 1);
        assert_eq!(plan["execution"][2]["action"], "node");
        assert_eq!(plan["execution"][2]["id"], "combine");
        assert_eq!(plan["execution"][2]["label"], op);
        assert_eq!(plan["execution"][3]["input"], "combine.audio");
    }
}

//! Integration tests for `render`, `ops`, and `check`.

mod support;

use support::*;

#[test]
fn render_fx_chain_applies_ordered_effects() {
    let input = temp_path("auralis-cli-render-fx-input", "wav");
    let render_output = temp_path("auralis-cli-render-fx-output", "wav");
    write_pcm16_wav(&input, 1, &[-16_384, -8_192, 0, 8_192, 16_384]);

    let render = Command::new(env!("CARGO_BIN_EXE_auralis"))
        .args([
            "render",
            input.to_str().unwrap(),
            "-o",
            render_output.to_str().unwrap(),
            "--fx",
            "gain -6 reverse",
        ])
        .output()
        .unwrap();

    assert!(render.status.success(), "stderr: {}", stderr(&render));
    assert_eq!(
        read_pcm16_wav(&render_output),
        (1, vec![8211, 4106, 0, -4106, -8211])
    );

    fs::remove_file(input).unwrap();
    fs::remove_file(render_output).unwrap();
}

#[test]
fn render_chain_output_matches_repeated_fx_chain() {
    let input = temp_path("auralis-cli-render-chain-input", "wav");
    let chain_output = temp_path("auralis-cli-render-chain-output", "wav");
    let fx_output = temp_path("auralis-cli-render-chain-fx-output", "wav");
    write_pcm16_wav(&input, 1, &[-16_384, -8_192, 0, 8_192, 16_384]);

    let chain = Command::new(env!("CARGO_BIN_EXE_auralis"))
        .args([
            "render",
            input.to_str().unwrap(),
            "-o",
            chain_output.to_str().unwrap(),
            "--chain",
            "gain -6 | reverse",
        ])
        .output()
        .unwrap();
    let fx = Command::new(env!("CARGO_BIN_EXE_auralis"))
        .args([
            "render",
            input.to_str().unwrap(),
            "-o",
            fx_output.to_str().unwrap(),
            "--fx",
            "gain -6",
            "--fx",
            "reverse",
        ])
        .output()
        .unwrap();

    assert!(chain.status.success(), "stderr: {}", stderr(&chain));
    assert!(fx.status.success(), "stderr: {}", stderr(&fx));
    assert_eq!(read_pcm16_wav(&chain_output), read_pcm16_wav(&fx_output));

    fs::remove_file(input).unwrap();
    fs::remove_file(chain_output).unwrap();
    fs::remove_file(fx_output).unwrap();
}

#[test]
fn pipe_output_matches_render_chain() {
    let input = temp_path("auralis-cli-pipe-input", "wav");
    let pipe_output = temp_path("auralis-cli-pipe-output", "wav");
    let render_output = temp_path("auralis-cli-pipe-render-output", "wav");
    write_pcm16_wav(&input, 1, &[-16_384, -8_192, 0, 8_192, 16_384]);

    let pipe = Command::new(env!("CARGO_BIN_EXE_auralis"))
        .args([
            "pipe",
            input.to_str().unwrap(),
            "gain -6 | reverse",
            "-o",
            pipe_output.to_str().unwrap(),
        ])
        .output()
        .unwrap();
    let render = Command::new(env!("CARGO_BIN_EXE_auralis"))
        .args([
            "render",
            input.to_str().unwrap(),
            "-o",
            render_output.to_str().unwrap(),
            "--chain",
            "gain -6 | reverse",
        ])
        .output()
        .unwrap();

    assert!(pipe.status.success(), "stderr: {}", stderr(&pipe));
    assert!(render.status.success(), "stderr: {}", stderr(&render));
    assert_eq!(read_pcm16_wav(&pipe_output), read_pcm16_wav(&render_output));

    fs::remove_file(input).unwrap();
    fs::remove_file(pipe_output).unwrap();
    fs::remove_file(render_output).unwrap();
}

#[test]
fn render_mix_combines_inputs_before_chain() {
    let first = temp_path("auralis-cli-render-mix-first", "wav");
    let second = temp_path("auralis-cli-render-mix-second", "wav");
    let output = temp_path("auralis-cli-render-mix-output", "wav");
    write_pcm16_wav(&first, 1, &[1000, -1000]);
    write_pcm16_wav(&second, 1, &[3000, 1000]);

    let command_output = Command::new(env!("CARGO_BIN_EXE_auralis"))
        .args([
            "render",
            first.to_str().unwrap(),
            "-o",
            output.to_str().unwrap(),
            "--combine",
            "mix",
            "--input",
            second.to_str().unwrap(),
            "--chain",
            "reverse",
        ])
        .output()
        .unwrap();

    assert!(
        command_output.status.success(),
        "stderr: {}",
        stderr(&command_output)
    );
    assert_eq!(read_pcm16_wav(&output), (1, vec![0, 2000]));

    fs::remove_file(first).unwrap();
    fs::remove_file(second).unwrap();
    fs::remove_file(output).unwrap();
}

#[test]
fn plan_render_lowers_linear_command_to_graph_plan() {
    let input = temp_path("auralis-cli-plan-render-input", "wav");
    let second = temp_path("auralis-cli-plan-render-second", "wav");
    let output = temp_path("auralis-cli-plan-render-output", "flac");

    let command_output = Command::new(env!("CARGO_BIN_EXE_auralis"))
        .args([
            "plan",
            "render",
            input.to_str().unwrap(),
            "-o",
            output.to_str().unwrap(),
            "--combine",
            "mix",
            "--input",
            second.to_str().unwrap(),
            "--chain",
            "gain -3 | reverse",
            "--container",
            "flac",
            "--guard",
        ])
        .output()
        .unwrap();

    assert!(
        command_output.status.success(),
        "stderr: {}",
        stderr(&command_output)
    );
    let stdout = stdout(&command_output);
    assert!(stdout.contains("Pipeline: render"), "{stdout}");
    assert!(stdout.contains("Spec: command:render"), "{stdout}");
    assert!(stdout.contains("node combine (combine.mix)"), "{stdout}");
    assert!(stdout.contains("step render/01-gain--3"), "{stdout}");
    assert!(stdout.contains("step render/02-reverse"), "{stdout}");
    assert!(stdout.contains("step render/03-guard"), "{stdout}");
    assert!(stdout.contains("step render/04-container-flac"), "{stdout}");
    assert!(stdout.contains("whole-buffer barrier"), "{stdout}");
}

#[test]
fn plan_render_json_uses_graph_plan_contract() {
    let input = temp_path("auralis-cli-plan-render-json-input", "wav");
    let output = temp_path("auralis-cli-plan-render-json-output", "wav");

    let command_output = Command::new(env!("CARGO_BIN_EXE_auralis"))
        .args([
            "plan",
            "--json",
            "render",
            input.to_str().unwrap(),
            "-o",
            output.to_str().unwrap(),
            "--fx",
            "gain -3",
        ])
        .output()
        .unwrap();

    assert!(
        command_output.status.success(),
        "stderr: {}",
        stderr(&command_output)
    );
    let plan: serde_json::Value = serde_json::from_str(&stdout(&command_output)).unwrap();
    assert_eq!(plan["pipeline"], "render");
    assert_eq!(plan["spec"], "command:render");
    assert_eq!(plan["graph"]["sources"], 1);
    assert_eq!(plan["graph"]["chains"], 1);
    assert_eq!(plan["execution"][1]["action"], "chain");
    assert_eq!(
        plan["execution"][1]["steps"],
        serde_json::json!(["render/01-gain--3"])
    );
}

#[test]
fn plan_pipe_lowers_compact_command_to_graph_plan() {
    let input = temp_path("auralis-cli-plan-pipe-input", "wav");
    let output = temp_path("auralis-cli-plan-pipe-output", "wav");

    let command_output = Command::new(env!("CARGO_BIN_EXE_auralis"))
        .args([
            "plan",
            "pipe",
            input.to_str().unwrap(),
            "gain -3 | reverse",
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
    let stdout = stdout(&command_output);
    assert!(stdout.contains("Pipeline: pipe"), "{stdout}");
    assert!(stdout.contains("Spec: command:pipe"), "{stdout}");
    assert!(stdout.contains("step pipe/01-gain--3"), "{stdout}");
    assert!(stdout.contains("step pipe/02-reverse"), "{stdout}");
    assert!(stdout.contains("whole-buffer barrier"), "{stdout}");
}

#[test]
fn plan_pipe_json_uses_graph_plan_contract() {
    let input = temp_path("auralis-cli-plan-pipe-json-input", "wav");
    let output = temp_path("auralis-cli-plan-pipe-json-output", "wav");

    let command_output = Command::new(env!("CARGO_BIN_EXE_auralis"))
        .args([
            "plan",
            "--json",
            "pipe",
            input.to_str().unwrap(),
            "gain -3",
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
    assert_eq!(plan["pipeline"], "pipe");
    assert_eq!(plan["spec"], "command:pipe");
    assert_eq!(plan["graph"]["sources"], 1);
    assert_eq!(plan["graph"]["chains"], 1);
    assert_eq!(plan["execution"][1]["action"], "chain");
    assert_eq!(
        plan["execution"][1]["steps"],
        serde_json::json!(["pipe/01-gain--3"])
    );
}

#[test]
fn plan_gain_lowers_recipe_command_to_graph_plan() {
    let input = temp_path("auralis-cli-plan-gain-input", "wav");
    let output = temp_path("auralis-cli-plan-gain-output", "wav");

    let command_output = Command::new(env!("CARGO_BIN_EXE_auralis"))
        .args([
            "plan",
            "gain",
            input.to_str().unwrap(),
            "-3",
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
    let stdout = stdout(&command_output);
    assert!(stdout.contains("Pipeline: gain"), "{stdout}");
    assert!(stdout.contains("Spec: command:gain"), "{stdout}");
    assert!(stdout.contains("step gain/01-gain--3"), "{stdout}");
}

#[test]
fn plan_reverse_json_uses_graph_plan_contract() {
    let input = temp_path("auralis-cli-plan-reverse-json-input", "wav");
    let output = temp_path("auralis-cli-plan-reverse-json-output", "wav");

    let command_output = Command::new(env!("CARGO_BIN_EXE_auralis"))
        .args([
            "plan",
            "--json",
            "reverse",
            input.to_str().unwrap(),
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
    assert_eq!(plan["pipeline"], "reverse");
    assert_eq!(plan["spec"], "command:reverse");
    assert_eq!(plan["graph"]["sources"], 1);
    assert_eq!(plan["graph"]["chains"], 1);
    assert_eq!(plan["graph"]["whole_buffer_barriers"], 1);
    assert_eq!(
        plan["execution"][1]["steps"],
        serde_json::json!(["reverse/01-reverse"])
    );
}

#[test]
fn plan_additional_simple_recipes_lower_to_graph_plan() {
    for (name, args, step) in [
        ("trim", vec!["10..20"], "trim/01-trim-1020"),
        ("deemph", Vec::new(), "deemph/01-deemph"),
        ("earwax", Vec::new(), "earwax/01-earwax"),
    ] {
        let input = temp_path(&format!("auralis-cli-plan-{name}-input"), "wav");
        let output = temp_path(&format!("auralis-cli-plan-{name}-output"), "wav");
        let mut command_args = vec!["plan", name, input.to_str().unwrap()];
        command_args.extend(args);
        command_args.extend(["-o", output.to_str().unwrap()]);

        let command_output = Command::new(env!("CARGO_BIN_EXE_auralis"))
            .args(command_args)
            .output()
            .unwrap();

        assert!(
            command_output.status.success(),
            "stderr: {}",
            stderr(&command_output)
        );
        let stdout = stdout(&command_output);
        assert!(stdout.contains(&format!("Pipeline: {name}")), "{stdout}");
        assert!(
            stdout.contains(&format!("Spec: command:{name}")),
            "{stdout}"
        );
        assert!(stdout.contains(step), "{stdout}");
    }
}

#[test]
fn plan_echo_recipes_json_use_graph_plan_contract() {
    for name in ["echo", "echos"] {
        let input = temp_path(&format!("auralis-cli-plan-{name}-json-input"), "wav");
        let output = temp_path(&format!("auralis-cli-plan-{name}-json-output"), "wav");

        let command_output = Command::new(env!("CARGO_BIN_EXE_auralis"))
            .args([
                "plan",
                "--json",
                name,
                input.to_str().unwrap(),
                "--gain-in",
                "0.7",
                "--gain-out",
                "0.8",
                "--tap",
                "40,0.5",
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
        assert_eq!(plan["graph"]["sources"], 1);
        assert_eq!(plan["graph"]["chains"], 1);
        assert_eq!(
            plan["execution"][1]["steps"],
            serde_json::json!([format!("{name}/01-{name}-07-08-40-05")])
        );
    }
}

#[test]
fn plan_modulation_recipes_use_graph_plan_contract() {
    let cases: [(&str, &[&str]); 3] = [
        (
            "chorus",
            &[
                "--gain-in",
                "0.5",
                "--gain-out",
                "0.8",
                "--interpolation",
                "linear",
                "--wave",
                "triangle",
                "--stage",
                "40,0.4,0.5,2,sine",
            ],
        ),
        (
            "flanger",
            &[
                "--delay",
                "1",
                "--depth",
                "3",
                "--regen",
                "4",
                "--width",
                "70",
                "--speed",
                "0.6",
                "--wave",
                "triangle",
                "--phase",
                "20",
                "--interpolation",
                "linear",
            ],
        ),
        (
            "phaser",
            &[
                "--gain-in",
                "0.4",
                "--gain-out",
                "0.7",
                "--delay",
                "3",
                "--regen",
                "0.5",
                "--speed",
                "0.6",
                "--wave",
                "triangle",
                "--interpolation",
                "linear",
            ],
        ),
    ];

    for (name, args) in cases {
        let input = temp_path(&format!("auralis-cli-plan-{name}-input"), "wav");
        let output = temp_path(&format!("auralis-cli-plan-{name}-output"), "wav");
        let mut command_args = vec!["plan", "--json", name, input.to_str().unwrap()];
        command_args.extend(args);
        command_args.extend(["-o", output.to_str().unwrap()]);

        let command_output = Command::new(env!("CARGO_BIN_EXE_auralis"))
            .args(command_args)
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
        assert_eq!(plan["graph"]["sources"], 1);
        assert_eq!(plan["graph"]["chains"], 1);
        assert!(
            plan["execution"][1]["steps"][0]
                .as_str()
                .unwrap()
                .starts_with(&format!("{name}/01-{name}"))
        );
    }
}

#[test]
fn plan_scalar_and_distortion_recipes_use_graph_plan_contract() {
    let cases: [(&str, &[&str], &str); 11] = [
        ("oops", &[], "oops/01-oops"),
        ("riaa", &[], "riaa/01-riaa"),
        ("swap", &[], "swap/01-swap"),
        ("contrast", &["--amount", "61"], "contrast/01-contrast-61"),
        (
            "overdrive",
            &["--gain", "12", "--color", "7"],
            "overdrive/01-overdrive-12-7",
        ),
        (
            "saturation",
            &[
                "--type",
                "tanh",
                "--blend",
                "0.7",
                "--offset",
                "0.1",
                "--parameter",
                "2",
            ],
            "saturation/01-saturation-tanh-07-01-2",
        ),
        (
            "dcshift",
            &["0.1", "--limiter-gain", "0.5"],
            "dcshift/01-dcshift-01-05",
        ),
        (
            "vol",
            &["2", "--type", "amplitude", "--limiter-gain", "0.3"],
            "vol/01-vol-2-amplitude-03",
        ),
        (
            "softvol",
            &["--volume", "0.8", "--double-time", "2", "--headroom", "3"],
            "softvol/01-softvol-08-2-3",
        ),
        (
            "tremolo",
            &["5", "--depth", "50"],
            "tremolo/01-tremolo-5-50",
        ),
        ("speed", &["1.2"], "speed/01-speed-12"),
    ];

    for (name, args, step) in cases {
        let input = temp_path(&format!("auralis-cli-plan-{name}-input"), "wav");
        let output = temp_path(&format!("auralis-cli-plan-{name}-output"), "wav");
        let mut command_args = vec!["plan", name, input.to_str().unwrap()];
        command_args.extend(args);
        command_args.extend(["-o", output.to_str().unwrap()]);

        let command_output = Command::new(env!("CARGO_BIN_EXE_auralis"))
            .args(command_args)
            .output()
            .unwrap();

        assert!(
            command_output.status.success(),
            "stderr: {}",
            stderr(&command_output)
        );
        let stdout = stdout(&command_output);
        assert!(stdout.contains(&format!("Pipeline: {name}")), "{stdout}");
        assert!(
            stdout.contains(&format!("Spec: command:{name}")),
            "{stdout}"
        );
        assert!(stdout.contains(step), "{stdout}");
    }
}

#[test]
fn check_fx_reports_ok_summary() {
    let command_output = Command::new(env!("CARGO_BIN_EXE_auralis"))
        .args(["check", "--fx", "gain -3", "--fx", "reverse"])
        .output()
        .unwrap();

    assert!(
        command_output.status.success(),
        "stderr: {}",
        stderr(&command_output)
    );
    let stdout = stdout(&command_output);
    assert!(stdout.contains("status: ok"), "{stdout}");
    assert!(stdout.contains("commands: 2"), "{stdout}");
    assert!(stdout.contains("boundaries: 0"), "{stdout}");
}

#[test]
fn check_fx_accepts_documented_highpass_named_parameters() {
    let command_output = Command::new(env!("CARGO_BIN_EXE_auralis"))
        .args(["check", "--fx", "filter.highpass cutoff=1000Hz q=0.707"])
        .output()
        .unwrap();

    assert!(
        command_output.status.success(),
        "stderr: {}",
        stderr(&command_output)
    );
    let stdout = stdout(&command_output);
    assert!(stdout.contains("status: ok"), "{stdout}");
    assert!(stdout.contains("commands: 1"), "{stdout}");
}

#[test]
fn check_fx_accepts_documented_trim_range() {
    let command_output = Command::new(env!("CARGO_BIN_EXE_auralis"))
        .args(["check", "--fx", "trim 10s..30s"])
        .output()
        .unwrap();

    assert!(
        command_output.status.success(),
        "stderr: {}",
        stderr(&command_output)
    );
    let stdout = stdout(&command_output);
    assert!(stdout.contains("status: ok"), "{stdout}");
    assert!(stdout.contains("commands: 1"), "{stdout}");
}

#[test]
fn check_chain_reports_ok_summary() {
    let command_output = Command::new(env!("CARGO_BIN_EXE_auralis"))
        .args(["check", "--chain", "gain -3 | reverse"])
        .output()
        .unwrap();

    assert!(
        command_output.status.success(),
        "stderr: {}",
        stderr(&command_output)
    );
    let stdout = stdout(&command_output);
    assert!(stdout.contains("status: ok"), "{stdout}");
    assert!(stdout.contains("commands: 2"), "{stdout}");
    assert!(stdout.contains("boundaries: 0"), "{stdout}");
}

#[test]
fn check_requires_fx_or_effects_file() {
    let command_output = Command::new(env!("CARGO_BIN_EXE_auralis"))
        .args(["check"])
        .output()
        .unwrap();

    assert!(!command_output.status.success());
    let stderr = stderr(&command_output);
    assert!(
        stderr.contains("error: one of SPEC, --fx, --chain, or --effects-file is required"),
        "{stderr}"
    );
}

#[test]
fn check_rejects_mixed_fx_and_chain() {
    let command_output = Command::new(env!("CARGO_BIN_EXE_auralis"))
        .args(["check", "--fx", "gain -3", "--chain", "reverse"])
        .output()
        .unwrap();

    assert!(!command_output.status.success());
    let stderr = stderr(&command_output);
    assert!(
        stderr.contains("error: --fx, --chain, and --effects-file are mutually exclusive"),
        "{stderr}"
    );
}

#[test]
fn check_rejects_unmatched_fx_quoting() {
    let command_output = Command::new(env!("CARGO_BIN_EXE_auralis"))
        .args(["check", "--fx", "'gain -3"])
        .output()
        .unwrap();

    assert!(!command_output.status.success());
    let stderr = stderr(&command_output);
    assert!(
        stderr.contains("error: effect string `'gain -3` contains unmatched shell quoting"),
        "{stderr}"
    );
}

#[test]
fn ops_lists_gain_and_reverse() {
    let command_output = Command::new(env!("CARGO_BIN_EXE_auralis"))
        .args(["ops"])
        .output()
        .unwrap();

    assert!(
        command_output.status.success(),
        "stderr: {}",
        stderr(&command_output)
    );
    let stdout = stdout(&command_output);
    assert!(stdout.contains("gain"), "{stdout}");
    assert!(stdout.contains("reverse"), "{stdout}");
}

#[test]
fn ops_effect_alias_resolves_to_descriptor() {
    let command_output = Command::new(env!("CARGO_BIN_EXE_auralis"))
        .args(["ops", "dc-shift"])
        .output()
        .unwrap();

    assert!(
        command_output.status.success(),
        "stderr: {}",
        stderr(&command_output)
    );
    let stdout = stdout(&command_output);
    assert!(stdout.contains("name: dcshift"), "{stdout}");
    assert!(stdout.contains("typed_api: DcShift"), "{stdout}");
    assert!(stdout.contains("aliases: dc-shift"), "{stdout}");
}

#[test]
fn ops_schema_json_lists_effect_registry_entries() {
    let command_output = Command::new(env!("CARGO_BIN_EXE_auralis"))
        .args(["ops", "--schema", "json"])
        .output()
        .unwrap();

    assert!(
        command_output.status.success(),
        "stderr: {}",
        stderr(&command_output)
    );
    let stdout = stdout(&command_output);
    let schema: serde_json::Value = serde_json::from_str(&stdout).unwrap();
    assert!(schema.is_array(), "{stdout}");
    let gain = schema
        .as_array()
        .unwrap()
        .iter()
        .find(|entry| entry["name"] == "gain")
        .unwrap();
    assert_eq!(gain["typed_api"], "Gain");
    assert_eq!(gain["sox_ng_syntax"], "gain [options] [gain-dB]");
}

#[test]
fn ops_schema_json_for_alias_resolves_single_descriptor() {
    let command_output = Command::new(env!("CARGO_BIN_EXE_auralis"))
        .args(["ops", "dc-shift", "--schema", "json"])
        .output()
        .unwrap();

    assert!(
        command_output.status.success(),
        "stderr: {}",
        stderr(&command_output)
    );
    let stdout = stdout(&command_output);
    let schema: serde_json::Value = serde_json::from_str(&stdout).unwrap();
    assert_eq!(schema["name"], "dcshift");
    assert_eq!(schema["typed_api"], "DcShift");
    assert_eq!(
        schema["aliases"],
        serde_json::json!(["dc-shift", "dc_shift"])
    );
}

//! CLI integration tests for diagnostics and help text.

mod support;

use support::*;

#[test]
fn top_level_help_documents_modern_run_subcommand() {
    let command_output = Command::new(env!("CARGO_BIN_EXE_auralis"))
        .args(["--help"])
        .output()
        .unwrap();

    assert!(command_output.status.success());
    let stdout = stdout(&command_output);
    assert!(stdout.contains("render"), "{stdout}");
    assert!(stdout.contains("convert"), "{stdout}");
    assert!(stdout.contains("run"), "{stdout}");
}

#[test]
fn run_rejects_legacy_positional_audio_shape() {
    let command_output = Command::new(env!("CARGO_BIN_EXE_auralis"))
        .args(["run", "input.wav", "output.wav", "reverse"])
        .output()
        .unwrap();

    assert!(!command_output.status.success());
    let stderr = stderr(&command_output);
    assert!(
        stderr.contains("unexpected argument 'output.wav'"),
        "{stderr}"
    );
}

#[test]
fn render_invalid_gain_argument_returns_clear_error() {
    let input = temp_path("auralis-cli-render-invalid-gain-input", "wav");
    let output = temp_path("auralis-cli-render-invalid-gain-output", "wav");
    write_pcm16_wav(&input, 1, &[0]);

    let command_output = Command::new(env!("CARGO_BIN_EXE_auralis"))
        .args([
            "render",
            input.to_str().unwrap(),
            "-o",
            output.to_str().unwrap(),
            "--fx",
            "gain NaN",
        ])
        .output()
        .unwrap();

    fs::remove_file(input).unwrap();
    let _ = fs::remove_file(output);
    assert!(!command_output.status.success());
    let stderr = stderr(&command_output);
    assert!(stderr.contains("decibels must be finite"), "{stderr}");
}

#[test]
fn render_invalid_dc_shift_argument_returns_clear_error() {
    let input = temp_path("auralis-cli-render-invalid-dc-shift-input", "wav");
    let output = temp_path("auralis-cli-render-invalid-dc-shift-output", "wav");
    write_pcm16_wav(&input, 1, &[0]);

    let command_output = Command::new(env!("CARGO_BIN_EXE_auralis"))
        .args([
            "render",
            input.to_str().unwrap(),
            "-o",
            output.to_str().unwrap(),
            "--fx",
            "dcshift 2.1",
        ])
        .output()
        .unwrap();

    fs::remove_file(input).unwrap();
    let _ = fs::remove_file(output);
    assert!(!command_output.status.success());
    let stderr = stderr(&command_output);
    assert!(
        stderr.contains("dc shift must be finite and in the range -2.0..=2.0"),
        "{stderr}"
    );
}

#[test]
fn render_invalid_trim_range_returns_clear_error() {
    let input = temp_path("auralis-cli-render-invalid-trim-input", "wav");
    let output = temp_path("auralis-cli-render-invalid-trim-output", "wav");
    write_pcm16_wav(&input, 1, &[0, 1, 2]);

    let command_output = Command::new(env!("CARGO_BIN_EXE_auralis"))
        .args([
            "render",
            input.to_str().unwrap(),
            "-o",
            output.to_str().unwrap(),
            "--fx",
            "trim 3 =1",
        ])
        .output()
        .unwrap();

    fs::remove_file(input).unwrap();
    let _ = fs::remove_file(output);
    assert!(!command_output.status.success());
    let stderr = stderr(&command_output);
    assert!(
        stderr.contains("trim start frame must be less than or equal"),
        "{stderr}"
    );
}

#[test]
fn check_missing_trim_position_returns_clear_error() {
    let command_output = Command::new(env!("CARGO_BIN_EXE_auralis"))
        .args(["check", "--fx", "trim"])
        .output()
        .unwrap();

    assert!(!command_output.status.success());
    let stderr = stderr(&command_output);
    assert!(
        stderr.contains("error: effect chain command 0 (`trim`) failed to parse: effect `trim` requires argument `position`"),
        "{stderr}"
    );
}

#[test]
fn check_graph_spec_reports_summary_for_valid_spec() {
    let spec = temp_path("auralis-cli-check-spec-valid", "toml");
    fs::write(
        &spec,
        r#"version = "auralis.graph/v1"

[[sources]]
id = "voice"
path = "input/voice.wav"

[[chains]]
id = "voice_clean"
input = "voice.audio"
steps = [
  { op = "trim" },
  { op = "filter.highpass" },
]

[[sinks]]
id = "wav"
input = "voice_clean.audio"
path = "build/out.wav"
"#,
    )
    .unwrap();

    let command_output = Command::new(env!("CARGO_BIN_EXE_auralis"))
        .args(["check", spec.to_str().unwrap()])
        .output()
        .unwrap();

    fs::remove_file(spec).unwrap();
    assert!(command_output.status.success());
    let stdout = stdout(&command_output);
    assert!(stdout.contains("status: ok"), "{stdout}");
    assert!(stdout.contains("sources: 1"), "{stdout}");
    assert!(stdout.contains("chains: 1"), "{stdout}");
    assert!(stdout.contains("sinks: 1"), "{stdout}");
    assert!(stdout.contains("expanded_steps: 2"), "{stdout}");
}

#[test]
fn check_graph_spec_reports_unknown_input_port() {
    let spec = temp_path("auralis-cli-check-spec-unknown-input", "toml");
    fs::write(
        &spec,
        r#"version = "auralis.graph/v1"

[[sources]]
id = "voice"
path = "input/voice.wav"

[[sinks]]
id = "wav"
input = "voic.audio"
path = "build/out.wav"
"#,
    )
    .unwrap();

    let command_output = Command::new(env!("CARGO_BIN_EXE_auralis"))
        .args(["check", spec.to_str().unwrap()])
        .output()
        .unwrap();

    fs::remove_file(spec).unwrap();
    assert!(!command_output.status.success());
    let stderr = stderr(&command_output);
    assert!(
        stderr.contains("unknown input port `voic.audio`"),
        "{stderr}"
    );
    assert!(stderr.contains("voice.audio"), "{stderr}");
}

#[test]
fn check_graph_spec_reports_duplicate_graph_ids() {
    let spec = temp_path("auralis-cli-check-spec-duplicate-id", "toml");
    fs::write(
        &spec,
        r#"version = "auralis.graph/v1"

[[sources]]
id = "voice"
path = "input/voice.wav"

[[sinks]]
id = "voice"
input = "voice.audio"
path = "build/out.wav"
"#,
    )
    .unwrap();

    let command_output = Command::new(env!("CARGO_BIN_EXE_auralis"))
        .args(["check", spec.to_str().unwrap()])
        .output()
        .unwrap();

    fs::remove_file(spec).unwrap();
    assert!(!command_output.status.success());
    let stderr = stderr(&command_output);
    assert!(
        stderr.contains("duplicate graph id `voice` is used by both a source and a sink"),
        "{stderr}"
    );
}

#[test]
fn plan_graph_spec_reports_preview_for_valid_spec() {
    let spec = temp_path("auralis-cli-plan-spec-valid", "toml");
    fs::write(
        &spec,
        r#"version = "auralis.graph/v1"
name = "episode-42"

[[sources]]
id = "voice"
path = "input/voice.wav"

[[chains]]
id = "voice_clean"
input = "voice.audio"
steps = [
  { id = "cut", op = "trim" },
  { op = "filter.highpass" },
]

[[nodes]]
id = "master"
input = "voice_clean.audio"

[[sinks]]
id = "wav"
input = "master.audio"
path = "build/out.wav"
"#,
    )
    .unwrap();

    let command_output = Command::new(env!("CARGO_BIN_EXE_auralis"))
        .args(["plan", spec.to_str().unwrap()])
        .output()
        .unwrap();

    fs::remove_file(spec).unwrap();
    assert!(command_output.status.success());
    let stdout = stdout(&command_output);
    assert!(stdout.contains("Pipeline: episode-42"), "{stdout}");
    assert!(stdout.contains("Inputs:"), "{stdout}");
    assert!(stdout.contains("voice  input/voice.wav"), "{stdout}");
    assert!(stdout.contains("Outputs:"), "{stdout}");
    assert!(stdout.contains("wav  build/out.wav"), "{stdout}");
    assert!(stdout.contains("expanded steps: 2"), "{stdout}");
    assert!(
        stdout.contains("chain voice_clean <- voice.audio"),
        "{stdout}"
    );
    assert!(stdout.contains("step cut"), "{stdout}");
    assert!(
        stdout.contains("step voice_clean/02-filter.highpass"),
        "{stdout}"
    );
    assert!(stdout.contains("node master"), "{stdout}");
    assert!(stdout.contains("write wav <- master.audio"), "{stdout}");
}

#[test]
fn plan_graph_spec_reuses_validation_errors() {
    let spec = temp_path("auralis-cli-plan-spec-unknown-input", "toml");
    fs::write(
        &spec,
        r#"version = "auralis.graph/v1"

[[sources]]
id = "voice"
path = "input/voice.wav"

[[sinks]]
id = "wav"
input = "missing.audio"
path = "build/out.wav"
"#,
    )
    .unwrap();

    let command_output = Command::new(env!("CARGO_BIN_EXE_auralis"))
        .args(["plan", spec.to_str().unwrap()])
        .output()
        .unwrap();

    fs::remove_file(spec).unwrap();
    assert!(!command_output.status.success());
    let stderr = stderr(&command_output);
    assert!(
        stderr.contains("unknown input port `missing.audio`"),
        "{stderr}"
    );
    assert!(stderr.contains("voice.audio"), "{stderr}");
}

#[test]
fn check_accepts_documented_named_fade_effect_syntax() {
    let command_output = Command::new(env!("CARGO_BIN_EXE_auralis"))
        .args(["check", "--fx", "fade out=3f curve=linear"])
        .output()
        .unwrap();

    assert!(
        command_output.status.success(),
        "{}",
        stderr(&command_output)
    );
    let stdout = stdout(&command_output);
    assert!(stdout.contains("status: ok"), "{stdout}");
    assert!(stdout.contains("commands: 1"), "{stdout}");
}

#[test]
fn run_graph_spec_writes_direct_source_to_sink_output() {
    let spec = temp_path("auralis-cli-run-spec-direct", "toml");
    let input = temp_path("auralis-cli-run-spec-direct-input", "wav");
    let output = temp_path("auralis-cli-run-spec-direct-output", "wav");
    write_pcm16_wav(&input, 1, &[1000, -2000, 3000]);
    fs::write(
        &spec,
        format!(
            r#"version = "auralis.graph/v1"

[[sources]]
id = "voice"
path = "{}"

[[sinks]]
id = "wav"
input = "voice.audio"
path = "{}"
"#,
            input.display(),
            output.display()
        ),
    )
    .unwrap();

    let command_output = Command::new(env!("CARGO_BIN_EXE_auralis"))
        .args(["run", spec.to_str().unwrap()])
        .output()
        .unwrap();

    fs::remove_file(spec).unwrap();
    fs::remove_file(input).unwrap();
    assert!(
        command_output.status.success(),
        "{}",
        stderr(&command_output)
    );
    let stdout = stdout(&command_output);
    assert!(stdout.contains("wrote"), "{stdout}");
    assert_eq!(read_pcm16_wav(&output), (1, vec![1000, -2000, 3000]));
    fs::remove_file(output).unwrap();
}

#[test]
fn run_graph_spec_applies_linear_chain_to_sink_output() {
    let spec = temp_path("auralis-cli-run-spec-chain", "toml");
    let input = temp_path("auralis-cli-run-spec-chain-input", "wav");
    let output = temp_path("auralis-cli-run-spec-chain-output", "wav");
    write_pcm16_wav(&input, 1, &[1000, -2000, 3000]);
    fs::write(
        &spec,
        format!(
            r#"version = "auralis.graph/v1"

[[sources]]
id = "voice"
path = "{}"

[[chains]]
id = "voice_reversed"
input = "voice.audio"
steps = [
  {{ op = "reverse" }},
]

[[sinks]]
id = "wav"
input = "voice_reversed.audio"
path = "{}"
"#,
            input.display(),
            output.display()
        ),
    )
    .unwrap();

    let command_output = Command::new(env!("CARGO_BIN_EXE_auralis"))
        .args(["run", spec.to_str().unwrap()])
        .output()
        .unwrap();

    fs::remove_file(spec).unwrap();
    fs::remove_file(input).unwrap();
    assert!(
        command_output.status.success(),
        "{}",
        stderr(&command_output)
    );
    let stdout = stdout(&command_output);
    assert!(stdout.contains("wrote"), "{stdout}");
    assert_eq!(read_pcm16_wav(&output), (1, vec![3000, -2000, 1000]));
    fs::remove_file(output).unwrap();
}

#[test]
fn run_graph_spec_applies_gain_chain_step_db_param() {
    let spec = temp_path("auralis-cli-run-spec-gain-db-param", "toml");
    let input = temp_path("auralis-cli-run-spec-gain-db-param-input", "wav");
    let graph_output = temp_path("auralis-cli-run-spec-gain-db-param-output", "wav");
    let render_output = temp_path("auralis-cli-run-spec-gain-db-param-render-output", "wav");
    write_pcm16_wav(&input, 1, &[-16_384, -8_192, 0, 8_192, 16_384]);
    fs::write(
        &spec,
        format!(
            r#"version = "auralis.graph/v1"

[[sources]]
id = "voice"
path = "{}"

[[chains]]
id = "voice_quiet"
input = "voice.audio"
steps = [
  {{ op = "gain", by = "-6dB" }},
]

[[sinks]]
id = "wav"
input = "voice_quiet.audio"
path = "{}"
"#,
            input.display(),
            graph_output.display()
        ),
    )
    .unwrap();

    let graph = Command::new(env!("CARGO_BIN_EXE_auralis"))
        .args(["run", spec.to_str().unwrap()])
        .output()
        .unwrap();
    let render = Command::new(env!("CARGO_BIN_EXE_auralis"))
        .args([
            "render",
            input.to_str().unwrap(),
            "-o",
            render_output.to_str().unwrap(),
            "--fx",
            "gain -6dB",
        ])
        .output()
        .unwrap();

    fs::remove_file(spec).unwrap();
    fs::remove_file(input).unwrap();
    assert!(graph.status.success(), "{}", stderr(&graph));
    assert!(render.status.success(), "{}", stderr(&render));
    assert_eq!(
        read_pcm16_wav(&graph_output),
        read_pcm16_wav(&render_output)
    );
    fs::remove_file(graph_output).unwrap();
    fs::remove_file(render_output).unwrap();
}

#[test]
fn run_graph_spec_applies_dcshift_chain_step_param() {
    let spec = temp_path("auralis-cli-run-spec-dcshift-param", "toml");
    let input = temp_path("auralis-cli-run-spec-dcshift-param-input", "wav");
    let graph_output = temp_path("auralis-cli-run-spec-dcshift-param-output", "wav");
    let render_output = temp_path("auralis-cli-run-spec-dcshift-param-render-output", "wav");
    write_pcm16_wav(&input, 2, &[-32768, 0, 8192, 30_000]);
    fs::write(
        &spec,
        format!(
            r#"version = "auralis.graph/v1"

[[sources]]
id = "voice"
path = "{}"

[[chains]]
id = "voice_shifted"
input = "voice.audio"
steps = [
  {{ op = "dcshift", shift = "0.25" }},
]

[[sinks]]
id = "wav"
input = "voice_shifted.audio"
path = "{}"
"#,
            input.display(),
            graph_output.display()
        ),
    )
    .unwrap();

    let graph = Command::new(env!("CARGO_BIN_EXE_auralis"))
        .args(["run", spec.to_str().unwrap()])
        .output()
        .unwrap();
    let render = Command::new(env!("CARGO_BIN_EXE_auralis"))
        .args([
            "render",
            input.to_str().unwrap(),
            "-o",
            render_output.to_str().unwrap(),
            "--fx",
            "dcshift 0.25",
        ])
        .output()
        .unwrap();

    fs::remove_file(spec).unwrap();
    fs::remove_file(input).unwrap();
    assert!(graph.status.success(), "{}", stderr(&graph));
    assert!(render.status.success(), "{}", stderr(&render));
    assert_eq!(
        read_pcm16_wav(&graph_output),
        read_pcm16_wav(&render_output)
    );
    fs::remove_file(graph_output).unwrap();
    fs::remove_file(render_output).unwrap();
}

#[test]
fn run_graph_spec_applies_trim_chain_step_range() {
    let spec = temp_path("auralis-cli-run-spec-trim-range", "toml");
    let input = temp_path("auralis-cli-run-spec-trim-range-input", "wav");
    let graph_output = temp_path("auralis-cli-run-spec-trim-range-output", "wav");
    let render_output = temp_path("auralis-cli-run-spec-trim-range-render-output", "wav");
    write_pcm16_wav(&input, 1, &[-1000, -500, 0, 500, 1000]);
    fs::write(
        &spec,
        format!(
            r#"version = "auralis.graph/v1"

[[sources]]
id = "voice"
path = "{}"

[[chains]]
id = "voice_trimmed"
input = "voice.audio"
steps = [
  {{ op = "trim", range = "1..4" }},
]

[[sinks]]
id = "wav"
input = "voice_trimmed.audio"
path = "{}"
"#,
            input.display(),
            graph_output.display()
        ),
    )
    .unwrap();

    let graph = Command::new(env!("CARGO_BIN_EXE_auralis"))
        .args(["run", spec.to_str().unwrap()])
        .output()
        .unwrap();
    let render = Command::new(env!("CARGO_BIN_EXE_auralis"))
        .args([
            "render",
            input.to_str().unwrap(),
            "-o",
            render_output.to_str().unwrap(),
            "--fx",
            "trim 1 =4",
        ])
        .output()
        .unwrap();

    fs::remove_file(spec).unwrap();
    fs::remove_file(input).unwrap();
    assert!(graph.status.success(), "{}", stderr(&graph));
    assert!(render.status.success(), "{}", stderr(&render));
    assert_eq!(
        read_pcm16_wav(&graph_output),
        read_pcm16_wav(&render_output)
    );
    fs::remove_file(graph_output).unwrap();
    fs::remove_file(render_output).unwrap();
}

#[test]
fn run_graph_spec_applies_fade_chain_step_params() {
    let spec = temp_path("auralis-cli-run-spec-fade-param", "toml");
    let input = temp_path("auralis-cli-run-spec-fade-param-input", "wav");
    let graph_output = temp_path("auralis-cli-run-spec-fade-param-output", "wav");
    let render_output = temp_path("auralis-cli-run-spec-fade-param-render-output", "wav");
    write_pcm16_wav(&input, 1, &[10000, 12000, 14000, 16000, 18000]);
    fs::write(
        &spec,
        format!(
            r#"version = "auralis.graph/v1"

[[sources]]
id = "voice"
path = "{}"

[[chains]]
id = "voice_faded"
input = "voice.audio"
steps = [
  {{ op = "fade", curve = "linear", fade_in = "2", fade_out = "2" }},
]

[[sinks]]
id = "wav"
input = "voice_faded.audio"
path = "{}"
"#,
            input.display(),
            graph_output.display()
        ),
    )
    .unwrap();

    let graph = Command::new(env!("CARGO_BIN_EXE_auralis"))
        .args(["run", spec.to_str().unwrap()])
        .output()
        .unwrap();
    let render = Command::new(env!("CARGO_BIN_EXE_auralis"))
        .args([
            "render",
            input.to_str().unwrap(),
            "-o",
            render_output.to_str().unwrap(),
            "--fx",
            "fade t 2 0 2",
        ])
        .output()
        .unwrap();

    fs::remove_file(spec).unwrap();
    fs::remove_file(input).unwrap();
    assert!(graph.status.success(), "{}", stderr(&graph));
    assert!(render.status.success(), "{}", stderr(&render));
    assert_eq!(
        read_pcm16_wav(&graph_output),
        read_pcm16_wav(&render_output)
    );
    fs::remove_file(graph_output).unwrap();
    fs::remove_file(render_output).unwrap();
}

#[test]
fn run_graph_spec_applies_highpass_chain_step_params() {
    let spec = temp_path("auralis-cli-run-spec-highpass-param", "toml");
    let input = temp_path("auralis-cli-run-spec-highpass-param-input", "wav");
    let graph_output = temp_path("auralis-cli-run-spec-highpass-param-output", "wav");
    let render_output = temp_path("auralis-cli-run-spec-highpass-param-render-output", "wav");
    write_pcm16_wav(&input, 1, &[0, 8000, -8000, 12000, -12000, 6000, -6000]);
    fs::write(
        &spec,
        format!(
            r#"version = "auralis.graph/v1"

[[sources]]
id = "voice"
path = "{}"

[[chains]]
id = "voice_filtered"
input = "voice.audio"
steps = [
  {{ op = "filter.highpass", cutoff = "1000Hz", q = 0.707 }},
]

[[sinks]]
id = "wav"
input = "voice_filtered.audio"
path = "{}"
"#,
            input.display(),
            graph_output.display()
        ),
    )
    .unwrap();

    let graph = Command::new(env!("CARGO_BIN_EXE_auralis"))
        .args(["run", spec.to_str().unwrap()])
        .output()
        .unwrap();
    let render = Command::new(env!("CARGO_BIN_EXE_auralis"))
        .args([
            "render",
            input.to_str().unwrap(),
            "-o",
            render_output.to_str().unwrap(),
            "--fx",
            "highpass 1000 0.707q",
        ])
        .output()
        .unwrap();

    fs::remove_file(spec).unwrap();
    fs::remove_file(input).unwrap();
    assert!(graph.status.success(), "{}", stderr(&graph));
    assert!(render.status.success(), "{}", stderr(&render));
    assert_eq!(
        read_pcm16_wav(&graph_output),
        read_pcm16_wav(&render_output)
    );
    fs::remove_file(graph_output).unwrap();
    fs::remove_file(render_output).unwrap();
}

#[test]
fn run_graph_spec_applies_norm_peak_chain_step_target() {
    let spec = temp_path("auralis-cli-run-spec-norm-peak-target", "toml");
    let input = temp_path("auralis-cli-run-spec-norm-peak-target-input", "wav");
    let graph_output = temp_path("auralis-cli-run-spec-norm-peak-target-output", "wav");
    let render_output = temp_path("auralis-cli-run-spec-norm-peak-target-render-output", "wav");
    write_pcm16_wav(&input, 1, &[0, 2000, -4000, 8000, -12000]);
    fs::write(
        &spec,
        format!(
            r#"version = "auralis.graph/v1"

[[sources]]
id = "voice"
path = "{}"

[[chains]]
id = "voice_normalized"
input = "voice.audio"
steps = [
  {{ op = "norm.peak", target = "-6dBFS" }},
]

[[sinks]]
id = "wav"
input = "voice_normalized.audio"
path = "{}"
"#,
            input.display(),
            graph_output.display()
        ),
    )
    .unwrap();

    let graph = Command::new(env!("CARGO_BIN_EXE_auralis"))
        .args(["run", spec.to_str().unwrap()])
        .output()
        .unwrap();
    let render = Command::new(env!("CARGO_BIN_EXE_auralis"))
        .args([
            "render",
            input.to_str().unwrap(),
            "-o",
            render_output.to_str().unwrap(),
            "--fx",
            "norm -6",
        ])
        .output()
        .unwrap();

    fs::remove_file(spec).unwrap();
    fs::remove_file(input).unwrap();
    assert!(graph.status.success(), "{}", stderr(&graph));
    assert!(render.status.success(), "{}", stderr(&render));
    assert_eq!(
        read_pcm16_wav(&graph_output),
        read_pcm16_wav(&render_output)
    );
    fs::remove_file(graph_output).unwrap();
    fs::remove_file(render_output).unwrap();
}

#[test]
fn run_graph_spec_reports_unsupported_node_execution() {
    let spec = temp_path("auralis-cli-run-spec-node", "toml");
    fs::write(
        &spec,
        r#"version = "auralis.graph/v1"

[[sources]]
id = "voice"
path = "input/voice.wav"

[[nodes]]
id = "master"
input = "voice.audio"

[[sinks]]
id = "wav"
input = "master.audio"
path = "build/out.wav"
"#,
    )
    .unwrap();

    let command_output = Command::new(env!("CARGO_BIN_EXE_auralis"))
        .args(["run", spec.to_str().unwrap()])
        .output()
        .unwrap();

    fs::remove_file(spec).unwrap();
    assert!(!command_output.status.success());
    let stderr = stderr(&command_output);
    assert!(
        stderr.contains("run currently supports source-to-chain-to-sink graph specs only"),
        "{stderr}"
    );
}

#[test]
fn render_help_documents_modern_effect_inputs() {
    let command_output = Command::new(env!("CARGO_BIN_EXE_auralis"))
        .args(["render", "--help"])
        .output()
        .unwrap();

    assert!(command_output.status.success());
    let stdout = stdout(&command_output);
    assert!(stdout.contains("--backend <BACKEND>"), "{stdout}");
    assert!(
        stdout.contains("Sample-processing backend to request"),
        "{stdout}"
    );
    assert!(stdout.contains("--combine <METHOD>"), "{stdout}");
    assert!(
        stdout.contains("Input-combiner method to apply before effects"),
        "{stdout}"
    );
    assert!(stdout.contains("--input <FILE>"), "{stdout}");
    assert!(
        stdout.contains("Additional PCM16 WAV input files to combine"),
        "{stdout}"
    );
    assert!(stdout.contains("--channels <CHANNELS>"), "{stdout}");
    assert!(
        stdout.contains("Output channel count; inserts SoX-ng-style channel conversion"),
        "{stdout}"
    );
    assert!(stdout.contains("--no-auto-channels"), "{stdout}");
    assert!(
        stdout.contains("Fail instead of automatically converting channels"),
        "{stdout}"
    );
    assert!(stdout.contains("--rate <RATE>"), "{stdout}");
    assert!(
        stdout.contains("Output sample rate; inserts deterministic rate conversion"),
        "{stdout}"
    );
    assert!(stdout.contains("--no-auto-rate"), "{stdout}");
    assert!(
        stdout.contains("Fail instead of automatically converting sample rate"),
        "{stdout}"
    );
    assert!(stdout.contains("--guard"), "{stdout}");
    assert!(
        stdout.contains("Attenuate final output only if it would clip"),
        "{stdout}"
    );
    assert!(stdout.contains("--norm [<DB>]"), "{stdout}");
    assert!(
        stdout.contains("Normalize final output to a peak level"),
        "{stdout}"
    );
    assert!(stdout.contains("--dither"), "{stdout}");
    assert!(
        stdout.contains("Apply deterministic TPDF dither before PCM16 encoding"),
        "{stdout}"
    );
    assert!(stdout.contains("--dither-seed <SEED>"), "{stdout}");
    assert!(stdout.contains("--fx <EFFECT>"), "{stdout}");
    assert!(
        stdout.contains("One typed effect command per flag"),
        "{stdout}"
    );
    assert!(stdout.contains("--chain <CHAIN>"), "{stdout}");
    assert!(stdout.contains("Compact ordered effect chain"), "{stdout}");
    assert!(stdout.contains("--effects-file <FILE>"), "{stdout}");
    assert!(
        stdout.contains("Read the effect chain from a SoX-ng-style effects file"),
        "{stdout}"
    );
}

#[test]
fn render_unsupported_input_extension_returns_clear_error() {
    let input = temp_path("auralis-cli-render-input-unsupported", "flac");
    let output = temp_path("auralis-cli-render-output", "wav");
    fs::write(&input, b"not a supported input").unwrap();

    let command_output = Command::new(env!("CARGO_BIN_EXE_auralis"))
        .args([
            "render",
            input.to_str().unwrap(),
            "-o",
            output.to_str().unwrap(),
        ])
        .output()
        .unwrap();

    fs::remove_file(input).unwrap();
    let _ = fs::remove_file(output);
    assert!(!command_output.status.success());
    let stderr = stderr(&command_output);
    assert!(
        stderr.contains("error: unsupported input format"),
        "{stderr}"
    );
    assert!(stderr.contains("only PCM16 WAV is supported"), "{stderr}");
}

#[test]
fn render_unsupported_output_extension_returns_clear_error() {
    let input = temp_path("auralis-cli-render-input", "wav");
    let output = temp_path("auralis-cli-render-output-unsupported", "flac");
    write_pcm16_wav(&input, 1, &[0]);

    let command_output = Command::new(env!("CARGO_BIN_EXE_auralis"))
        .args([
            "render",
            input.to_str().unwrap(),
            "-o",
            output.to_str().unwrap(),
        ])
        .output()
        .unwrap();

    fs::remove_file(input).unwrap();
    let _ = fs::remove_file(output);
    assert!(!command_output.status.success());
    let stderr = stderr(&command_output);
    assert!(
        stderr.contains("error: unsupported output format"),
        "{stderr}"
    );
    assert!(stderr.contains("only PCM16 WAV is supported"), "{stderr}");
}

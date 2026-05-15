//! Integration tests for graph spec target references.

mod support;

use support::*;

#[test]
fn run_graph_spec_target_writes_only_selected_sink() {
    let spec = temp_path("auralis-cli-run-target-spec", "toml");
    let input = temp_path("auralis-cli-run-target-input", "wav");
    let master_output = temp_path("auralis-cli-run-target-master", "wav");
    let preview_output = temp_path("auralis-cli-run-target-preview", "wav");
    write_pcm16_wav(&input, 1, &[1000, -2000, 3000]);
    fs::write(
        &spec,
        format!(
            r#"version = "auralis.graph/v1"

[[sources]]
id = "voice"
path = "{}"

[[chains]]
id = "master"
input = "voice.audio"
steps = [
  {{ op = "reverse" }},
]

[[sinks]]
id = "wav"
input = "master.audio"
path = "{}"

[[sinks]]
id = "preview"
input = "master.audio"
path = "{}"
"#,
            input.display(),
            master_output.display(),
            preview_output.display()
        ),
    )
    .unwrap();

    let spec_ref = format!("{}#preview", spec.display());
    let command_output = Command::new(env!("CARGO_BIN_EXE_auralis"))
        .args(["run", &spec_ref])
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
    assert!(stdout.contains("preview"), "{stdout}");
    assert!(
        !stdout.contains(&master_output.display().to_string()),
        "{stdout}"
    );
    assert!(!master_output.exists());
    assert_eq!(
        read_pcm16_wav(&preview_output),
        (1, vec![3000, -2000, 1000])
    );
    fs::remove_file(preview_output).unwrap();
}

#[test]
fn plan_graph_spec_accepts_target_fragment() {
    let spec = temp_path("auralis-cli-plan-target-spec", "toml");
    let input = temp_path("auralis-cli-plan-target-input", "wav");
    let output = temp_path("auralis-cli-plan-target-output", "wav");
    write_pcm16_wav(&input, 1, &[1000]);
    fs::write(
        &spec,
        format!(
            r#"version = "auralis.graph/v1"

[[sources]]
id = "voice"
path = "{}"

[[chains]]
id = "master"
input = "voice.audio"
steps = [
  {{ op = "gain", by = "0dB" }},
]

[[sinks]]
id = "wav"
input = "master.audio"
path = "{}"
"#,
            input.display(),
            output.display()
        ),
    )
    .unwrap();

    let spec_ref = format!("{}#master", spec.display());
    let command_output = Command::new(env!("CARGO_BIN_EXE_auralis"))
        .args(["plan", &spec_ref])
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
    assert!(stdout.contains("Target: master"), "{stdout}");
    let _ = fs::remove_file(output);
}

#[test]
fn plan_graph_spec_accepts_target_flag() {
    let spec = temp_path("auralis-cli-plan-target-flag-spec", "toml");
    let input = temp_path("auralis-cli-plan-target-flag-input", "wav");
    let output = temp_path("auralis-cli-plan-target-flag-output", "wav");
    write_pcm16_wav(&input, 1, &[1000]);
    fs::write(
        &spec,
        format!(
            r#"version = "auralis.graph/v1"

[[sources]]
id = "voice"
path = "{}"

[[chains]]
id = "master"
input = "voice.audio"
steps = [
  {{ op = "gain", by = "0dB" }},
]

[[sinks]]
id = "wav"
input = "master.audio"
path = "{}"
"#,
            input.display(),
            output.display()
        ),
    )
    .unwrap();

    let command_output = Command::new(env!("CARGO_BIN_EXE_auralis"))
        .args(["plan", spec.to_str().unwrap(), "--target", "master"])
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
    assert!(stdout.contains("Target: master"), "{stdout}");
    let _ = fs::remove_file(output);
}

#[test]
fn run_graph_spec_target_flag_uses_default_auralis_toml() {
    let dir = temp_path("auralis-cli-run-target-flag", "dir");
    fs::create_dir_all(&dir).unwrap();
    let input = dir.join("input.wav");
    let master_output = dir.join("master.wav");
    let preview_output = dir.join("preview.wav");
    write_pcm16_wav(&input, 1, &[1000, -2000, 3000]);
    fs::write(
        dir.join("Auralis.toml"),
        r#"version = "auralis.graph/v1"

[[sources]]
id = "voice"
path = "input.wav"

[[chains]]
id = "master"
input = "voice.audio"
steps = [
  { op = "reverse" },
]

[[sinks]]
id = "wav"
input = "master.audio"
path = "master.wav"

[[sinks]]
id = "preview"
input = "master.audio"
path = "preview.wav"
"#,
    )
    .unwrap();

    let command_output = Command::new(env!("CARGO_BIN_EXE_auralis"))
        .current_dir(&dir)
        .args(["run", "--target", "preview"])
        .output()
        .unwrap();

    assert!(
        command_output.status.success(),
        "{}",
        stderr(&command_output)
    );
    assert!(!master_output.exists());
    assert_eq!(
        read_pcm16_wav(&preview_output),
        (1, vec![3000, -2000, 1000])
    );
    fs::remove_dir_all(dir).unwrap();
}

#[test]
fn run_graph_spec_accepts_documented_cache_modes() {
    let help = Command::new(env!("CARGO_BIN_EXE_auralis"))
        .args(["run", "--help"])
        .output()
        .unwrap();
    assert!(help.status.success(), "{}", stderr(&help));
    let help_stdout = stdout(&help);
    assert!(help_stdout.contains("--cache <CACHE>"), "{help_stdout}");
    assert!(
        help_stdout.contains("[possible values: off, smart, full]"),
        "{help_stdout}"
    );

    let spec = temp_path("auralis-cli-run-cache-mode-spec", "toml");
    let input = temp_path("auralis-cli-run-cache-mode-input", "wav");
    let output = temp_path("auralis-cli-run-cache-mode-output", "wav");
    write_pcm16_wav(&input, 1, &[1000, -2000, 3000]);
    fs::write(
        &spec,
        format!(
            r#"version = "auralis.graph/v1"

[[sources]]
id = "voice"
path = "{}"

[[chains]]
id = "master"
input = "voice.audio"
steps = [
  {{ op = "reverse" }},
]

[[sinks]]
id = "wav"
input = "master.audio"
path = "{}"
"#,
            input.display(),
            output.display()
        ),
    )
    .unwrap();

    let command_output = Command::new(env!("CARGO_BIN_EXE_auralis"))
        .args(["run", spec.to_str().unwrap(), "--cache", "off"])
        .output()
        .unwrap();

    fs::remove_file(spec).unwrap();
    fs::remove_file(input).unwrap();
    assert!(
        command_output.status.success(),
        "{}",
        stderr(&command_output)
    );
    assert_eq!(read_pcm16_wav(&output), (1, vec![3000, -2000, 1000]));
    fs::remove_file(output).unwrap();
}

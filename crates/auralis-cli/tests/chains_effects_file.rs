//! CLI integration tests for effect chains and effects files.

mod support;

use auralis::AudioFile;
use support::*;

#[test]
fn render_fx_chain_output_matches_in_memory_chain() {
    let input = temp_path("auralis-cli-render-chain-input", "wav");
    let cli_output = temp_path("auralis-cli-render-chain-cli-output", "wav");
    let library_output = temp_path("auralis-cli-render-chain-library-output", "wav");
    write_pcm16_wav(&input, 2, &[-16_384, 16_384, -8_192, 8_192, 0, 4096]);
    let chain_tokens = ["gain", "-6", "dcshift", "0.125", "reverse"];
    let chain = auralis::parse_effect_chain(&chain_tokens).unwrap();

    AudioFile::open_wav(&input)
        .unwrap()
        .into_pipeline()
        .apply_effect_chain(&chain)
        .write_wav(&library_output)
        .unwrap();

    let command_output = Command::new(env!("CARGO_BIN_EXE_auralis"))
        .args([
            "render",
            input.to_str().unwrap(),
            "-o",
            cli_output.to_str().unwrap(),
            "--fx",
            "gain -6",
            "--fx",
            "dcshift 0.125",
            "--fx",
            "reverse",
        ])
        .output()
        .unwrap();

    assert!(
        command_output.status.success(),
        "stderr: {}",
        stderr(&command_output)
    );
    assert_eq!(read_pcm16_wav(&cli_output), read_pcm16_wav(&library_output));

    fs::remove_file(input).unwrap();
    fs::remove_file(cli_output).unwrap();
    fs::remove_file(library_output).unwrap();
}

#[test]
fn render_effects_file_output_matches_equivalent_fx_chain() {
    let input = temp_path("auralis-cli-render-effects-file-input", "wav");
    let effects_file = temp_path("auralis-cli-render-effects-file", "effects");
    let fx_output = temp_path("auralis-cli-render-effects-file-fx-output", "wav");
    let file_output = temp_path("auralis-cli-render-effects-file-output", "wav");
    write_pcm16_wav(&input, 2, &[-16_384, 16_384, -8_192, 8_192, 0, 4096]);
    fs::write(
        &effects_file,
        "# level then edit\n\
         gain -6\n\
         dcshift 0.125\n\
         reverse\n",
    )
    .unwrap();

    let fx_command_output = Command::new(env!("CARGO_BIN_EXE_auralis"))
        .args([
            "render",
            input.to_str().unwrap(),
            "-o",
            fx_output.to_str().unwrap(),
            "--fx",
            "gain -6",
            "--fx",
            "dcshift 0.125",
            "--fx",
            "reverse",
        ])
        .output()
        .unwrap();
    let file_command_output = Command::new(env!("CARGO_BIN_EXE_auralis"))
        .args([
            "render",
            input.to_str().unwrap(),
            "-o",
            file_output.to_str().unwrap(),
            "--effects-file",
            effects_file.to_str().unwrap(),
        ])
        .output()
        .unwrap();

    assert!(
        fx_command_output.status.success(),
        "stderr: {}",
        stderr(&fx_command_output)
    );
    assert!(
        file_command_output.status.success(),
        "stderr: {}",
        stderr(&file_command_output)
    );
    assert_eq!(read_pcm16_wav(&file_output), read_pcm16_wav(&fx_output));

    fs::remove_file(input).unwrap();
    fs::remove_file(effects_file).unwrap();
    fs::remove_file(fx_output).unwrap();
    fs::remove_file(file_output).unwrap();
}

#[test]
fn render_fx_chain_preserves_user_order() {
    let input = temp_path("auralis-cli-render-chain-order-input", "wav");
    let gain_then_shift = temp_path("auralis-cli-render-chain-gain-shift-output", "wav");
    let shift_then_gain = temp_path("auralis-cli-render-chain-shift-gain-output", "wav");
    write_pcm16_wav(&input, 1, &[4096, 8192, 12_288]);

    let gain_then_shift_output = Command::new(env!("CARGO_BIN_EXE_auralis"))
        .args([
            "render",
            input.to_str().unwrap(),
            "-o",
            gain_then_shift.to_str().unwrap(),
            "--fx",
            "gain -6",
            "--fx",
            "dcshift 0.125",
        ])
        .output()
        .unwrap();
    let shift_then_gain_output = Command::new(env!("CARGO_BIN_EXE_auralis"))
        .args([
            "render",
            input.to_str().unwrap(),
            "-o",
            shift_then_gain.to_str().unwrap(),
            "--fx",
            "dcshift 0.125",
            "--fx",
            "gain -6",
        ])
        .output()
        .unwrap();

    assert!(
        gain_then_shift_output.status.success(),
        "stderr: {}",
        stderr(&gain_then_shift_output)
    );
    assert!(
        shift_then_gain_output.status.success(),
        "stderr: {}",
        stderr(&shift_then_gain_output)
    );
    assert_ne!(
        read_pcm16_wav(&gain_then_shift),
        read_pcm16_wav(&shift_then_gain)
    );

    fs::remove_file(input).unwrap();
    fs::remove_file(gain_then_shift).unwrap();
    fs::remove_file(shift_then_gain).unwrap();
}

#[test]
fn render_accepts_boundary_separator_in_chain_and_effects_file() {
    let input = temp_path("auralis-cli-render-chain-boundary-input", "wav");
    let flat_output = temp_path("auralis-cli-render-chain-boundary-flat-output", "wav");
    let boundary_output = temp_path("auralis-cli-render-chain-boundary-output", "wav");
    let file_output = temp_path("auralis-cli-render-chain-boundary-file-output", "wav");
    let effects_file = temp_path("auralis-cli-render-chain-boundary", "effects");
    write_pcm16_wav(&input, 1, &[4096, -8192, 12_288, -16_384]);
    fs::write(&effects_file, "gain -6 :\ndcshift 0.125 reverse\n").unwrap();

    let flat_command_output = Command::new(env!("CARGO_BIN_EXE_auralis"))
        .args([
            "render",
            input.to_str().unwrap(),
            "-o",
            flat_output.to_str().unwrap(),
            "--chain",
            "gain -6 | dcshift 0.125 | reverse",
        ])
        .output()
        .unwrap();
    let boundary_command_output = Command::new(env!("CARGO_BIN_EXE_auralis"))
        .args([
            "render",
            input.to_str().unwrap(),
            "-o",
            boundary_output.to_str().unwrap(),
            "--chain",
            "gain -6 : | dcshift 0.125 | reverse",
        ])
        .output()
        .unwrap();
    let file_command_output = Command::new(env!("CARGO_BIN_EXE_auralis"))
        .args([
            "render",
            input.to_str().unwrap(),
            "-o",
            file_output.to_str().unwrap(),
            "--effects-file",
            effects_file.to_str().unwrap(),
        ])
        .output()
        .unwrap();

    assert!(
        flat_command_output.status.success(),
        "stderr: {}",
        stderr(&flat_command_output)
    );
    assert!(
        boundary_command_output.status.success(),
        "stderr: {}",
        stderr(&boundary_command_output)
    );
    assert!(
        file_command_output.status.success(),
        "stderr: {}",
        stderr(&file_command_output)
    );
    assert_eq!(
        read_pcm16_wav(&boundary_output),
        read_pcm16_wav(&flat_output)
    );
    assert_eq!(read_pcm16_wav(&file_output), read_pcm16_wav(&flat_output));

    fs::remove_file(input).unwrap();
    fs::remove_file(flat_output).unwrap();
    fs::remove_file(boundary_output).unwrap();
    fs::remove_file(file_output).unwrap();
    fs::remove_file(effects_file).unwrap();
}

#[test]
fn render_boundary_control_returns_clear_error() {
    let input = temp_path("auralis-cli-render-chain-boundary-control-input", "wav");
    let output = temp_path("auralis-cli-render-chain-boundary-control-output", "wav");
    write_pcm16_wav(&input, 1, &[0, 1, 2]);

    let command_output = Command::new(env!("CARGO_BIN_EXE_auralis"))
        .args([
            "render",
            input.to_str().unwrap(),
            "-o",
            output.to_str().unwrap(),
            "--fx",
            "gain -3 : newfile",
        ])
        .output()
        .unwrap();

    fs::remove_file(input).unwrap();
    let _ = fs::remove_file(output);
    assert!(!command_output.status.success());
    let stderr = stderr(&command_output);
    assert!(stderr.contains("unsupported boundary control"), "{stderr}");
    assert!(
        stderr.contains("`newfile` and `restart` semantics are not implemented"),
        "{stderr}"
    );
}

#[test]
fn render_blocked_dolbyb_effect_returns_actionable_error() {
    let input = temp_path("auralis-cli-render-dolbyb-blocked-input", "wav");
    let output = temp_path("auralis-cli-render-dolbyb-blocked-output", "wav");
    write_pcm16_wav(&input, 1, &[0, 1, 2]);

    let command_output = Command::new(env!("CARGO_BIN_EXE_auralis"))
        .args([
            "render",
            input.to_str().unwrap(),
            "-o",
            output.to_str().unwrap(),
            "--fx",
            "dolbyb",
        ])
        .output()
        .unwrap();

    fs::remove_file(input).unwrap();
    let _ = fs::remove_file(output);
    assert!(!command_output.status.success());
    let stderr = stderr(&command_output);
    assert!(
        stderr.contains("known SoX-ng effect `dolbyb` is blocked"),
        "{stderr}"
    );
    assert!(stderr.contains("use `sox_ng ... dolbyb ...`"), "{stderr}");
    assert!(
        stderr.contains("compatible pure-Rust/public-domain spec"),
        "{stderr}"
    );
}

#[test]
fn render_not_planned_dop_effect_returns_actionable_error() {
    let input = temp_path("auralis-cli-render-dop-not-planned-input", "wav");
    let output = temp_path("auralis-cli-render-dop-not-planned-output", "wav");
    write_pcm16_wav(&input, 1, &[0, 1, 2]);

    let command_output = Command::new(env!("CARGO_BIN_EXE_auralis"))
        .args([
            "render",
            input.to_str().unwrap(),
            "-o",
            output.to_str().unwrap(),
            "--fx",
            "dop",
        ])
        .output()
        .unwrap();

    fs::remove_file(input).unwrap();
    let _ = fs::remove_file(output);
    assert!(!command_output.status.success());
    let stderr = stderr(&command_output);
    assert!(
        stderr.contains("known SoX-ng effect `dop` is not planned"),
        "{stderr}"
    );
    assert!(
        stderr.contains("DSD-over-PCM transport packing"),
        "{stderr}"
    );
    assert!(stderr.contains("future DSD/DoP format support"), "{stderr}");
}

#[test]
fn render_blocked_ladspa_effect_returns_actionable_error() {
    let input = temp_path("auralis-cli-render-ladspa-blocked-input", "wav");
    let output = temp_path("auralis-cli-render-ladspa-blocked-output", "wav");
    write_pcm16_wav(&input, 1, &[0, 1, 2]);

    let command_output = Command::new(env!("CARGO_BIN_EXE_auralis"))
        .args([
            "render",
            input.to_str().unwrap(),
            "-o",
            output.to_str().unwrap(),
            "--fx",
            "ladspa cmt",
        ])
        .output()
        .unwrap();

    fs::remove_file(input).unwrap();
    let _ = fs::remove_file(output);
    assert!(!command_output.status.success());
    let stderr = stderr(&command_output);
    assert!(
        stderr.contains("known SoX-ng effect `ladspa` is blocked"),
        "{stderr}"
    );
    assert!(stderr.contains("native external plugins"), "{stderr}");
    assert!(stderr.contains("future external-host boundary"), "{stderr}");
}

#[test]
fn render_not_planned_sdm_effect_returns_actionable_error() {
    let input = temp_path("auralis-cli-render-sdm-not-planned-input", "wav");
    let output = temp_path("auralis-cli-render-sdm-not-planned-output", "wav");
    write_pcm16_wav(&input, 1, &[0, 1, 2]);

    let command_output = Command::new(env!("CARGO_BIN_EXE_auralis"))
        .args([
            "render",
            input.to_str().unwrap(),
            "-o",
            output.to_str().unwrap(),
            "--fx",
            "sdm",
        ])
        .output()
        .unwrap();

    fs::remove_file(input).unwrap();
    let _ = fs::remove_file(output);
    assert!(!command_output.status.success());
    let stderr = stderr(&command_output);
    assert!(
        stderr.contains("known SoX-ng effect `sdm` is not planned"),
        "{stderr}"
    );
    assert!(
        stderr.contains("DSD-oriented sigma-delta modulator"),
        "{stderr}"
    );
    assert!(
        stderr.contains("future DSD/1-bit format support"),
        "{stderr}"
    );
}

#[test]
fn render_missing_effects_file_returns_clear_error() {
    let input = temp_path("auralis-cli-render-missing-effects-file-input", "wav");
    let effects_file = temp_path("auralis-cli-render-missing-effects-file", "effects");
    let output = temp_path("auralis-cli-render-missing-effects-file-output", "wav");
    write_pcm16_wav(&input, 1, &[0, 1, 2]);

    let command_output = Command::new(env!("CARGO_BIN_EXE_auralis"))
        .args([
            "render",
            input.to_str().unwrap(),
            "-o",
            output.to_str().unwrap(),
            "--effects-file",
            effects_file.to_str().unwrap(),
        ])
        .output()
        .unwrap();

    fs::remove_file(input).unwrap();
    let _ = fs::remove_file(output);
    assert!(!command_output.status.success());
    let stderr = stderr(&command_output);
    assert!(stderr.contains("failed to read effects file"), "{stderr}");
    assert!(stderr.contains(effects_file.to_str().unwrap()), "{stderr}");
}

#[test]
fn render_unreadable_effects_file_returns_clear_error() {
    let input = temp_path("auralis-cli-render-unreadable-effects-file-input", "wav");
    let effects_dir = temp_path("auralis-cli-render-unreadable-effects-file", "effects");
    let output = temp_path("auralis-cli-render-unreadable-effects-file-output", "wav");
    write_pcm16_wav(&input, 1, &[0, 1, 2]);
    fs::create_dir(&effects_dir).unwrap();

    let command_output = Command::new(env!("CARGO_BIN_EXE_auralis"))
        .args([
            "render",
            input.to_str().unwrap(),
            "-o",
            output.to_str().unwrap(),
            "--effects-file",
            effects_dir.to_str().unwrap(),
        ])
        .output()
        .unwrap();

    fs::remove_file(input).unwrap();
    fs::remove_dir(effects_dir).unwrap();
    let _ = fs::remove_file(output);
    assert!(!command_output.status.success());
    let stderr = stderr(&command_output);
    assert!(stderr.contains("failed to read effects file"), "{stderr}");
}

#[test]
fn render_rejects_effects_file_with_fx_chain() {
    let input = temp_path("auralis-cli-render-effects-file-mixed-chain-input", "wav");
    let effects_file = temp_path("auralis-cli-render-effects-file-mixed-chain", "effects");
    let output = temp_path("auralis-cli-render-effects-file-mixed-chain-output", "wav");
    write_pcm16_wav(&input, 1, &[0, 1, 2]);
    fs::write(&effects_file, "gain -3\n").unwrap();

    let command_output = Command::new(env!("CARGO_BIN_EXE_auralis"))
        .args([
            "render",
            input.to_str().unwrap(),
            "-o",
            output.to_str().unwrap(),
            "--effects-file",
            effects_file.to_str().unwrap(),
            "--fx",
            "reverse",
        ])
        .output()
        .unwrap();

    fs::remove_file(input).unwrap();
    fs::remove_file(effects_file).unwrap();
    let _ = fs::remove_file(output);
    assert!(!command_output.status.success());
    let stderr = stderr(&command_output);
    assert!(
        stderr.contains("--fx, --chain, and --effects-file are mutually exclusive"),
        "{stderr}"
    );
}

#[test]
fn render_rejects_effects_file_with_chain() {
    let input = temp_path("auralis-cli-render-effects-file-mixed-chain-input", "wav");
    let effects_file = temp_path("auralis-cli-render-effects-file-mixed-chain", "effects");
    let output = temp_path("auralis-cli-render-effects-file-mixed-chain-output", "wav");
    write_pcm16_wav(&input, 1, &[0, 1, 2]);
    fs::write(&effects_file, "reverse\n").unwrap();

    let command_output = Command::new(env!("CARGO_BIN_EXE_auralis"))
        .args([
            "render",
            input.to_str().unwrap(),
            "-o",
            output.to_str().unwrap(),
            "--effects-file",
            effects_file.to_str().unwrap(),
            "--chain",
            "gain -3",
        ])
        .output()
        .unwrap();

    fs::remove_file(input).unwrap();
    fs::remove_file(effects_file).unwrap();
    let _ = fs::remove_file(output);
    assert!(!command_output.status.success());
    let stderr = stderr(&command_output);
    assert!(
        stderr.contains("--fx, --chain, and --effects-file are mutually exclusive"),
        "{stderr}"
    );
}

#[test]
fn render_invalid_fx_chain_reports_failing_effect_and_argument() {
    let input = temp_path("auralis-cli-render-invalid-chain-input", "wav");
    let output = temp_path("auralis-cli-render-invalid-chain-output", "wav");
    write_pcm16_wav(&input, 1, &[0, 1, 2]);

    let command_output = Command::new(env!("CARGO_BIN_EXE_auralis"))
        .args([
            "render",
            input.to_str().unwrap(),
            "-o",
            output.to_str().unwrap(),
            "--fx",
            "gain -3",
            "--fx",
            "trim reverse",
        ])
        .output()
        .unwrap();

    fs::remove_file(input).unwrap();
    let _ = fs::remove_file(output);
    assert!(!command_output.status.success());
    let stderr = stderr(&command_output);
    assert!(
        stderr.contains("effect chain command 1 (`trim`) failed to parse"),
        "{stderr}"
    );
    assert!(
        stderr.contains("effect `trim` requires argument `position`"),
        "{stderr}"
    );
}

#[test]
fn render_rejects_mixed_fx_and_chain_inputs() {
    let input = temp_path("auralis-cli-render-mixed-chain-input", "wav");
    let output = temp_path("auralis-cli-render-mixed-chain-output", "wav");
    write_pcm16_wav(&input, 1, &[0, 1, 2]);

    let command_output = Command::new(env!("CARGO_BIN_EXE_auralis"))
        .args([
            "render",
            input.to_str().unwrap(),
            "-o",
            output.to_str().unwrap(),
            "--fx",
            "gain -3",
            "--chain",
            "reverse",
        ])
        .output()
        .unwrap();

    fs::remove_file(input).unwrap();
    let _ = fs::remove_file(output);
    assert!(!command_output.status.success());
    let stderr = stderr(&command_output);
    assert!(
        stderr.contains("--fx, --chain, and --effects-file are mutually exclusive"),
        "{stderr}"
    );
}

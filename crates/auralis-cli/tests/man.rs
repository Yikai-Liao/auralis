//! Integration tests for the `auralis man` command.

mod support;

use support::*;

#[test]
fn top_level_man_page_lists_modern_commands() {
    let output = Command::new(env!("CARGO_BIN_EXE_auralis"))
        .args(["man"])
        .output()
        .unwrap();

    assert!(output.status.success(), "stderr: {}", stderr(&output));
    let stdout = stdout(&output);
    assert!(stdout.contains("NAME"), "{stdout}");
    assert!(
        stdout.contains("auralis - modern deterministic audio processing CLI"),
        "{stdout}"
    );
    assert!(stdout.contains("trim"), "{stdout}");
    assert!(stdout.contains("normalize"), "{stdout}");
    assert!(stdout.contains("gain"), "{stdout}");
    assert!(stdout.contains("reverse"), "{stdout}");
    assert!(stdout.contains("deemph"), "{stdout}");
    assert!(stdout.contains("earwax"), "{stdout}");
    assert!(stdout.contains("oops"), "{stdout}");
    assert!(stdout.contains("riaa"), "{stdout}");
    assert!(stdout.contains("swap"), "{stdout}");
    assert!(stdout.contains("contrast"), "{stdout}");
    assert!(stdout.contains("overdrive"), "{stdout}");
    assert!(stdout.contains("saturation"), "{stdout}");
    assert!(stdout.contains("fade"), "{stdout}");
    assert!(stdout.contains("mix"), "{stdout}");
    assert!(stdout.contains("concat"), "{stdout}");
    assert!(stdout.contains("mix-power"), "{stdout}");
    assert!(stdout.contains("merge"), "{stdout}");
    assert!(stdout.contains("multiply"), "{stdout}");
    assert!(stdout.contains("render"), "{stdout}");
    assert!(stdout.contains("plan"), "{stdout}");
    assert!(stdout.contains("completions"), "{stdout}");
    assert!(stdout.contains("man"), "{stdout}");
}

#[test]
fn render_man_page_includes_core_options() {
    let output = Command::new(env!("CARGO_BIN_EXE_auralis"))
        .args(["man", "render"])
        .output()
        .unwrap();

    assert!(output.status.success(), "stderr: {}", stderr(&output));
    let stdout = stdout(&output);
    assert!(stdout.contains("NAME"), "{stdout}");
    assert!(
        stdout.contains("render - run one ordered DSP pipeline"),
        "{stdout}"
    );
    assert!(stdout.contains("--fx EFFECT"), "{stdout}");
    assert!(stdout.contains("--chain CHAIN"), "{stdout}");
    assert!(stdout.contains("--combine METHOD"), "{stdout}");
}

#[test]
fn normalize_man_page_includes_peak_option() {
    let output = Command::new(env!("CARGO_BIN_EXE_auralis"))
        .args(["man", "normalize"])
        .output()
        .unwrap();

    assert!(output.status.success(), "stderr: {}", stderr(&output));
    let stdout = stdout(&output);
    assert!(
        stdout.contains("normalize - normalize to a peak level"),
        "{stdout}"
    );
    assert!(stdout.contains("--peak DBFS"), "{stdout}");
}

#[test]
fn reverse_man_page_describes_recipe_lowering() {
    let output = Command::new(env!("CARGO_BIN_EXE_auralis"))
        .args(["man", "reverse"])
        .output()
        .unwrap();

    assert!(output.status.success(), "stderr: {}", stderr(&output));
    let stdout = stdout(&output);
    assert!(
        stdout.contains("reverse - reverse one audio file"),
        "{stdout}"
    );
    assert!(stdout.contains("render --fx reverse"), "{stdout}");
    assert!(stdout.contains("-o, --output FILE"), "{stdout}");
}

#[test]
fn no_arg_effect_man_pages_describe_recipe_lowering() {
    for (topic, summary, render_form) in [
        ("deemph", "deemph - apply de-emphasis", "render --fx deemph"),
        (
            "earwax",
            "earwax - apply headphone-cue filtering",
            "render --fx earwax",
        ),
        (
            "oops",
            "oops - extract out-of-phase stereo",
            "render --fx oops",
        ),
        ("riaa", "riaa - apply RIAA equalization", "render --fx riaa"),
        (
            "swap",
            "swap - swap adjacent channel pairs",
            "render --fx swap",
        ),
    ] {
        let output = Command::new(env!("CARGO_BIN_EXE_auralis"))
            .args(["man", topic])
            .output()
            .unwrap();

        assert!(output.status.success(), "stderr: {}", stderr(&output));
        let stdout = stdout(&output);
        assert!(stdout.contains(summary), "{stdout}");
        assert!(stdout.contains(render_form), "{stdout}");
        assert!(stdout.contains("-o, --output FILE"), "{stdout}");
    }
}

#[test]
fn gain_man_page_describes_recipe_lowering() {
    let output = Command::new(env!("CARGO_BIN_EXE_auralis"))
        .args(["man", "gain"])
        .output()
        .unwrap();

    assert!(output.status.success(), "stderr: {}", stderr(&output));
    let stdout = stdout(&output);
    assert!(
        stdout.contains("gain - adjust one audio file by gain"),
        "{stdout}"
    );
    assert!(stdout.contains("render --fx 'gain ...'"), "{stdout}");
    assert!(stdout.contains("-o, --output FILE"), "{stdout}");
}

#[test]
fn distortion_man_pages_describe_recipe_lowering() {
    for (topic, summary, render_form, option) in [
        (
            "contrast",
            "contrast - enhance sample contrast",
            "render --fx 'contrast ...'",
            "--amount AMOUNT",
        ),
        (
            "overdrive",
            "overdrive - apply overdrive distortion",
            "render --fx 'overdrive ...'",
            "--color COLOR",
        ),
        (
            "saturation",
            "saturation - apply saturation distortion",
            "render --fx 'saturation ...'",
            "--parameter VALUE",
        ),
    ] {
        let output = Command::new(env!("CARGO_BIN_EXE_auralis"))
            .args(["man", topic])
            .output()
            .unwrap();

        assert!(output.status.success(), "stderr: {}", stderr(&output));
        let stdout = stdout(&output);
        assert!(stdout.contains(summary), "{stdout}");
        assert!(stdout.contains(render_form), "{stdout}");
        assert!(stdout.contains(option), "{stdout}");
        assert!(stdout.contains("-o, --output FILE"), "{stdout}");
    }
}

#[test]
fn fade_man_page_describes_recipe_lowering() {
    let output = Command::new(env!("CARGO_BIN_EXE_auralis"))
        .args(["man", "fade"])
        .output()
        .unwrap();

    assert!(output.status.success(), "stderr: {}", stderr(&output));
    let stdout = stdout(&output);
    assert!(
        stdout.contains("fade - fade one audio file in or out"),
        "{stdout}"
    );
    assert!(stdout.contains("render --fx 'fade ...'"), "{stdout}");
    assert!(stdout.contains("--in FRAMES"), "{stdout}");
    assert!(stdout.contains("--out FRAMES"), "{stdout}");
    assert!(stdout.contains("--curve CURVE"), "{stdout}");
}

#[test]
fn mix_man_page_describes_recipe_lowering() {
    let output = Command::new(env!("CARGO_BIN_EXE_auralis"))
        .args(["man", "mix"])
        .output()
        .unwrap();

    assert!(output.status.success(), "stderr: {}", stderr(&output));
    let stdout = stdout(&output);
    assert!(stdout.contains("mix - mix audio files"), "{stdout}");
    assert!(stdout.contains("render --combine mix"), "{stdout}");
    assert!(stdout.contains("-o, --output FILE"), "{stdout}");
}

#[test]
fn concat_man_page_describes_recipe_lowering() {
    let output = Command::new(env!("CARGO_BIN_EXE_auralis"))
        .args(["man", "concat"])
        .output()
        .unwrap();

    assert!(output.status.success(), "stderr: {}", stderr(&output));
    let stdout = stdout(&output);
    assert!(
        stdout.contains("concat - concatenate audio files"),
        "{stdout}"
    );
    assert!(stdout.contains("render --combine concatenate"), "{stdout}");
    assert!(stdout.contains("-o, --output FILE"), "{stdout}");
}

#[test]
fn mix_power_man_page_describes_recipe_lowering() {
    let output = Command::new(env!("CARGO_BIN_EXE_auralis"))
        .args(["man", "mix-power"])
        .output()
        .unwrap();

    assert!(output.status.success(), "stderr: {}", stderr(&output));
    let stdout = stdout(&output);
    assert!(
        stdout.contains("mix-power - equal-power mix audio files"),
        "{stdout}"
    );
    assert!(stdout.contains("render --combine mix-power"), "{stdout}");
    assert!(stdout.contains("-o, --output FILE"), "{stdout}");
}

#[test]
fn merge_man_page_describes_recipe_lowering() {
    let output = Command::new(env!("CARGO_BIN_EXE_auralis"))
        .args(["man", "merge"])
        .output()
        .unwrap();

    assert!(output.status.success(), "stderr: {}", stderr(&output));
    let stdout = stdout(&output);
    assert!(stdout.contains("merge - merge audio channels"), "{stdout}");
    assert!(stdout.contains("render --combine merge"), "{stdout}");
    assert!(stdout.contains("-o, --output FILE"), "{stdout}");
}

#[test]
fn multiply_man_page_describes_recipe_lowering() {
    let output = Command::new(env!("CARGO_BIN_EXE_auralis"))
        .args(["man", "multiply"])
        .output()
        .unwrap();

    assert!(output.status.success(), "stderr: {}", stderr(&output));
    let stdout = stdout(&output);
    assert!(
        stdout.contains("multiply - multiply audio files"),
        "{stdout}"
    );
    assert!(stdout.contains("render --combine multiply"), "{stdout}");
    assert!(stdout.contains("-o, --output FILE"), "{stdout}");
}

#[test]
fn man_rejects_unknown_topic() {
    let output = Command::new(env!("CARGO_BIN_EXE_auralis"))
        .args(["man", "missing"])
        .output()
        .unwrap();

    assert!(!output.status.success());
    let stderr = stderr(&output);
    assert!(
        stderr.contains("no built-in manual page for `missing`"),
        "{stderr}"
    );
}

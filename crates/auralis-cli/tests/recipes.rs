//! Integration tests for recipe-layer CLI commands.

mod support;

use support::*;

#[test]
fn trim_recipe_lowers_to_typed_render_trim() {
    let input = temp_path("auralis-cli-trim-recipe-input", "wav");
    let recipe_output = temp_path("auralis-cli-trim-recipe-output", "wav");
    let render_output = temp_path("auralis-cli-trim-render-output", "wav");
    write_pcm16_wav(
        &input,
        2,
        &[
            -1000, 1000, -2000, 2000, -3000, 3000, -4000, 4000, -5000, 5000,
        ],
    );

    let recipe = Command::new(env!("CARGO_BIN_EXE_auralis"))
        .args([
            "trim",
            input.to_str().unwrap(),
            "1..4",
            "-o",
            recipe_output.to_str().unwrap(),
        ])
        .output()
        .unwrap();
    let render = Command::new(env!("CARGO_BIN_EXE_auralis"))
        .args([
            "render",
            input.to_str().unwrap(),
            "-o",
            render_output.to_str().unwrap(),
            "--fx",
            "trim 1..4",
        ])
        .output()
        .unwrap();

    assert!(recipe.status.success(), "stderr: {}", stderr(&recipe));
    assert!(render.status.success(), "stderr: {}", stderr(&render));
    assert_eq!(
        read_pcm16_wav(&recipe_output),
        read_pcm16_wav(&render_output)
    );
    assert_eq!(
        read_pcm16_wav(&recipe_output),
        (2, vec![-2000, 2000, -3000, 3000, -4000, 4000])
    );

    fs::remove_file(input).unwrap();
    fs::remove_file(recipe_output).unwrap();
    fs::remove_file(render_output).unwrap();
}

#[test]
fn normalize_recipe_accepts_dbfs_suffix_and_uses_render_norm_policy() {
    let input = temp_path("auralis-cli-normalize-recipe-input", "wav");
    let recipe_output = temp_path("auralis-cli-normalize-recipe-output", "wav");
    let render_output = temp_path("auralis-cli-normalize-render-output", "wav");
    write_pcm16_wav(&input, 1, &[4096, -16_384]);

    let recipe = Command::new(env!("CARGO_BIN_EXE_auralis"))
        .args([
            "normalize",
            input.to_str().unwrap(),
            "-o",
            recipe_output.to_str().unwrap(),
            "--peak",
            "-12dBFS",
        ])
        .output()
        .unwrap();
    let render = Command::new(env!("CARGO_BIN_EXE_auralis"))
        .args([
            "render",
            input.to_str().unwrap(),
            "-o",
            render_output.to_str().unwrap(),
            "--norm",
            "-12",
        ])
        .output()
        .unwrap();

    assert!(recipe.status.success(), "stderr: {}", stderr(&recipe));
    assert!(render.status.success(), "stderr: {}", stderr(&render));
    assert_eq!(
        read_pcm16_wav(&recipe_output),
        read_pcm16_wav(&render_output)
    );

    fs::remove_file(input).unwrap();
    fs::remove_file(recipe_output).unwrap();
    fs::remove_file(render_output).unwrap();
}

#[test]
fn gain_recipe_lowers_to_typed_render_gain() {
    let input = temp_path("auralis-cli-gain-recipe-input", "wav");
    let recipe_output = temp_path("auralis-cli-gain-recipe-output", "wav");
    let render_output = temp_path("auralis-cli-gain-render-output", "wav");
    write_pcm16_wav(&input, 1, &[1024, -2048, 4096, -8192]);

    let recipe = Command::new(env!("CARGO_BIN_EXE_auralis"))
        .args([
            "gain",
            input.to_str().unwrap(),
            "-6dB",
            "-o",
            recipe_output.to_str().unwrap(),
        ])
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

    assert!(recipe.status.success(), "stderr: {}", stderr(&recipe));
    assert!(render.status.success(), "stderr: {}", stderr(&render));
    assert_eq!(
        read_pcm16_wav(&recipe_output),
        read_pcm16_wav(&render_output)
    );

    fs::remove_file(input).unwrap();
    fs::remove_file(recipe_output).unwrap();
    fs::remove_file(render_output).unwrap();
}

#[test]
fn reverse_recipe_lowers_to_typed_render_reverse() {
    let input = temp_path("auralis-cli-reverse-recipe-input", "wav");
    let recipe_output = temp_path("auralis-cli-reverse-recipe-output", "wav");
    let render_output = temp_path("auralis-cli-reverse-render-output", "wav");
    write_pcm16_wav(
        &input,
        2,
        &[
            -1000, 1000, -2000, 2000, -3000, 3000, -4000, 4000, -5000, 5000,
        ],
    );

    let recipe = Command::new(env!("CARGO_BIN_EXE_auralis"))
        .args([
            "reverse",
            input.to_str().unwrap(),
            "-o",
            recipe_output.to_str().unwrap(),
        ])
        .output()
        .unwrap();
    let render = Command::new(env!("CARGO_BIN_EXE_auralis"))
        .args([
            "render",
            input.to_str().unwrap(),
            "-o",
            render_output.to_str().unwrap(),
            "--fx",
            "reverse",
        ])
        .output()
        .unwrap();

    assert!(recipe.status.success(), "stderr: {}", stderr(&recipe));
    assert!(render.status.success(), "stderr: {}", stderr(&render));
    assert_eq!(
        read_pcm16_wav(&recipe_output),
        read_pcm16_wav(&render_output)
    );
    assert_eq!(
        read_pcm16_wav(&recipe_output),
        (
            2,
            vec![
                -5000, 5000, -4000, 4000, -3000, 3000, -2000, 2000, -1000, 1000
            ]
        )
    );

    fs::remove_file(input).unwrap();
    fs::remove_file(recipe_output).unwrap();
    fs::remove_file(render_output).unwrap();
}

#[test]
fn no_arg_effect_recipes_lower_to_typed_render_effects() {
    let cases = [
        ("deemph", 48_000, 1, vec![10_000, -8_000, 6_000, -4_000]),
        (
            "earwax",
            44_100,
            2,
            vec![12_000, -12_000, 8_000, -8_000, 4_000, -4_000],
        ),
        ("oops", 48_000, 2, vec![16_384, 0, 0, 16_384]),
        ("riaa", 48_000, 1, vec![4_000, -2_000, 1_000, -500]),
        ("swap", 48_000, 2, vec![1000, -1000, 2000, -2000]),
    ];

    for (effect, sample_rate, channels, samples) in cases {
        assert_recipe_matches_render_effect(effect, sample_rate, channels, &samples);
    }
}

#[test]
fn stereo_channel_recipes_preserve_expected_sample_semantics() {
    let oops_output = recipe_effect_output("oops", 48_000, 2, &[16_384, 0, 0, 16_384]);
    let swap_output = recipe_effect_output("swap", 48_000, 2, &[1000, -1000, 2000, -2000]);

    assert_eq!(oops_output, (2, vec![16_384, 16_384, -16_384, -16_384]));
    assert_eq!(swap_output, (2, vec![-1000, 1000, -2000, 2000]));
}

#[test]
fn distortion_recipes_lower_to_typed_render_effects() {
    let cases = [
        (
            "contrast",
            vec!["--amount", "25"],
            "contrast 25",
            vec![-18_000, -6_000, 0, 6_000, 18_000],
        ),
        (
            "overdrive",
            vec!["--gain", "12", "--color", "25"],
            "overdrive 12 25",
            vec![-18_000, -6_000, 0, 6_000, 18_000],
        ),
        (
            "saturation",
            vec![
                "--type",
                "sqrt",
                "--blend",
                "0.75",
                "--offset",
                "0.1",
                "--parameter",
                "0.25",
            ],
            "saturation sqrt 0.75 0.1 0.25",
            vec![-18_000, -6_000, 0, 6_000, 18_000],
        ),
    ];

    for (effect, recipe_args, render_fx, samples) in cases {
        assert_recipe_with_args_matches_render_effect(effect, &recipe_args, render_fx, &samples);
    }
}

#[test]
fn level_and_modulation_recipes_lower_to_typed_render_effects() {
    let cases = [
        (
            "dcshift",
            vec!["0.1", "--limiter-gain", "0.02"],
            "dcshift 0.1 0.02",
            vec![-8000, -4000, 0, 4000, 8000],
        ),
        (
            "vol",
            vec!["0.25", "--type", "power"],
            "vol 0.25 power",
            vec![-12000, -6000, 0, 6000, 12000],
        ),
        (
            "softvol",
            vec!["--volume", "0.5", "--double-time", "0", "--headroom", "0"],
            "softvol 0.5 0 0",
            vec![-12000, -6000, 0, 6000, 12000],
        ),
        (
            "tremolo",
            vec!["5", "--depth", "60"],
            "tremolo 5 60",
            vec![12000, 12000, 12000, 12000, 12000, 12000],
        ),
    ];

    for (effect, recipe_args, render_fx, samples) in cases {
        assert_recipe_with_args_matches_render_effect(effect, &recipe_args, render_fx, &samples);
    }
}

#[test]
fn time_and_pitch_recipes_lower_to_typed_render_effects() {
    let cases = [
        ("speed", vec!["1.25"], "speed 1.25"),
        (
            "tempo",
            vec![
                "1.25",
                "--quick",
                "--profile",
                "speech",
                "--segment",
                "60",
                "--search",
                "10",
                "--overlap",
                "8",
            ],
            "tempo -q -s 1.25 60 10 8",
        ),
        (
            "pitch",
            vec![
                "-1200",
                "--quick",
                "--segment",
                "60",
                "--search",
                "10",
                "--overlap",
                "8",
            ],
            "pitch -q -1200 60 10 8",
        ),
    ];

    for (effect, recipe_args, render_fx) in cases {
        assert_recipe_with_args_matches_render_effect(
            effect,
            &recipe_args,
            render_fx,
            &[-12000, -6000, 0, 6000, 12000, 6000, 0, -6000],
        );
    }
}

#[test]
fn eq_recipes_lower_to_typed_render_effects() {
    let cases = [
        (
            "bass",
            vec!["6", "--frequency", "120", "--width", "0.707q"],
            "bass 6 120 0.707q",
        ),
        (
            "treble",
            vec!["-3", "--frequency", "4k", "--width", "1o"],
            "treble -3 4k 1o",
        ),
        (
            "equalizer",
            vec!["--frequency", "1k", "--width", "500", "--gain", "-2"],
            "equalizer 1k 500 -2",
        ),
    ];

    for (effect, recipe_args, render_fx) in cases {
        assert_recipe_with_args_matches_render_effect(
            effect,
            &recipe_args,
            render_fx,
            &[-12000, -6000, 0, 6000, 12000, 6000, 0, -6000],
        );
    }
}

#[test]
fn filter_recipes_lower_to_typed_render_effects() {
    let cases = [
        (
            "allpass",
            vec!["--frequency", "1k", "--width", "0.707q"],
            "allpass 1k 0.707q",
        ),
        (
            "band",
            vec!["--frequency", "750", "--width", "1o", "--unpitched"],
            "band -n 750 1o",
        ),
        (
            "bandpass",
            vec!["--frequency", "1k", "--width", "500", "--constant-skirt"],
            "bandpass -c 1k 500",
        ),
        (
            "bandreject",
            vec!["--frequency", "2k", "--width", "0.5k"],
            "bandreject 2k 0.5k",
        ),
        (
            "highpass",
            vec!["--frequency", "300", "--width", "0.707q"],
            "highpass 300 0.707q",
        ),
        (
            "lowpass",
            vec!["--frequency", "3k", "--poles", "1"],
            "lowpass -1 3k",
        ),
    ];

    for (effect, recipe_args, render_fx) in cases {
        assert_recipe_with_args_matches_render_effect(
            effect,
            &recipe_args,
            render_fx,
            &[-12000, -6000, 0, 6000, 12000, 6000, 0, -6000],
        );
    }
}

#[test]
fn echo_recipes_lower_to_typed_render_effects() {
    let cases = [
        (
            "echo",
            vec![
                "--gain-in",
                "0.5",
                "--gain-out",
                "1",
                "--tap",
                "1,0.25",
                "--tap",
                "2,-0.125",
            ],
            "echo 0.5 1 1 0.25 2 -0.125",
        ),
        (
            "echos",
            vec![
                "--gain-in",
                "0.5",
                "--gain-out",
                "1",
                "--tap",
                "1,0.25",
                "--tap",
                "2,0.125",
            ],
            "echos 0.5 1 1 0.25 2 0.125",
        ),
    ];

    for (effect, recipe_args, render_fx) in cases {
        assert_recipe_with_args_matches_render_effect(
            effect,
            &recipe_args,
            render_fx,
            &[12_000, 0, -6_000, 0, 3_000],
        );
    }
}

#[test]
fn modulation_recipes_lower_to_typed_render_effects() {
    let cases = [
        (
            "chorus",
            vec![
                "--gain-in",
                "0.6",
                "--gain-out",
                "0.8",
                "--interpolation",
                "quadratic",
                "--wave",
                "triangle",
                "--stage",
                "1,0.25,1,0",
                "--stage",
                "2,-0.125,1,0,sine",
            ],
            "chorus -q -t 0.6 0.8 1 0.25 1 0 2 -0.125 1 0 -sine",
        ),
        (
            "flanger",
            vec![
                "--delay",
                "1",
                "--depth",
                "2",
                "--regen",
                "25",
                "--width",
                "100",
                "--speed",
                "1",
                "--wave",
                "sine",
                "--phase",
                "50",
                "--interpolation",
                "none",
            ],
            "flanger 1 2 25 100 1 sine 50 none",
        ),
        (
            "phaser",
            vec![
                "--gain-in",
                "0.8",
                "--gain-out",
                "0.74",
                "--delay",
                "3",
                "--regen",
                "0.4",
                "--speed",
                "0.5",
                "--wave",
                "sine",
                "--interpolation",
                "quadratic",
            ],
            "phaser -q -s 0.8 0.74 3 0.4 0.5",
        ),
    ];

    for (effect, recipe_args, render_fx) in cases {
        assert_recipe_with_args_matches_render_effect(
            effect,
            &recipe_args,
            render_fx,
            &[12_000, 0, -6_000, 0, 3_000],
        );
    }
}

#[test]
fn fade_recipe_lowers_to_typed_render_fade() {
    let input = temp_path("auralis-cli-fade-recipe-input", "wav");
    let recipe_output = temp_path("auralis-cli-fade-recipe-output", "wav");
    let render_output = temp_path("auralis-cli-fade-render-output", "wav");
    write_pcm16_wav(
        &input,
        2,
        &[-10000, 10000, -20000, 20000, -30000, 30000, -4000, 4000],
    );

    let recipe = Command::new(env!("CARGO_BIN_EXE_auralis"))
        .args([
            "fade",
            input.to_str().unwrap(),
            "--in",
            "2",
            "--out",
            "2",
            "--curve",
            "linear",
            "-o",
            recipe_output.to_str().unwrap(),
        ])
        .output()
        .unwrap();
    let render = Command::new(env!("CARGO_BIN_EXE_auralis"))
        .args([
            "render",
            input.to_str().unwrap(),
            "-o",
            render_output.to_str().unwrap(),
            "--fx",
            "fade in=2 out=2 curve=linear",
        ])
        .output()
        .unwrap();

    assert!(recipe.status.success(), "stderr: {}", stderr(&recipe));
    assert!(render.status.success(), "stderr: {}", stderr(&render));
    assert_eq!(
        read_pcm16_wav(&recipe_output),
        read_pcm16_wav(&render_output)
    );
    assert_eq!(
        read_pcm16_wav(&recipe_output),
        (2, vec![0, 0, -10000, 10000, -30000, 30000, -2000, 2000])
    );

    fs::remove_file(input).unwrap();
    fs::remove_file(recipe_output).unwrap();
    fs::remove_file(render_output).unwrap();
}

#[test]
fn structural_recipes_lower_to_typed_render_effects() {
    let cases = [
        ("delay", vec!["--position", "1s"], "delay 1s"),
        (
            "pad",
            vec!["--start", "1", "--at", "2@2", "--end", "1"],
            "pad 1 2@2 1",
        ),
        ("repeat", vec!["2"], "repeat 2"),
        ("downsample", vec!["3"], "downsample 3"),
        ("upsample", vec!["3"], "upsample 3"),
    ];

    for (effect, recipe_args, render_fx) in cases {
        assert_recipe_with_args_matches_render_effect(
            effect,
            &recipe_args,
            render_fx,
            &[1000, -2000, 3000],
        );
    }
}

#[test]
fn fir_and_quantization_recipes_lower_to_typed_render_effects() {
    let cases = [
        ("hilbert", vec!["--taps", "5"], "hilbert -n 5"),
        (
            "loudness",
            vec!["--gain", "-6", "--reference", "70", "--half-points", "127"],
            "loudness -6 70 127",
        ),
        (
            "dither",
            vec!["--sloped", "--precision", "12"],
            "dither -S -p 12",
        ),
    ];

    for (effect, recipe_args, render_fx) in cases {
        assert_recipe_with_args_matches_render_effect(
            effect,
            &recipe_args,
            render_fx,
            &[1000, -2000, 3000, -4000, 5000, -6000],
        );
    }
}

#[test]
fn reverb_and_stretch_recipes_lower_to_typed_render_effects() {
    let cases = [
        (
            "reverb",
            vec![
                "--wet-only",
                "--reverberance",
                "75",
                "--hf-damping",
                "25",
                "--room-scale",
                "50",
                "--stereo-depth",
                "0",
                "--pre-delay",
                "10",
                "--wet-gain",
                "-3",
            ],
            "reverb -w 75 25 50 0 10 -3",
        ),
        (
            "stretch",
            vec![
                "1.5", "--window", "10", "--fade", "quarter", "--shift", "0.75", "--fading", "0.25",
            ],
            "stretch 1.5 10 q 0.75 0.25",
        ),
    ];

    for (effect, recipe_args, render_fx) in cases {
        assert_recipe_with_args_matches_render_effect(
            effect,
            &recipe_args,
            render_fx,
            &[1000, -2000, 3000, -4000, 5000, -6000],
        );
    }
}

fn assert_recipe_with_args_matches_render_effect(
    effect: &str,
    recipe_args: &[&str],
    render_fx: &str,
    samples: &[i16],
) {
    let input = temp_path(&format!("auralis-cli-{effect}-recipe-input"), "wav");
    let recipe_output = temp_path(&format!("auralis-cli-{effect}-recipe-output"), "wav");
    let render_output = temp_path(&format!("auralis-cli-{effect}-render-output"), "wav");
    write_pcm16_wav(&input, 1, samples);

    let mut recipe_command = Command::new(env!("CARGO_BIN_EXE_auralis"));
    recipe_command
        .arg(effect)
        .arg(input.to_str().unwrap())
        .args(recipe_args)
        .args(["-o", recipe_output.to_str().unwrap()]);
    let recipe = recipe_command.output().unwrap();
    let render = Command::new(env!("CARGO_BIN_EXE_auralis"))
        .args([
            "render",
            input.to_str().unwrap(),
            "-o",
            render_output.to_str().unwrap(),
            "--fx",
            render_fx,
        ])
        .output()
        .unwrap();

    assert!(recipe.status.success(), "stderr: {}", stderr(&recipe));
    assert!(render.status.success(), "stderr: {}", stderr(&render));
    assert_eq!(
        read_pcm16_wav(&recipe_output),
        read_pcm16_wav(&render_output)
    );

    fs::remove_file(input).unwrap();
    fs::remove_file(recipe_output).unwrap();
    fs::remove_file(render_output).unwrap();
}

fn assert_recipe_matches_render_effect(
    effect: &str,
    sample_rate: u32,
    channels: u16,
    samples: &[i16],
) {
    let input = temp_path(&format!("auralis-cli-{effect}-recipe-input"), "wav");
    let recipe_output = temp_path(&format!("auralis-cli-{effect}-recipe-output"), "wav");
    let render_output = temp_path(&format!("auralis-cli-{effect}-render-output"), "wav");
    write_pcm16_wav_with_sample_rate(&input, sample_rate, channels, samples);

    let recipe = Command::new(env!("CARGO_BIN_EXE_auralis"))
        .args([
            effect,
            input.to_str().unwrap(),
            "-o",
            recipe_output.to_str().unwrap(),
        ])
        .output()
        .unwrap();
    let render = Command::new(env!("CARGO_BIN_EXE_auralis"))
        .args([
            "render",
            input.to_str().unwrap(),
            "-o",
            render_output.to_str().unwrap(),
            "--fx",
            effect,
        ])
        .output()
        .unwrap();

    assert!(recipe.status.success(), "stderr: {}", stderr(&recipe));
    assert!(render.status.success(), "stderr: {}", stderr(&render));
    assert_eq!(
        read_pcm16_wav(&recipe_output),
        read_pcm16_wav(&render_output)
    );

    fs::remove_file(input).unwrap();
    fs::remove_file(recipe_output).unwrap();
    fs::remove_file(render_output).unwrap();
}

fn recipe_effect_output(
    effect: &str,
    sample_rate: u32,
    channels: u16,
    samples: &[i16],
) -> (u16, Vec<i16>) {
    let input = temp_path(&format!("auralis-cli-{effect}-semantics-input"), "wav");
    let output = temp_path(&format!("auralis-cli-{effect}-semantics-output"), "wav");
    write_pcm16_wav_with_sample_rate(&input, sample_rate, channels, samples);

    let recipe = Command::new(env!("CARGO_BIN_EXE_auralis"))
        .args([
            effect,
            input.to_str().unwrap(),
            "-o",
            output.to_str().unwrap(),
        ])
        .output()
        .unwrap();

    assert!(recipe.status.success(), "stderr: {}", stderr(&recipe));
    let actual = read_pcm16_wav(&output);

    fs::remove_file(input).unwrap();
    fs::remove_file(output).unwrap();

    actual
}

#[test]
fn mix_recipe_lowers_to_typed_render_mix() {
    let first = temp_path("auralis-cli-mix-recipe-first", "wav");
    let second = temp_path("auralis-cli-mix-recipe-second", "wav");
    let third = temp_path("auralis-cli-mix-recipe-third", "wav");
    let recipe_output = temp_path("auralis-cli-mix-recipe-output", "wav");
    let render_output = temp_path("auralis-cli-mix-render-output", "wav");
    write_pcm16_wav(&first, 1, &[1200, -1200, 600]);
    write_pcm16_wav(&second, 1, &[600, 0, -600]);
    write_pcm16_wav(&third, 1, &[0, 900, -300]);

    let recipe = Command::new(env!("CARGO_BIN_EXE_auralis"))
        .args([
            "mix",
            first.to_str().unwrap(),
            second.to_str().unwrap(),
            third.to_str().unwrap(),
            "-o",
            recipe_output.to_str().unwrap(),
        ])
        .output()
        .unwrap();
    let render = Command::new(env!("CARGO_BIN_EXE_auralis"))
        .args([
            "render",
            first.to_str().unwrap(),
            "-o",
            render_output.to_str().unwrap(),
            "--combine",
            "mix",
            "--input",
            second.to_str().unwrap(),
            "--input",
            third.to_str().unwrap(),
        ])
        .output()
        .unwrap();

    assert!(recipe.status.success(), "stderr: {}", stderr(&recipe));
    assert!(render.status.success(), "stderr: {}", stderr(&render));
    assert_eq!(
        read_pcm16_wav(&recipe_output),
        read_pcm16_wav(&render_output)
    );
    assert_eq!(read_pcm16_wav(&recipe_output), (1, vec![600, -100, -100]));

    fs::remove_file(first).unwrap();
    fs::remove_file(second).unwrap();
    fs::remove_file(third).unwrap();
    fs::remove_file(recipe_output).unwrap();
    fs::remove_file(render_output).unwrap();
}

#[test]
fn concat_recipe_lowers_to_typed_render_concatenate() {
    let first = temp_path("auralis-cli-concat-recipe-first", "wav");
    let second = temp_path("auralis-cli-concat-recipe-second", "wav");
    let third = temp_path("auralis-cli-concat-recipe-third", "wav");
    let recipe_output = temp_path("auralis-cli-concat-recipe-output", "wav");
    let render_output = temp_path("auralis-cli-concat-render-output", "wav");
    write_pcm16_wav(&first, 1, &[100, 200]);
    write_pcm16_wav(&second, 1, &[-300, -400, -500]);
    write_pcm16_wav(&third, 1, &[600]);

    let recipe = Command::new(env!("CARGO_BIN_EXE_auralis"))
        .args([
            "concat",
            first.to_str().unwrap(),
            second.to_str().unwrap(),
            third.to_str().unwrap(),
            "-o",
            recipe_output.to_str().unwrap(),
        ])
        .output()
        .unwrap();
    let render = Command::new(env!("CARGO_BIN_EXE_auralis"))
        .args([
            "render",
            first.to_str().unwrap(),
            "-o",
            render_output.to_str().unwrap(),
            "--combine",
            "concatenate",
            "--input",
            second.to_str().unwrap(),
            "--input",
            third.to_str().unwrap(),
        ])
        .output()
        .unwrap();

    assert!(recipe.status.success(), "stderr: {}", stderr(&recipe));
    assert!(render.status.success(), "stderr: {}", stderr(&render));
    assert_eq!(
        read_pcm16_wav(&recipe_output),
        read_pcm16_wav(&render_output)
    );
    assert_eq!(
        read_pcm16_wav(&recipe_output),
        (1, vec![100, 200, -300, -400, -500, 600])
    );

    fs::remove_file(first).unwrap();
    fs::remove_file(second).unwrap();
    fs::remove_file(third).unwrap();
    fs::remove_file(recipe_output).unwrap();
    fs::remove_file(render_output).unwrap();
}

#[test]
fn mix_power_recipe_lowers_to_typed_render_mix_power() {
    let first = temp_path("auralis-cli-mix-power-recipe-first", "wav");
    let second = temp_path("auralis-cli-mix-power-recipe-second", "wav");
    let recipe_output = temp_path("auralis-cli-mix-power-recipe-output", "wav");
    let render_output = temp_path("auralis-cli-mix-power-render-output", "wav");
    write_pcm16_wav(&first, 1, &[1000, -1000]);
    write_pcm16_wav(&second, 1, &[3000, 1000]);

    let recipe = Command::new(env!("CARGO_BIN_EXE_auralis"))
        .args([
            "mix-power",
            first.to_str().unwrap(),
            second.to_str().unwrap(),
            "-o",
            recipe_output.to_str().unwrap(),
        ])
        .output()
        .unwrap();
    let render = Command::new(env!("CARGO_BIN_EXE_auralis"))
        .args([
            "render",
            first.to_str().unwrap(),
            "-o",
            render_output.to_str().unwrap(),
            "--combine",
            "mix-power",
            "--input",
            second.to_str().unwrap(),
        ])
        .output()
        .unwrap();

    assert!(recipe.status.success(), "stderr: {}", stderr(&recipe));
    assert!(render.status.success(), "stderr: {}", stderr(&render));
    assert_eq!(
        read_pcm16_wav(&recipe_output),
        read_pcm16_wav(&render_output)
    );
    assert_eq!(read_pcm16_wav(&recipe_output), (1, vec![2828, 0]));

    fs::remove_file(first).unwrap();
    fs::remove_file(second).unwrap();
    fs::remove_file(recipe_output).unwrap();
    fs::remove_file(render_output).unwrap();
}

#[test]
fn merge_recipe_lowers_to_typed_render_merge() {
    let first = temp_path("auralis-cli-merge-recipe-first", "wav");
    let second = temp_path("auralis-cli-merge-recipe-second", "wav");
    let recipe_output = temp_path("auralis-cli-merge-recipe-output", "wav");
    let render_output = temp_path("auralis-cli-merge-render-output", "wav");
    write_pcm16_wav(&first, 1, &[1000, -1000]);
    write_pcm16_wav(&second, 1, &[3000, 1000]);

    let recipe = Command::new(env!("CARGO_BIN_EXE_auralis"))
        .args([
            "merge",
            first.to_str().unwrap(),
            second.to_str().unwrap(),
            "-o",
            recipe_output.to_str().unwrap(),
        ])
        .output()
        .unwrap();
    let render = Command::new(env!("CARGO_BIN_EXE_auralis"))
        .args([
            "render",
            first.to_str().unwrap(),
            "-o",
            render_output.to_str().unwrap(),
            "--combine",
            "merge",
            "--input",
            second.to_str().unwrap(),
        ])
        .output()
        .unwrap();

    assert!(recipe.status.success(), "stderr: {}", stderr(&recipe));
    assert!(render.status.success(), "stderr: {}", stderr(&render));
    assert_eq!(
        read_pcm16_wav(&recipe_output),
        read_pcm16_wav(&render_output)
    );
    assert_eq!(
        read_pcm16_wav(&recipe_output),
        (2, vec![1000, 3000, -1000, 1000])
    );

    fs::remove_file(first).unwrap();
    fs::remove_file(second).unwrap();
    fs::remove_file(recipe_output).unwrap();
    fs::remove_file(render_output).unwrap();
}

#[test]
fn multiply_recipe_lowers_to_typed_render_multiply() {
    let first = temp_path("auralis-cli-multiply-recipe-first", "wav");
    let second = temp_path("auralis-cli-multiply-recipe-second", "wav");
    let recipe_output = temp_path("auralis-cli-multiply-recipe-output", "wav");
    let render_output = temp_path("auralis-cli-multiply-render-output", "wav");
    write_pcm16_wav(&first, 1, &[16_384, -16_384]);
    write_pcm16_wav(&second, 1, &[8192, 16_384]);

    let recipe = Command::new(env!("CARGO_BIN_EXE_auralis"))
        .args([
            "multiply",
            first.to_str().unwrap(),
            second.to_str().unwrap(),
            "-o",
            recipe_output.to_str().unwrap(),
        ])
        .output()
        .unwrap();
    let render = Command::new(env!("CARGO_BIN_EXE_auralis"))
        .args([
            "render",
            first.to_str().unwrap(),
            "-o",
            render_output.to_str().unwrap(),
            "--combine",
            "multiply",
            "--input",
            second.to_str().unwrap(),
        ])
        .output()
        .unwrap();

    assert!(recipe.status.success(), "stderr: {}", stderr(&recipe));
    assert!(render.status.success(), "stderr: {}", stderr(&render));
    assert_eq!(
        read_pcm16_wav(&recipe_output),
        read_pcm16_wav(&render_output)
    );
    assert_eq!(read_pcm16_wav(&recipe_output), (1, vec![4096, -8192]));

    fs::remove_file(first).unwrap();
    fs::remove_file(second).unwrap();
    fs::remove_file(recipe_output).unwrap();
    fs::remove_file(render_output).unwrap();
}

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

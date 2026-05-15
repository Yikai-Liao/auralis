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

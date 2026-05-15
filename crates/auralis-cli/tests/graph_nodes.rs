//! Graph node execution coverage for registry-backed effect parameters.

mod support;

use support::{Command, fs, read_pcm16_wav_with_sample_rate, stderr, temp_path, write_pcm16_wav};

fn assert_graph_node_matches_render(
    name: &str,
    samples: &[i16],
    op: &str,
    params: &str,
    render_fx: &str,
) {
    let spec = temp_path(&format!("auralis-cli-graph-node-{name}-spec"), "toml");
    let input = temp_path(&format!("auralis-cli-graph-node-{name}-input"), "wav");
    let graph_output = temp_path(&format!("auralis-cli-graph-node-{name}-output"), "wav");
    let render_output = temp_path(&format!("auralis-cli-graph-node-{name}-render"), "wav");
    write_pcm16_wav(&input, 1, samples);
    fs::write(
        &spec,
        format!(
            r#"version = "auralis.graph/v1"

[[sources]]
id = "voice"
path = "{}"

[[nodes]]
id = "processed"
op = "{}"
input = "voice.audio"
{}

[[sinks]]
id = "wav"
input = "processed.audio"
path = "{}"
"#,
            input.display(),
            op,
            params,
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
            render_fx,
        ])
        .output()
        .unwrap();

    let _ = fs::remove_file(&spec);
    let _ = fs::remove_file(&input);
    assert!(graph.status.success(), "{}", stderr(&graph));
    assert!(render.status.success(), "{}", stderr(&render));
    assert_eq!(
        read_pcm16_wav_with_sample_rate(&graph_output),
        read_pcm16_wav_with_sample_rate(&render_output)
    );
    let _ = fs::remove_file(&graph_output);
    let _ = fs::remove_file(&render_output);
}

#[test]
fn graph_nodes_lower_named_tone_filter_parameters() {
    let samples = &[1000, -2000, 3000, -4000, 5000, -6000, 7000, -8000];

    assert_graph_node_matches_render(
        "bass",
        samples,
        "bass",
        r#"gain = "-3"
frequency = "120"
width = "0.707q""#,
        "bass -3 120 0.707q",
    );
    assert_graph_node_matches_render(
        "equalizer",
        samples,
        "equalizer",
        r#"frequency = "1000"
width = "0.707q"
gain = "3""#,
        "equalizer 1000 0.707q 3",
    );
    assert_graph_node_matches_render(
        "lowpass",
        samples,
        "filter.lowpass",
        r#"frequency = "12000Hz"
q = 0.707"#,
        "lowpass 12000 0.707q",
    );
}

#[test]
fn graph_nodes_lower_named_flagged_filter_parameters() {
    let samples = &[1000, -2000, 3000, -4000, 5000, -6000, 7000, -8000];

    assert_graph_node_matches_render(
        "band",
        samples,
        "band",
        r#"frequency = "1000"
width = "0.707q"
unpitched = true"#,
        "band -n 1000 0.707q",
    );
    assert_graph_node_matches_render(
        "bandpass",
        samples,
        "bandpass",
        r#"frequency = "1000"
width = "0.707q"
constant_skirt = true"#,
        "bandpass -c 1000 0.707q",
    );
}

#[test]
fn graph_nodes_lower_named_scalar_effect_parameters() {
    let samples = &[2000, -3000, 4000, -5000, 6000, -7000, 8000, -9000];

    assert_graph_node_matches_render(
        "contrast",
        samples,
        "contrast",
        r#"amount = "25""#,
        "contrast 25",
    );
    assert_graph_node_matches_render(
        "overdrive",
        samples,
        "overdrive",
        r#"gain = "12"
color = "25""#,
        "overdrive 12 25",
    );
    assert_graph_node_matches_render(
        "softvol",
        samples,
        "softvol",
        r#"volume = "0.5"
double_time = "0"
headroom = "0""#,
        "softvol 0.5 0 0",
    );
    assert_graph_node_matches_render(
        "tremolo",
        samples,
        "tremolo",
        r#"speed = "5"
depth = "50""#,
        "tremolo 5 50",
    );
}

#[test]
fn graph_nodes_lower_named_resampling_and_repeat_parameters() {
    let samples = &[1000, 2000, 3000, 4000, 5000, 6000];

    assert_graph_node_matches_render("repeat", samples, "repeat", r#"count = "1""#, "repeat 1");
    assert_graph_node_matches_render(
        "downsample",
        samples,
        "downsample",
        r#"factor = "2""#,
        "downsample 2",
    );
    assert_graph_node_matches_render(
        "upsample",
        samples,
        "upsample",
        r#"factor = "2""#,
        "upsample 2",
    );
}

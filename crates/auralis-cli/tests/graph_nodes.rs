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
        read_pcm16_wav_with_sample_rate(&render_output),
        "{name}"
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
        "vol",
        samples,
        "vol",
        r#"gain = "2"
type = "amplitude"
limiter_gain = "0.05""#,
        "vol 2 amplitude 0.05",
    );
    assert_graph_node_matches_render(
        "saturation",
        samples,
        "saturation",
        r#"type = "sqrt"
blend = "0.75"
offset = "0.1"
parameter = "0.25""#,
        "saturation sqrt 0.75 0.1 0.25",
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
fn graph_nodes_lower_named_echo_parameters() {
    let samples = &[1000, 0, 2000, 0, -1000, 0, -2000, 0];

    assert_graph_node_matches_render(
        "echo",
        samples,
        "echo",
        r#"gain_in = "0.8"
gain_out = "0.9"
taps = [
  { delay = "1", decay = "0.5" },
  { delay = "2", decay = "-0.25" },
]"#,
        "echo 0.8 0.9 1 0.5 2 -0.25",
    );
    assert_graph_node_matches_render(
        "echos",
        samples,
        "echos",
        r#"gain_in = "0.8"
gain_out = "0.9"
taps = [
  { delay = "1", decay = "0.5" },
  { delay = "2", decay = "0.25" },
]"#,
        "echos 0.8 0.9 1 0.5 2 0.25",
    );
}

#[test]
fn graph_nodes_lower_named_modulation_parameters() {
    let samples = &[
        0, 1200, 2400, 3600, 4800, 6000, 4800, 3600, 2400, 1200, 0, -1200, -2400, -3600, -4800,
        -6000,
    ];

    assert_graph_node_matches_render(
        "flanger",
        samples,
        "flanger",
        r#"delay = "1"
depth = "2"
regen = "0"
width = "71"
speed = "0.5"
wave = "triangle"
phase = "25"
interpolation = "quadratic""#,
        "flanger -q -t 1 2 0 71 0.5 triangle 25",
    );
    assert_graph_node_matches_render(
        "chorus",
        samples,
        "chorus",
        r#"gain_in = "0.6"
gain_out = "0.8"
interpolation = "quadratic"
wave = "triangle"
stages = [
  { delay = "1", decay = "0.25", speed = "1", depth = "0" },
  { delay = "2", decay = "-0.125", speed = "1", depth = "0", wave = "sine" },
]"#,
        "chorus -q -t 0.6 0.8 1 0.25 1 0 2 -0.125 1 0 -sine",
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

#[test]
fn graph_nodes_lower_named_routing_and_analysis_parameters() {
    let samples = &[1000, -2000, 3000, -4000, 5000, -6000];

    assert_graph_node_matches_render(
        "remix",
        samples,
        "remix",
        r#"level_mode = "automatic"
mix_power = true
outputs = ["1", "0"]"#,
        "remix -a -p 1 0",
    );
    assert_graph_node_matches_render(
        "stat",
        samples,
        "stat",
        r#"scale = "2"
rms = true
volume_only = true
json = true"#,
        "stat -s 2 -rms -v -j",
    );
    assert_graph_node_matches_render(
        "stats",
        samples,
        "stats",
        r#"signed_bits = "16"
window = "0.01"
json = true"#,
        "stats -b 16 -w 0.01 -j",
    );
}

#[test]
fn graph_nodes_lower_named_sinc_parameters() {
    let samples = &[1000, -2000, 3000, -4000, 5000, -6000, 7000, -8000];

    assert_graph_node_matches_render(
        "sinc",
        samples,
        "sinc",
        r#"beta = "8"
taps = "11"
round_taps = true
range = "1000-4000""#,
        "sinc -b 8 -n 11 -r 1000-4000",
    );
}

#[test]
fn graph_nodes_lower_named_structural_and_fir_parameters() {
    let samples = &[1000, -2000, 3000, -4000, 5000, -6000];

    assert_graph_node_matches_render("delay", samples, "delay", r#"positions = ["1"]"#, "delay 1");
    assert_graph_node_matches_render(
        "pad",
        samples,
        "pad",
        r#"start = "1"
positioned = ["2@2"]
end = "1""#,
        "pad 1 2@2 1",
    );
    assert_graph_node_matches_render(
        "hilbert",
        samples,
        "hilbert",
        r#"taps = "5""#,
        "hilbert -n 5",
    );
    assert_graph_node_matches_render(
        "loudness",
        samples,
        "loudness",
        r#"gain = "-6"
reference = "70"
half_points = "127""#,
        "loudness -6 70 127",
    );
}

#[test]
fn graph_nodes_lower_named_quantization_and_reverb_parameters() {
    let samples = &[1000, -2000, 3000, -4000, 5000, -6000];

    assert_graph_node_matches_render(
        "dither",
        samples,
        "dither",
        r#"sloped = true
precision = "12""#,
        "dither -S -p 12",
    );
    assert_graph_node_matches_render(
        "reverb",
        samples,
        "reverb",
        r#"wet_only = true
reverberance = "75"
hf_damping = "25"
room_scale = "50"
stereo_depth = "0"
pre_delay = "10"
wet_gain = "-3""#,
        "reverb -w 75 25 50 0 10 -3",
    );
}

#[test]
fn graph_nodes_lower_named_time_and_pitch_parameters() {
    let samples = &[-12000, -6000, 0, 6000, 12000, 6000, 0, -6000];

    assert_graph_node_matches_render(
        "tempo",
        samples,
        "tempo",
        r#"factor = "1.25"
quick = true
profile = "speech"
segment = "60"
search = "10"
overlap = "8""#,
        "tempo -q -s 1.25 60 10 8",
    );
    assert_graph_node_matches_render(
        "pitch",
        samples,
        "pitch",
        r#"cents = "-1200"
quick = true
segment = "60"
search = "10"
overlap = "8""#,
        "pitch -q -1200 60 10 8",
    );
    assert_graph_node_matches_render(
        "stretch",
        samples,
        "stretch",
        r#"factor = "1.5"
window = "10"
fade = "quarter"
shift = "0.75"
fading = "0.25""#,
        "stretch 1.5 10 q 0.75 0.25",
    );
}

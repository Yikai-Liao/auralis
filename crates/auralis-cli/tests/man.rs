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
    assert!(stdout.contains("norm"), "{stdout}");
    assert!(stdout.contains("rate"), "{stdout}");
    assert!(stdout.contains("channels"), "{stdout}");
    assert!(stdout.contains("gain"), "{stdout}");
    assert!(stdout.contains("reverse"), "{stdout}");
    assert!(stdout.contains("deemph"), "{stdout}");
    assert!(stdout.contains("earwax"), "{stdout}");
    assert!(stdout.contains("echo"), "{stdout}");
    assert!(stdout.contains("echos"), "{stdout}");
    assert!(stdout.contains("chorus"), "{stdout}");
    assert!(stdout.contains("flanger"), "{stdout}");
    assert!(stdout.contains("phaser"), "{stdout}");
    assert!(stdout.contains("oops"), "{stdout}");
    assert!(stdout.contains("riaa"), "{stdout}");
    assert!(stdout.contains("swap"), "{stdout}");
    assert!(stdout.contains("contrast"), "{stdout}");
    assert!(stdout.contains("overdrive"), "{stdout}");
    assert!(stdout.contains("saturation"), "{stdout}");
    assert!(stdout.contains("dcshift"), "{stdout}");
    assert!(stdout.contains("vol"), "{stdout}");
    assert!(stdout.contains("softvol"), "{stdout}");
    assert!(stdout.contains("tremolo"), "{stdout}");
    assert!(stdout.contains("speed"), "{stdout}");
    assert!(stdout.contains("tempo"), "{stdout}");
    assert!(stdout.contains("pitch"), "{stdout}");
    assert!(stdout.contains("bass"), "{stdout}");
    assert!(stdout.contains("treble"), "{stdout}");
    assert!(stdout.contains("equalizer"), "{stdout}");
    assert!(stdout.contains("allpass"), "{stdout}");
    assert!(stdout.contains("band"), "{stdout}");
    assert!(stdout.contains("bandpass"), "{stdout}");
    assert!(stdout.contains("bandreject"), "{stdout}");
    assert!(stdout.contains("highpass"), "{stdout}");
    assert!(stdout.contains("lowpass"), "{stdout}");
    assert!(stdout.contains("fade"), "{stdout}");
    assert!(stdout.contains("delay"), "{stdout}");
    assert!(stdout.contains("pad"), "{stdout}");
    assert!(stdout.contains("repeat"), "{stdout}");
    assert!(stdout.contains("downsample"), "{stdout}");
    assert!(stdout.contains("upsample"), "{stdout}");
    assert!(stdout.contains("hilbert"), "{stdout}");
    assert!(stdout.contains("loudness"), "{stdout}");
    assert!(stdout.contains("dither"), "{stdout}");
    assert!(stdout.contains("reverb"), "{stdout}");
    assert!(stdout.contains("stretch"), "{stdout}");
    assert!(stdout.contains("mix"), "{stdout}");
    assert!(stdout.contains("concat"), "{stdout}");
    assert!(stdout.contains("mix-power"), "{stdout}");
    assert!(stdout.contains("merge"), "{stdout}");
    assert!(stdout.contains("multiply"), "{stdout}");
    assert!(stdout.contains("render"), "{stdout}");
    assert!(stdout.contains("pipe"), "{stdout}");
    assert!(stdout.contains("plan"), "{stdout}");
    assert!(stdout.contains("init"), "{stdout}");
    assert!(stdout.contains("cache"), "{stdout}");
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
fn pipe_man_page_includes_core_options() {
    let output = Command::new(env!("CARGO_BIN_EXE_auralis"))
        .args(["man", "pipe"])
        .output()
        .unwrap();

    assert!(output.status.success(), "stderr: {}", stderr(&output));
    let stdout = stdout(&output);
    assert!(
        stdout.contains("pipe - run one compact DSP expression"),
        "{stdout}"
    );
    assert!(
        stdout.contains("auralis pipe INPUT.wav EXPR -o OUTPUT.wav"),
        "{stdout}"
    );
    assert!(stdout.contains("--backend BACKEND"), "{stdout}");
}

#[test]
fn init_man_page_includes_scaffold_contract() {
    let output = Command::new(env!("CARGO_BIN_EXE_auralis"))
        .args(["man", "init"])
        .output()
        .unwrap();

    assert!(output.status.success(), "stderr: {}", stderr(&output));
    let stdout = stdout(&output);
    assert!(
        stdout.contains("init - create a graph spec scaffold"),
        "{stdout}"
    );
    assert!(stdout.contains("auralis init [SPEC]"), "{stdout}");
    assert!(
        stdout.contains("will not overwrite an existing file"),
        "{stdout}"
    );
}

#[test]
fn cache_man_page_includes_status_contract() {
    let output = Command::new(env!("CARGO_BIN_EXE_auralis"))
        .args(["man", "cache"])
        .output()
        .unwrap();

    assert!(output.status.success(), "stderr: {}", stderr(&output));
    let stdout = stdout(&output);
    assert!(
        stdout.contains("cache - inspect local persistent cache state"),
        "{stdout}"
    );
    assert!(
        stdout.contains("auralis cache status [--root DIR] [--json]"),
        "{stdout}"
    );
    assert!(stdout.contains("--root DIR"), "{stdout}");
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
fn boundary_effect_man_pages_describe_recipe_lowering() {
    for (topic, summary, render_form, option) in [
        (
            "norm",
            "norm - normalize with the typed norm effect",
            "render --fx 'norm ...'",
            "DBFS",
        ),
        (
            "rate",
            "rate - resample with the typed rate effect",
            "render --fx 'rate ...'",
            "RATE",
        ),
        (
            "channels",
            "channels - convert to a target channel count",
            "render --fx 'channels ...'",
            "CHANNELS",
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
fn echo_man_pages_describe_recipe_lowering() {
    for (topic, summary, render_form) in [
        (
            "echo",
            "echo - add parallel delayed echoes",
            "render --fx 'echo ...'",
        ),
        (
            "echos",
            "echos - add cascaded delayed echoes",
            "render --fx 'echos ...'",
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
        assert!(stdout.contains("--tap DELAY_MS,DECAY"), "{stdout}");
        assert!(stdout.contains("-o, --output FILE"), "{stdout}");
    }
}

#[test]
fn modulation_man_pages_describe_recipe_lowering() {
    for (topic, summary, render_form, option) in [
        (
            "chorus",
            "chorus - add chorus modulation",
            "render --fx 'chorus ...'",
            "--stage STAGE",
        ),
        (
            "flanger",
            "flanger - add flanger modulation",
            "render --fx 'flanger ...'",
            "--phase PERCENT",
        ),
        (
            "phaser",
            "phaser - add phaser modulation",
            "render --fx 'phaser ...'",
            "--regen AMOUNT",
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
fn level_and_modulation_man_pages_describe_recipe_lowering() {
    for (topic, summary, render_form, option) in [
        (
            "dcshift",
            "dcshift - shift DC level",
            "render --fx 'dcshift ...'",
            "--limiter-gain GAIN",
        ),
        (
            "vol",
            "vol - apply SoX-ng volume scaling",
            "render --fx 'vol ...'",
            "--type TYPE",
        ),
        (
            "softvol",
            "softvol - apply soft volume changes",
            "render --fx 'softvol ...'",
            "--headroom DB",
        ),
        (
            "tremolo",
            "tremolo - apply tremolo modulation",
            "render --fx 'tremolo ...'",
            "--depth PERCENT",
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
fn time_and_pitch_man_pages_describe_recipe_lowering() {
    for (topic, summary, render_form, option) in [
        (
            "speed",
            "speed - change playback speed",
            "render --fx 'speed ...'",
            "FACTOR",
        ),
        (
            "tempo",
            "tempo - change tempo without changing pitch",
            "render --fx 'tempo ...'",
            "--profile PROFILE",
        ),
        (
            "pitch",
            "pitch - shift pitch without changing tempo",
            "render --fx 'pitch ...'",
            "--quick",
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
fn eq_man_pages_describe_recipe_lowering() {
    for (topic, summary, render_form, option) in [
        (
            "bass",
            "bass - boost or cut bass frequencies",
            "render --fx 'bass ...'",
            "--frequency HZ",
        ),
        (
            "treble",
            "treble - boost or cut treble frequencies",
            "render --fx 'treble ...'",
            "--width WIDTH",
        ),
        (
            "equalizer",
            "equalizer - apply one peaking equalizer band",
            "render --fx 'equalizer ...'",
            "--gain DB",
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
fn filter_man_pages_describe_recipe_lowering() {
    for (topic, summary, render_form, option) in [
        (
            "allpass",
            "allpass - apply an all-pass filter",
            "render --fx 'allpass ...'",
            "--poles 1|2",
        ),
        (
            "band",
            "band - apply a resonator band-pass filter",
            "render --fx 'band ...'",
            "--unpitched",
        ),
        (
            "bandpass",
            "bandpass - apply an RBJ band-pass filter",
            "render --fx 'bandpass ...'",
            "--constant-skirt",
        ),
        (
            "bandreject",
            "bandreject - apply an RBJ band-reject filter",
            "render --fx 'bandreject ...'",
            "--width WIDTH",
        ),
        (
            "highpass",
            "highpass - apply a high-pass filter",
            "render --fx 'highpass ...'",
            "--poles 1|2",
        ),
        (
            "lowpass",
            "lowpass - apply a low-pass filter",
            "render --fx 'lowpass ...'",
            "--frequency HZ",
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
fn structural_man_pages_describe_recipe_lowering() {
    for (topic, summary, render_form, option) in [
        (
            "delay",
            "delay - delay audio channels",
            "render --fx 'delay ...'",
            "--position POSITION",
        ),
        (
            "pad",
            "pad - add silence padding",
            "render --fx 'pad ...'",
            "--at FRAMES@POSITION",
        ),
        (
            "repeat",
            "repeat - append finite copies",
            "render --fx 'repeat ...'",
            "COUNT",
        ),
        (
            "downsample",
            "downsample - keep every Nth sample",
            "render --fx 'downsample ...'",
            "FACTOR",
        ),
        (
            "upsample",
            "upsample - insert zero samples",
            "render --fx 'upsample ...'",
            "FACTOR",
        ),
        (
            "hilbert",
            "hilbert - apply Hilbert transform",
            "render --fx 'hilbert ...'",
            "--taps TAPS",
        ),
        (
            "loudness",
            "loudness - apply loudness compensation",
            "render --fx 'loudness ...'",
            "--half-points N",
        ),
        (
            "dither",
            "dither - apply deterministic dithering",
            "render --fx 'dither ...'",
            "--precision BITS",
        ),
        (
            "reverb",
            "reverb - apply stereo reverberation",
            "render --fx 'reverb ...'",
            "--wet-gain DB",
        ),
        (
            "stretch",
            "stretch - change duration with windowed stretching",
            "render --fx 'stretch ...'",
            "--fade SHAPE",
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

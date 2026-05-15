//! Integration tests for the `auralis completions` command.

mod support;

use support::*;

#[test]
fn bash_completions_include_modern_commands_and_flags() {
    let output = Command::new(env!("CARGO_BIN_EXE_auralis"))
        .args(["completions", "bash"])
        .output()
        .unwrap();

    assert!(output.status.success(), "stderr: {}", stderr(&output));
    let stdout = stdout(&output);
    assert!(stdout.contains("_auralis_completions()"), "{stdout}");
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
    assert!(stdout.contains("--locked"), "{stdout}");
    assert!(stdout.contains("--cache"), "{stdout}");
    assert!(stdout.contains("--target"), "{stdout}");
    assert!(stdout.contains("--root"), "{stdout}");
    assert!(stdout.contains("--yes"), "{stdout}");
    assert!(stdout.contains("--schema"), "{stdout}");
}

#[test]
fn zsh_completions_include_modern_commands_and_flags() {
    let output = Command::new(env!("CARGO_BIN_EXE_auralis"))
        .args(["completions", "zsh"])
        .output()
        .unwrap();

    assert!(output.status.success(), "stderr: {}", stderr(&output));
    let stdout = stdout(&output);
    assert!(stdout.contains("#compdef auralis"), "{stdout}");
    assert!(stdout.contains("'normalize:normalize'"), "{stdout}");
    assert!(stdout.contains("'norm:norm'"), "{stdout}");
    assert!(stdout.contains("'rate:rate'"), "{stdout}");
    assert!(stdout.contains("'channels:channels'"), "{stdout}");
    assert!(stdout.contains("'gain:gain'"), "{stdout}");
    assert!(stdout.contains("'reverse:reverse'"), "{stdout}");
    assert!(stdout.contains("'deemph:deemph'"), "{stdout}");
    assert!(stdout.contains("'earwax:earwax'"), "{stdout}");
    assert!(stdout.contains("'echo:echo'"), "{stdout}");
    assert!(stdout.contains("'echos:echos'"), "{stdout}");
    assert!(stdout.contains("'chorus:chorus'"), "{stdout}");
    assert!(stdout.contains("'flanger:flanger'"), "{stdout}");
    assert!(stdout.contains("'phaser:phaser'"), "{stdout}");
    assert!(stdout.contains("'oops:oops'"), "{stdout}");
    assert!(stdout.contains("'riaa:riaa'"), "{stdout}");
    assert!(stdout.contains("'swap:swap'"), "{stdout}");
    assert!(stdout.contains("'contrast:contrast'"), "{stdout}");
    assert!(stdout.contains("'overdrive:overdrive'"), "{stdout}");
    assert!(stdout.contains("'saturation:saturation'"), "{stdout}");
    assert!(stdout.contains("'dcshift:dcshift'"), "{stdout}");
    assert!(stdout.contains("'vol:vol'"), "{stdout}");
    assert!(stdout.contains("'softvol:softvol'"), "{stdout}");
    assert!(stdout.contains("'tremolo:tremolo'"), "{stdout}");
    assert!(stdout.contains("'speed:speed'"), "{stdout}");
    assert!(stdout.contains("'tempo:tempo'"), "{stdout}");
    assert!(stdout.contains("'pitch:pitch'"), "{stdout}");
    assert!(stdout.contains("'bass:bass'"), "{stdout}");
    assert!(stdout.contains("'treble:treble'"), "{stdout}");
    assert!(stdout.contains("'equalizer:equalizer'"), "{stdout}");
    assert!(stdout.contains("'allpass:allpass'"), "{stdout}");
    assert!(stdout.contains("'band:band'"), "{stdout}");
    assert!(stdout.contains("'bandpass:bandpass'"), "{stdout}");
    assert!(stdout.contains("'bandreject:bandreject'"), "{stdout}");
    assert!(stdout.contains("'highpass:highpass'"), "{stdout}");
    assert!(stdout.contains("'lowpass:lowpass'"), "{stdout}");
    assert!(stdout.contains("'fade:fade'"), "{stdout}");
    assert!(stdout.contains("'delay:delay'"), "{stdout}");
    assert!(stdout.contains("'pad:pad'"), "{stdout}");
    assert!(stdout.contains("'repeat:repeat'"), "{stdout}");
    assert!(stdout.contains("'downsample:downsample'"), "{stdout}");
    assert!(stdout.contains("'upsample:upsample'"), "{stdout}");
    assert!(stdout.contains("'hilbert:hilbert'"), "{stdout}");
    assert!(stdout.contains("'loudness:loudness'"), "{stdout}");
    assert!(stdout.contains("'dither:dither'"), "{stdout}");
    assert!(stdout.contains("'reverb:reverb'"), "{stdout}");
    assert!(stdout.contains("'stretch:stretch'"), "{stdout}");
    assert!(stdout.contains("'mix:mix'"), "{stdout}");
    assert!(stdout.contains("'concat:concat'"), "{stdout}");
    assert!(stdout.contains("'mix-power:mix-power'"), "{stdout}");
    assert!(stdout.contains("'merge:merge'"), "{stdout}");
    assert!(stdout.contains("'multiply:multiply'"), "{stdout}");
    assert!(stdout.contains("'pipe:pipe'"), "{stdout}");
    assert!(stdout.contains("'init:init'"), "{stdout}");
    assert!(stdout.contains("'fmt:fmt'"), "{stdout}");
    assert!(stdout.contains("'--check[fmt option]'"), "{stdout}");
    assert!(stdout.contains("'--peak[normalize option]'"), "{stdout}");
    assert!(stdout.contains("'--curve[fade option]'"), "{stdout}");
    assert!(stdout.contains("'--locked[plan option]'"), "{stdout}");
    assert!(stdout.contains("'--target[plan option]'"), "{stdout}");
    assert!(stdout.contains("'--cache[run option]'"), "{stdout}");
    assert!(stdout.contains("'--output[graph option]'"), "{stdout}");
}

#[test]
fn fish_completions_include_modern_commands_and_flags() {
    let output = Command::new(env!("CARGO_BIN_EXE_auralis"))
        .args(["completions", "fish"])
        .output()
        .unwrap();

    assert!(output.status.success(), "stderr: {}", stderr(&output));
    let stdout = stdout(&output);
    for command in [
        "ops",
        "trim",
        "norm",
        "rate",
        "channels",
        "gain",
        "reverse",
        "deemph",
        "earwax",
        "echo",
        "echos",
        "chorus",
        "flanger",
        "phaser",
        "oops",
        "riaa",
        "swap",
        "contrast",
        "overdrive",
        "saturation",
        "dcshift",
        "vol",
        "softvol",
        "tremolo",
        "speed",
        "tempo",
        "pitch",
        "bass",
        "treble",
        "equalizer",
        "allpass",
        "band",
        "bandpass",
        "bandreject",
        "highpass",
        "lowpass",
        "fade",
        "delay",
        "pad",
        "repeat",
        "downsample",
        "upsample",
        "hilbert",
        "loudness",
        "dither",
        "reverb",
        "stretch",
        "mix",
        "concat",
        "mix-power",
        "merge",
        "multiply",
        "pipe",
        "init",
    ] {
        assert_fish_subcommand(&stdout, command);
    }
    for (command, option) in [
        ("fade", "out"),
        ("echo", "tap"),
        ("echos", "gain-in"),
        ("chorus", "stage"),
        ("flanger", "interpolation"),
        ("phaser", "regen"),
        ("saturation", "parameter"),
        ("softvol", "headroom"),
        ("tremolo", "depth"),
        ("tempo", "profile"),
        ("pitch", "quick"),
        ("bass", "frequency"),
        ("treble", "width"),
        ("equalizer", "gain"),
        ("allpass", "poles"),
        ("band", "unpitched"),
        ("bandpass", "constant-skirt"),
        ("highpass", "width"),
        ("lowpass", "poles"),
        ("check", "locked"),
        ("delay", "position"),
        ("pad", "at"),
        ("hilbert", "taps"),
        ("loudness", "half-points"),
        ("dither", "noise-shape"),
        ("reverb", "wet-only"),
        ("stretch", "fade"),
        ("render", "chain"),
        ("pipe", "output"),
        ("run", "cache"),
        ("cache", "yes"),
    ] {
        assert_fish_option(&stdout, command, option);
    }
}

fn assert_fish_subcommand(stdout: &str, command: &str) {
    assert!(
        stdout.contains(&format!(
            "complete -c auralis -n '__fish_use_subcommand' -a '{command}'"
        )),
        "{stdout}"
    );
}

fn assert_fish_option(stdout: &str, command: &str, option: &str) {
    assert!(
        stdout.contains(&format!(
            "complete -c auralis -n '__fish_seen_subcommand_from {command}' -l {option}"
        )),
        "{stdout}"
    );
}

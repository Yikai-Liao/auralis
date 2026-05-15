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
    assert!(stdout.contains("dcshift"), "{stdout}");
    assert!(stdout.contains("vol"), "{stdout}");
    assert!(stdout.contains("softvol"), "{stdout}");
    assert!(stdout.contains("tremolo"), "{stdout}");
    assert!(stdout.contains("bass"), "{stdout}");
    assert!(stdout.contains("treble"), "{stdout}");
    assert!(stdout.contains("equalizer"), "{stdout}");
    assert!(stdout.contains("fade"), "{stdout}");
    assert!(stdout.contains("mix"), "{stdout}");
    assert!(stdout.contains("concat"), "{stdout}");
    assert!(stdout.contains("mix-power"), "{stdout}");
    assert!(stdout.contains("merge"), "{stdout}");
    assert!(stdout.contains("multiply"), "{stdout}");
    assert!(stdout.contains("render"), "{stdout}");
    assert!(stdout.contains("plan"), "{stdout}");
    assert!(stdout.contains("completions"), "{stdout}");
    assert!(stdout.contains("--locked"), "{stdout}");
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
    assert!(stdout.contains("'gain:gain'"), "{stdout}");
    assert!(stdout.contains("'reverse:reverse'"), "{stdout}");
    assert!(stdout.contains("'deemph:deemph'"), "{stdout}");
    assert!(stdout.contains("'earwax:earwax'"), "{stdout}");
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
    assert!(stdout.contains("'bass:bass'"), "{stdout}");
    assert!(stdout.contains("'treble:treble'"), "{stdout}");
    assert!(stdout.contains("'equalizer:equalizer'"), "{stdout}");
    assert!(stdout.contains("'fade:fade'"), "{stdout}");
    assert!(stdout.contains("'mix:mix'"), "{stdout}");
    assert!(stdout.contains("'concat:concat'"), "{stdout}");
    assert!(stdout.contains("'mix-power:mix-power'"), "{stdout}");
    assert!(stdout.contains("'merge:merge'"), "{stdout}");
    assert!(stdout.contains("'multiply:multiply'"), "{stdout}");
    assert!(stdout.contains("'fmt:fmt'"), "{stdout}");
    assert!(stdout.contains("'--check[fmt option]'"), "{stdout}");
    assert!(stdout.contains("'--peak[normalize option]'"), "{stdout}");
    assert!(stdout.contains("'--curve[fade option]'"), "{stdout}");
    assert!(stdout.contains("'--locked[plan option]'"), "{stdout}");
}

#[test]
fn fish_completions_include_modern_commands_and_flags() {
    let output = Command::new(env!("CARGO_BIN_EXE_auralis"))
        .args(["completions", "fish"])
        .output()
        .unwrap();

    assert!(output.status.success(), "stderr: {}", stderr(&output));
    let stdout = stdout(&output);
    assert!(
        stdout.contains("complete -c auralis -n '__fish_use_subcommand' -a 'ops'"),
        "{stdout}"
    );
    assert!(
        stdout.contains("complete -c auralis -n '__fish_use_subcommand' -a 'trim'"),
        "{stdout}"
    );
    for command in [
        "gain",
        "reverse",
        "deemph",
        "earwax",
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
        "bass",
        "treble",
        "equalizer",
        "fade",
        "mix",
        "concat",
        "mix-power",
        "merge",
        "multiply",
    ] {
        assert!(
            stdout.contains(&format!(
                "complete -c auralis -n '__fish_use_subcommand' -a '{command}'"
            )),
            "{stdout}"
        );
    }
    assert!(
        stdout.contains("complete -c auralis -n '__fish_seen_subcommand_from fade' -l out"),
        "{stdout}"
    );
    assert!(
        stdout.contains(
            "complete -c auralis -n '__fish_seen_subcommand_from saturation' -l parameter"
        ),
        "{stdout}"
    );
    assert!(
        stdout.contains("complete -c auralis -n '__fish_seen_subcommand_from softvol' -l headroom"),
        "{stdout}"
    );
    assert!(
        stdout.contains("complete -c auralis -n '__fish_seen_subcommand_from tremolo' -l depth"),
        "{stdout}"
    );
    assert!(
        stdout.contains("complete -c auralis -n '__fish_seen_subcommand_from bass' -l frequency"),
        "{stdout}"
    );
    assert!(
        stdout.contains("complete -c auralis -n '__fish_seen_subcommand_from treble' -l width"),
        "{stdout}"
    );
    assert!(
        stdout.contains("complete -c auralis -n '__fish_seen_subcommand_from equalizer' -l gain"),
        "{stdout}"
    );
    assert!(
        stdout.contains("complete -c auralis -n '__fish_seen_subcommand_from check' -l locked"),
        "{stdout}"
    );
    assert!(
        stdout.contains("complete -c auralis -n '__fish_seen_subcommand_from render' -l chain"),
        "{stdout}"
    );
}

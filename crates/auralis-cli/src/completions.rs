//! Shell completion generation.

use clap::ValueEnum;

#[derive(Debug, Clone, Copy, ValueEnum)]
pub(crate) enum CompletionShell {
    Bash,
    Zsh,
    Fish,
}

struct CompletionSpec {
    name: &'static str,
    options: &'static [&'static str],
}

const COMPLETION_SPECS: &[CompletionSpec] = &[
    CompletionSpec {
        name: "inspect",
        options: &["--json"],
    },
    CompletionSpec {
        name: "convert",
        options: &[
            "-o",
            "--output",
            "--backend",
            "-c",
            "--channels",
            "--no-auto-channels",
            "-r",
            "--rate",
            "--no-auto-rate",
            "-G",
            "--guard",
            "--norm",
            "--sample",
        ],
    },
    CompletionSpec {
        name: "trim",
        options: &["-o", "--output", "--backend"],
    },
    CompletionSpec {
        name: "normalize",
        options: &["-o", "--output", "--peak", "--backend"],
    },
    CompletionSpec {
        name: "norm",
        options: &["-o", "--output", "--backend"],
    },
    CompletionSpec {
        name: "rate",
        options: &["-o", "--output", "--backend"],
    },
    CompletionSpec {
        name: "channels",
        options: &["-o", "--output", "--backend"],
    },
    CompletionSpec {
        name: "gain",
        options: &["-o", "--output", "--backend"],
    },
    CompletionSpec {
        name: "reverse",
        options: &["-o", "--output", "--backend"],
    },
    CompletionSpec {
        name: "deemph",
        options: &["-o", "--output", "--backend"],
    },
    CompletionSpec {
        name: "earwax",
        options: &["-o", "--output", "--backend"],
    },
    CompletionSpec {
        name: "echo",
        options: &[
            "-o",
            "--output",
            "--gain-in",
            "--gain-out",
            "--tap",
            "--backend",
        ],
    },
    CompletionSpec {
        name: "echos",
        options: &[
            "-o",
            "--output",
            "--gain-in",
            "--gain-out",
            "--tap",
            "--backend",
        ],
    },
    CompletionSpec {
        name: "chorus",
        options: &[
            "-o",
            "--output",
            "--gain-in",
            "--gain-out",
            "--interpolation",
            "--wave",
            "--stage",
            "--backend",
        ],
    },
    CompletionSpec {
        name: "flanger",
        options: &[
            "-o",
            "--output",
            "--delay",
            "--depth",
            "--regen",
            "--width",
            "--speed",
            "--wave",
            "--phase",
            "--interpolation",
            "--backend",
        ],
    },
    CompletionSpec {
        name: "phaser",
        options: &[
            "-o",
            "--output",
            "--gain-in",
            "--gain-out",
            "--delay",
            "--regen",
            "--speed",
            "--wave",
            "--interpolation",
            "--backend",
        ],
    },
    CompletionSpec {
        name: "oops",
        options: &["-o", "--output", "--backend"],
    },
    CompletionSpec {
        name: "riaa",
        options: &["-o", "--output", "--backend"],
    },
    CompletionSpec {
        name: "swap",
        options: &["-o", "--output", "--backend"],
    },
    CompletionSpec {
        name: "contrast",
        options: &["-o", "--output", "--amount", "--backend"],
    },
    CompletionSpec {
        name: "overdrive",
        options: &["-o", "--output", "--gain", "--color", "--backend"],
    },
    CompletionSpec {
        name: "saturation",
        options: &[
            "-o",
            "--output",
            "--type",
            "--blend",
            "--offset",
            "--parameter",
            "--backend",
        ],
    },
    CompletionSpec {
        name: "dcshift",
        options: &["-o", "--output", "--limiter-gain", "--backend"],
    },
    CompletionSpec {
        name: "vol",
        options: &["-o", "--output", "--type", "--limiter-gain", "--backend"],
    },
    CompletionSpec {
        name: "softvol",
        options: &[
            "-o",
            "--output",
            "--volume",
            "--double-time",
            "--headroom",
            "--backend",
        ],
    },
    CompletionSpec {
        name: "tremolo",
        options: &["-o", "--output", "--depth", "--backend"],
    },
    CompletionSpec {
        name: "speed",
        options: &["-o", "--output", "--backend"],
    },
    CompletionSpec {
        name: "tempo",
        options: &[
            "-o",
            "--output",
            "--quick",
            "--profile",
            "--segment",
            "--search",
            "--overlap",
            "--backend",
        ],
    },
    CompletionSpec {
        name: "pitch",
        options: &[
            "-o",
            "--output",
            "--quick",
            "--segment",
            "--search",
            "--overlap",
            "--backend",
        ],
    },
    CompletionSpec {
        name: "bass",
        options: &["-o", "--output", "--frequency", "--width", "--backend"],
    },
    CompletionSpec {
        name: "treble",
        options: &["-o", "--output", "--frequency", "--width", "--backend"],
    },
    CompletionSpec {
        name: "equalizer",
        options: &[
            "-o",
            "--output",
            "--frequency",
            "--width",
            "--gain",
            "--backend",
        ],
    },
    CompletionSpec {
        name: "allpass",
        options: &[
            "-o",
            "--output",
            "--frequency",
            "--width",
            "--poles",
            "--backend",
        ],
    },
    CompletionSpec {
        name: "band",
        options: &[
            "-o",
            "--output",
            "--frequency",
            "--width",
            "--unpitched",
            "--backend",
        ],
    },
    CompletionSpec {
        name: "bandpass",
        options: &[
            "-o",
            "--output",
            "--frequency",
            "--width",
            "--constant-skirt",
            "--backend",
        ],
    },
    CompletionSpec {
        name: "bandreject",
        options: &["-o", "--output", "--frequency", "--width", "--backend"],
    },
    CompletionSpec {
        name: "highpass",
        options: &[
            "-o",
            "--output",
            "--frequency",
            "--width",
            "--poles",
            "--backend",
        ],
    },
    CompletionSpec {
        name: "lowpass",
        options: &[
            "-o",
            "--output",
            "--frequency",
            "--width",
            "--poles",
            "--backend",
        ],
    },
    CompletionSpec {
        name: "fade",
        options: &["-o", "--output", "--in", "--out", "--curve", "--backend"],
    },
    CompletionSpec {
        name: "delay",
        options: &["-o", "--output", "--position", "--backend"],
    },
    CompletionSpec {
        name: "pad",
        options: &["-o", "--output", "--start", "--end", "--at", "--backend"],
    },
    CompletionSpec {
        name: "repeat",
        options: &["-o", "--output", "--backend"],
    },
    CompletionSpec {
        name: "downsample",
        options: &["-o", "--output", "--backend"],
    },
    CompletionSpec {
        name: "upsample",
        options: &["-o", "--output", "--backend"],
    },
    CompletionSpec {
        name: "hilbert",
        options: &["-o", "--output", "--taps", "--backend"],
    },
    CompletionSpec {
        name: "loudness",
        options: &[
            "-o",
            "--output",
            "--gain",
            "--reference",
            "--half-points",
            "--backend",
        ],
    },
    CompletionSpec {
        name: "dither",
        options: &[
            "-o",
            "--output",
            "--sloped",
            "--noise-shape",
            "--precision",
            "--backend",
        ],
    },
    CompletionSpec {
        name: "reverb",
        options: &[
            "-o",
            "--output",
            "--wet-only",
            "--reverberance",
            "--hf-damping",
            "--room-scale",
            "--stereo-depth",
            "--pre-delay",
            "--wet-gain",
            "--backend",
        ],
    },
    CompletionSpec {
        name: "stretch",
        options: &[
            "-o",
            "--output",
            "--window",
            "--fade",
            "--shift",
            "--fading",
            "--backend",
        ],
    },
    CompletionSpec {
        name: "mix",
        options: &["-o", "--output", "--backend"],
    },
    CompletionSpec {
        name: "concat",
        options: &["-o", "--output", "--backend"],
    },
    CompletionSpec {
        name: "mix-power",
        options: &["-o", "--output", "--backend"],
    },
    CompletionSpec {
        name: "merge",
        options: &["-o", "--output", "--backend"],
    },
    CompletionSpec {
        name: "multiply",
        options: &["-o", "--output", "--backend"],
    },
    CompletionSpec {
        name: "render",
        options: &[
            "-o",
            "--output",
            "--backend",
            "--combine",
            "--input",
            "-c",
            "--channels",
            "--no-auto-channels",
            "-r",
            "--rate",
            "--no-auto-rate",
            "-G",
            "--guard",
            "--norm",
            "--dither",
            "--dither-seed",
            "--effects-file",
            "--fx",
            "--chain",
        ],
    },
    CompletionSpec {
        name: "pipe",
        options: &["-o", "--output", "--backend"],
    },
    CompletionSpec {
        name: "check",
        options: &["--locked", "--effects-file", "--fx", "--chain"],
    },
    CompletionSpec {
        name: "plan",
        options: &["--json", "--locked"],
    },
    CompletionSpec {
        name: "graph",
        options: &["-o", "--output", "--format"],
    },
    CompletionSpec {
        name: "fmt",
        options: &["--check"],
    },
    CompletionSpec {
        name: "init",
        options: &[],
    },
    CompletionSpec {
        name: "completions",
        options: &[],
    },
    CompletionSpec {
        name: "man",
        options: &[],
    },
    CompletionSpec {
        name: "explain",
        options: &[],
    },
    CompletionSpec {
        name: "run",
        options: &["--locked"],
    },
    CompletionSpec {
        name: "ops",
        options: &["--schema"],
    },
];

pub(crate) fn print_completions(shell: CompletionShell) {
    match shell {
        CompletionShell::Bash => print_bash_completions(),
        CompletionShell::Zsh => print_zsh_completions(),
        CompletionShell::Fish => print_fish_completions(),
    }
}

fn print_bash_completions() {
    let commands = COMPLETION_SPECS
        .iter()
        .map(|spec| spec.name)
        .collect::<Vec<_>>()
        .join(" ");
    println!("_auralis_completions() {{");
    println!("  local cur prev words cword");
    println!("  _init_completion || return");
    println!();
    println!("  case \"$prev\" in");
    println!("    auralis)");
    println!("      COMPREPLY=( $(compgen -W \"{commands}\" -- \"$cur\") )");
    println!("      return");
    println!("      ;;");
    for spec in COMPLETION_SPECS {
        if spec.options.is_empty() {
            continue;
        }
        println!("    {})", spec.name);
        println!(
            "      COMPREPLY=( $(compgen -W \"{}\" -- \"$cur\") )",
            spec.options.join(" ")
        );
        println!("      return");
        println!("      ;;");
    }
    println!("  esac");
    println!();
    println!("  if [[ $cword -eq 1 ]]; then");
    println!("    COMPREPLY=( $(compgen -W \"{commands}\" -- \"$cur\") )");
    println!("    return");
    println!("  fi");
    println!("}}");
    println!("complete -F _auralis_completions auralis");
}

fn print_zsh_completions() {
    println!("#compdef auralis");
    println!("local -a commands");
    println!("commands=(");
    for spec in COMPLETION_SPECS {
        println!("  '{}:{}'", spec.name, spec.name);
    }
    println!(")");
    println!("if (( CURRENT == 2 )); then");
    println!("  _describe 'command' commands");
    println!("  return");
    println!("fi");
    println!("case $words[2] in");
    for spec in COMPLETION_SPECS {
        println!("  {})", spec.name);
        if spec.options.is_empty() {
            println!("    _message 'no additional option completions'");
        } else {
            println!("    _values 'option' \\");
            for option in spec.options {
                println!("      '{}[{} option]' \\", option, spec.name);
            }
            println!("      ;");
        }
        println!("    ;;");
    }
    println!("esac");
}

fn print_fish_completions() {
    for spec in COMPLETION_SPECS {
        println!(
            "complete -c auralis -n '__fish_use_subcommand' -a '{}' -d '{}'",
            spec.name, spec.name
        );
    }
    for spec in COMPLETION_SPECS {
        for option in spec.options {
            let (flag_kind, flag_name) = if let Some(long) = option.strip_prefix("--") {
                ("-l", long)
            } else if let Some(short) = option.strip_prefix('-') {
                ("-s", short)
            } else {
                continue;
            };
            println!(
                "complete -c auralis -n '__fish_seen_subcommand_from {}' {} {}",
                spec.name, flag_kind, flag_name
            );
        }
    }
}

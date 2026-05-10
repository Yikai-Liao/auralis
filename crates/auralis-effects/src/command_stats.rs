use crate::{
    Stats, StatsDisplayScale,
    command::{CommandResult, EffectCommand, EffectCommandParseError, parse_f64, render_f64},
};

pub(super) fn parse_stats(effect: &'static str, args: &[&str]) -> CommandResult<EffectCommand> {
    let mut stats = Stats::new();
    let mut index = 0;

    while index < args.len() {
        match args[index] {
            "-b" => {
                let bits = parse_bits_option(effect, args.get(index + 1).copied(), "bits")?;
                stats = stats.with_signed_bits(bits).map_err(|source| {
                    EffectCommandParseError::InvalidEffectConfig {
                        effect,
                        argument: "bits",
                        source,
                    }
                })?;
                index += 2;
            }
            "-x" => {
                let bits = parse_bits_option(effect, args.get(index + 1).copied(), "bits")?;
                stats = stats.with_hex_bits(bits).map_err(|source| {
                    EffectCommandParseError::InvalidEffectConfig {
                        effect,
                        argument: "bits",
                        source,
                    }
                })?;
                index += 2;
            }
            "-s" => {
                let Some(scale) = args.get(index + 1) else {
                    return Err(EffectCommandParseError::MissingArgument {
                        effect,
                        argument: "scale",
                    });
                };
                stats = stats
                    .with_scale(parse_f64(effect, "scale", scale)?)
                    .map_err(|source| EffectCommandParseError::InvalidEffectConfig {
                        effect,
                        argument: "scale",
                        source,
                    })?;
                index += 2;
            }
            "-w" => {
                let Some(window) = args.get(index + 1) else {
                    return Err(EffectCommandParseError::MissingArgument {
                        effect,
                        argument: "window-time",
                    });
                };
                stats = stats
                    .with_window_time(parse_f64(effect, "window-time", window)?)
                    .map_err(|source| EffectCommandParseError::InvalidEffectConfig {
                        effect,
                        argument: "window-time",
                        source,
                    })?;
                index += 2;
            }
            "-j" => {
                stats = stats.json();
                index += 1;
            }
            option if crate::command::is_option_like(option) => {
                return Err(EffectCommandParseError::UnsupportedOption {
                    effect,
                    option: option.to_owned(),
                });
            }
            argument => {
                return Err(EffectCommandParseError::UnexpectedArgument {
                    effect,
                    argument: argument.to_owned(),
                });
            }
        }
    }

    Ok(EffectCommand::Stats(stats))
}

pub(super) fn render_stats(stats: Stats) -> Vec<String> {
    let mut tokens = vec!["stats".to_owned()];
    match stats.display_scale() {
        StatsDisplayScale::Float(scale) if scale.to_bits() != 1.0_f64.to_bits() => {
            tokens.push("-s".to_owned());
            tokens.push(render_f64(scale));
        }
        StatsDisplayScale::Float(_) => {}
        StatsDisplayScale::SignedBits(bits) => {
            tokens.push("-b".to_owned());
            tokens.push(bits.to_string());
        }
        StatsDisplayScale::HexBits(bits) => {
            tokens.push("-x".to_owned());
            tokens.push(bits.to_string());
        }
    }
    if stats.window_time_seconds().to_bits() != 0.05_f64.to_bits() {
        tokens.push("-w".to_owned());
        tokens.push(render_f64(stats.window_time_seconds()));
    }
    if stats.is_json() {
        tokens.push("-j".to_owned());
    }
    tokens
}

fn parse_bits_option(
    effect: &'static str,
    value: Option<&str>,
    argument: &'static str,
) -> CommandResult<u8> {
    let Some(value) = value else {
        return Err(EffectCommandParseError::MissingArgument { effect, argument });
    };
    let bits = parse_f64(effect, argument, value)?;
    if bits.fract() != 0.0 || !(2.0..=32.0).contains(&bits) {
        return Err(EffectCommandParseError::InvalidEffectConfig {
            effect,
            argument,
            source: crate::EffectError::InvalidStats,
        });
    }

    #[allow(
        clippy::cast_possible_truncation,
        clippy::cast_sign_loss,
        reason = "bits is validated to the u8-compatible SoX-ng range before casting"
    )]
    Ok(bits as u8)
}

#[cfg(test)]
mod tests {
    use super::parse_stats;
    use crate::{EffectCommand, EffectCommandParseError, EffectError, Stats};

    #[test]
    fn parses_default_and_supported_options() {
        assert_eq!(
            parse_stats("stats", &[]).unwrap(),
            EffectCommand::Stats(Stats::new())
        );
        assert_eq!(
            parse_stats("stats", &["-x", "16", "-w", "0.1", "-j"])
                .unwrap()
                .render_tokens(),
            ["stats", "-x", "16", "-w", "0.1", "-j"]
        );
        assert_eq!(
            parse_stats("stats", &["-b", "12"]).unwrap().render_tokens(),
            ["stats", "-b", "12"]
        );
    }

    #[test]
    fn rejects_unsupported_options_extra_arguments_and_bad_ranges() {
        assert_eq!(
            parse_stats("stats", &["-rms"]).unwrap_err(),
            EffectCommandParseError::UnsupportedOption {
                effect: "stats",
                option: "-rms".to_owned(),
            }
        );
        assert_eq!(
            parse_stats("stats", &["extra"]).unwrap_err(),
            EffectCommandParseError::UnexpectedArgument {
                effect: "stats",
                argument: "extra".to_owned(),
            }
        );
        assert!(matches!(
            parse_stats("stats", &["-b", "1"]).unwrap_err(),
            EffectCommandParseError::InvalidEffectConfig {
                source: EffectError::InvalidStats,
                ..
            }
        ));
    }
}

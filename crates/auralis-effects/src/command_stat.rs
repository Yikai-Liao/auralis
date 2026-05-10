use crate::{
    Stat,
    command::{CommandResult, EffectCommand, EffectCommandParseError, parse_f64},
};

pub(super) fn parse_stat(effect: &'static str, args: &[&str]) -> CommandResult<EffectCommand> {
    let mut stat = Stat::new();
    let mut index = 0;

    while index < args.len() {
        match args[index] {
            "-s" => {
                let Some(scale) = args.get(index + 1) else {
                    return Err(EffectCommandParseError::MissingArgument {
                        effect,
                        argument: "scale",
                    });
                };
                stat = stat
                    .with_scale(parse_f64(effect, "scale", scale)?)
                    .map_err(|source| EffectCommandParseError::InvalidEffectConfig {
                        effect,
                        argument: "scale",
                        source,
                    })?;
                index += 2;
            }
            "-rms" => {
                stat = stat.with_rms_scaling();
                index += 1;
            }
            "-v" => {
                stat = stat.volume_only();
                index += 1;
            }
            "-j" => {
                stat = stat.json();
                index += 1;
            }
            "-freq" | "-d" | "-a" | "-e" | "-h" => {
                return Err(EffectCommandParseError::UnsupportedOption {
                    effect,
                    option: args[index].to_owned(),
                });
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

    Ok(EffectCommand::Stat(stat))
}

pub(super) fn render_stat(stat: Stat) -> Vec<String> {
    let mut tokens = vec!["stat".to_owned()];
    if stat.scale().to_bits() != 1.0_f64.to_bits() {
        tokens.push("-s".to_owned());
        tokens.push(stat.scale().to_string());
    }
    if stat.scale_to_rms() {
        tokens.push("-rms".to_owned());
    }
    if stat.is_volume_only() {
        tokens.push("-v".to_owned());
    }
    if stat.is_json() {
        tokens.push("-j".to_owned());
    }
    tokens
}

#[cfg(test)]
mod tests {
    use super::parse_stat;
    use crate::{EffectCommand, EffectCommandParseError, EffectError, Stat};

    #[test]
    fn parses_default_and_supported_options() {
        assert_eq!(
            parse_stat("stat", &[]).unwrap(),
            EffectCommand::Stat(Stat::new())
        );
        assert_eq!(
            parse_stat("stat", &["-s", "2", "-rms", "-v", "-j"])
                .unwrap()
                .render_tokens(),
            ["stat", "-s", "2", "-rms", "-v", "-j"]
        );
    }

    #[test]
    fn rejects_unsupported_options_extra_arguments_and_bad_scale() {
        assert_eq!(
            parse_stat("stat", &["-freq"]).unwrap_err(),
            EffectCommandParseError::UnsupportedOption {
                effect: "stat",
                option: "-freq".to_owned(),
            }
        );
        assert_eq!(
            parse_stat("stat", &["extra"]).unwrap_err(),
            EffectCommandParseError::UnexpectedArgument {
                effect: "stat",
                argument: "extra".to_owned(),
            }
        );
        assert!(matches!(
            parse_stat("stat", &["-s", "0"]).unwrap_err(),
            EffectCommandParseError::InvalidEffectConfig {
                source: EffectError::InvalidStat,
                ..
            }
        ));
    }
}

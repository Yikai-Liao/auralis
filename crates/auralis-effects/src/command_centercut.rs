use crate::Centercut;
use crate::command::{
    CommandResult, EffectCommand, EffectCommandParseError, is_option_like, parse_f32, render_f32,
};

pub(super) fn parse_centercut(effect: &'static str, args: &[&str]) -> CommandResult<EffectCommand> {
    let mut gain = 1.0;
    let mut bass_to_sides = false;
    let mut window_size = 8192_usize;
    let mut index = 0_usize;

    while let Some(arg) = args.get(index).copied() {
        match arg {
            "-b" => {
                bass_to_sides = true;
                index += 1;
            }
            "-a" | "-w" => {
                let value = args.get(index + 1).copied().ok_or(
                    EffectCommandParseError::MissingArgument {
                        effect,
                        argument: option_argument(arg),
                    },
                )?;
                apply_option(effect, arg, value, &mut gain, &mut window_size)?;
                index += 2;
            }
            _ if arg.starts_with("-a") && arg.len() > 2 => {
                apply_option(effect, "-a", &arg[2..], &mut gain, &mut window_size)?;
                index += 1;
            }
            _ if arg.starts_with("-w") && arg.len() > 2 => {
                apply_option(effect, "-w", &arg[2..], &mut gain, &mut window_size)?;
                index += 1;
            }
            _ if is_option_like(arg) => {
                return Err(EffectCommandParseError::UnsupportedOption {
                    effect,
                    option: arg.to_owned(),
                });
            }
            _ => {
                return Err(EffectCommandParseError::UnexpectedArgument {
                    effect,
                    argument: arg.to_owned(),
                });
            }
        }
    }

    Centercut::with_options(gain, bass_to_sides, window_size)
        .map(EffectCommand::Centercut)
        .map_err(|source| EffectCommandParseError::InvalidEffectConfig {
            effect,
            argument: "centercut",
            source,
        })
}

pub(super) fn render_centercut(centercut: Centercut) -> Vec<String> {
    let mut tokens = vec!["centercut".to_owned()];

    if centercut.gain.to_bits() != 1.0_f32.to_bits() {
        tokens.push("-a".to_owned());
        tokens.push(render_f32(centercut.gain));
    }
    if centercut.bass_to_sides {
        tokens.push("-b".to_owned());
    }
    if centercut.window_size != 8192 {
        tokens.push("-w".to_owned());
        tokens.push(centercut.window_size.to_string());
    }

    tokens
}

fn apply_option(
    effect: &'static str,
    option: &str,
    value: &str,
    gain: &mut f32,
    window_size: &mut usize,
) -> CommandResult<()> {
    match option {
        "-a" => {
            *gain = parse_f32(effect, "gain", value)?;
            Ok(())
        }
        "-w" => {
            *window_size = value.parse::<usize>().map_err(|source| {
                EffectCommandParseError::InvalidFrameCount {
                    effect,
                    argument: "window-size",
                    value: value.to_owned(),
                    source,
                }
            })?;
            Ok(())
        }
        _ => unreachable!("centercut parser only passes supported options"),
    }
}

fn option_argument(option: &str) -> &'static str {
    match option {
        "-a" => "gain",
        "-w" => "window-size",
        _ => unreachable!("centercut parser only asks for supported option arguments"),
    }
}

#[cfg(test)]
mod tests {
    use super::parse_centercut;
    use crate::{Centercut, EffectCommand, EffectCommandParseError, EffectError};

    #[test]
    fn parses_default_and_explicit_options() {
        assert_eq!(
            parse_centercut("centercut", &[]).unwrap(),
            EffectCommand::Centercut(Centercut::new())
        );
        assert_eq!(
            parse_centercut("centercut", &["-a", "0.5", "-b", "-w", "16"]).unwrap(),
            EffectCommand::Centercut(Centercut::with_options(0.5, true, 16).unwrap())
        );
        assert_eq!(
            parse_centercut("centercut", &["-a0.25", "-w32"])
                .unwrap()
                .render_tokens(),
            ["centercut", "-a", "0.25", "-w", "32"]
        );
    }

    #[test]
    fn rejects_invalid_options_and_values() {
        assert_eq!(
            parse_centercut("centercut", &["extra"]).unwrap_err(),
            EffectCommandParseError::UnexpectedArgument {
                effect: "centercut",
                argument: "extra".to_owned(),
            }
        );
        assert_eq!(
            parse_centercut("centercut", &["-x"]).unwrap_err(),
            EffectCommandParseError::UnsupportedOption {
                effect: "centercut",
                option: "-x".to_owned(),
            }
        );
        assert_eq!(
            parse_centercut("centercut", &["-w", "12"]).unwrap_err(),
            EffectCommandParseError::InvalidEffectConfig {
                effect: "centercut",
                argument: "centercut",
                source: EffectError::InvalidCentercutWindowSize,
            }
        );
    }
}

use crate::command::{
    CommandResult, EffectCommand, EffectCommandParseError, is_option_like, reject_extra_arguments,
};
use crate::{EffectError, Hilbert};

pub(super) fn parse_hilbert(effect: &'static str, args: &[&str]) -> CommandResult<EffectCommand> {
    let hilbert = match args {
        [] => Hilbert::default_taps(),
        ["-n", taps] => Hilbert::with_taps(parse_taps(effect, taps)?).map_err(|source| {
            EffectCommandParseError::InvalidEffectConfig {
                effect,
                argument: "taps",
                source,
            }
        })?,
        [option] if option.starts_with("-n") && option.len() > 2 => {
            Hilbert::with_taps(parse_taps(effect, &option[2..])?).map_err(|source| {
                EffectCommandParseError::InvalidEffectConfig {
                    effect,
                    argument: "taps",
                    source,
                }
            })?
        }
        [option, ..] if is_option_like(option) => {
            return Err(EffectCommandParseError::UnsupportedOption {
                effect,
                option: (*option).to_owned(),
            });
        }
        _ => {
            reject_extra_arguments(effect, args)?;
            unreachable!("reject_extra_arguments returned for non-empty hilbert args");
        }
    };

    Ok(EffectCommand::Hilbert(hilbert))
}

pub(super) fn render_hilbert(hilbert: Hilbert) -> Vec<String> {
    match hilbert.taps() {
        Some(taps) => vec!["hilbert".to_owned(), "-n".to_owned(), taps.to_string()],
        None => vec!["hilbert".to_owned()],
    }
}

fn parse_taps(effect: &'static str, value: &str) -> CommandResult<u32> {
    value
        .parse::<u32>()
        .map_err(|source| EffectCommandParseError::InvalidFrameCount {
            effect,
            argument: "taps",
            value: value.to_owned(),
            source,
        })
        .and_then(|taps| {
            if !taps.is_multiple_of(2) && taps >= 3 {
                Ok(taps)
            } else {
                Err(EffectCommandParseError::InvalidEffectConfig {
                    effect,
                    argument: "taps",
                    source: EffectError::InvalidHilbert,
                })
            }
        })
}

#[cfg(test)]
mod tests {
    use super::parse_hilbert;
    use crate::{EffectCommand, EffectCommandParseError, EffectError, Hilbert};

    #[test]
    fn parses_default_and_explicit_taps() {
        assert_eq!(
            parse_hilbert("hilbert", &[]).unwrap(),
            EffectCommand::Hilbert(Hilbert::default_taps())
        );
        assert_eq!(
            parse_hilbert("hilbert", &["-n", "5"]).unwrap(),
            EffectCommand::Hilbert(Hilbert::with_taps(5).unwrap())
        );
        assert_eq!(
            parse_hilbert("hilbert", &["-n7"]).unwrap().render_tokens(),
            ["hilbert", "-n", "7"]
        );
    }

    #[test]
    fn rejects_bad_taps_arguments_and_options() {
        assert_eq!(
            parse_hilbert("hilbert", &["-n"]).unwrap_err(),
            EffectCommandParseError::UnsupportedOption {
                effect: "hilbert",
                option: "-n".to_owned(),
            }
        );
        assert_eq!(
            parse_hilbert("hilbert", &["-x"]).unwrap_err(),
            EffectCommandParseError::UnsupportedOption {
                effect: "hilbert",
                option: "-x".to_owned(),
            }
        );
        assert_eq!(
            parse_hilbert("hilbert", &["-n", "4"]).unwrap_err(),
            EffectCommandParseError::InvalidEffectConfig {
                effect: "hilbert",
                argument: "taps",
                source: EffectError::InvalidHilbert,
            }
        );
        assert!(matches!(
            parse_hilbert("hilbert", &["-n", "abc"]).unwrap_err(),
            EffectCommandParseError::InvalidFrameCount {
                argument: "taps",
                ..
            }
        ));
    }
}

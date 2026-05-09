use crate::Swap;
use crate::command::{CommandResult, EffectCommand, reject_extra_arguments};

pub(super) fn parse_swap(effect: &'static str, args: &[&str]) -> CommandResult<EffectCommand> {
    reject_extra_arguments(effect, args)?;

    Ok(EffectCommand::Swap(Swap::new()))
}

#[cfg(test)]
mod tests {
    use super::parse_swap;
    use crate::{EffectCommand, EffectCommandParseError, Swap};

    #[test]
    fn parses_bare_swap() {
        assert_eq!(
            parse_swap("swap", &[]).unwrap(),
            EffectCommand::Swap(Swap::new())
        );
        assert_eq!(parse_swap("swap", &[]).unwrap().render_tokens(), ["swap"]);
    }

    #[test]
    fn rejects_arguments_and_options() {
        assert_eq!(
            parse_swap("swap", &["extra"]).unwrap_err(),
            EffectCommandParseError::UnexpectedArgument {
                effect: "swap",
                argument: "extra".to_owned(),
            }
        );
        assert_eq!(
            parse_swap("swap", &["-x"]).unwrap_err(),
            EffectCommandParseError::UnsupportedOption {
                effect: "swap",
                option: "-x".to_owned(),
            }
        );
    }
}

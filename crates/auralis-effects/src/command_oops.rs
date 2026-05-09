use crate::Oops;
use crate::command::{CommandResult, EffectCommand, reject_extra_arguments};

pub(super) fn parse_oops(effect: &'static str, args: &[&str]) -> CommandResult<EffectCommand> {
    reject_extra_arguments(effect, args)?;

    Ok(EffectCommand::Oops(Oops::new()))
}

#[cfg(test)]
mod tests {
    use super::parse_oops;
    use crate::{EffectCommand, EffectCommandParseError, Oops};

    #[test]
    fn parses_bare_oops() {
        assert_eq!(
            parse_oops("oops", &[]).unwrap(),
            EffectCommand::Oops(Oops::new())
        );
        assert_eq!(parse_oops("oops", &[]).unwrap().render_tokens(), ["oops"]);
    }

    #[test]
    fn rejects_arguments_and_options() {
        assert_eq!(
            parse_oops("oops", &["extra"]).unwrap_err(),
            EffectCommandParseError::UnexpectedArgument {
                effect: "oops",
                argument: "extra".to_owned(),
            }
        );
        assert_eq!(
            parse_oops("oops", &["-x"]).unwrap_err(),
            EffectCommandParseError::UnsupportedOption {
                effect: "oops",
                option: "-x".to_owned(),
            }
        );
    }
}

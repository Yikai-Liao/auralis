use crate::Earwax;
use crate::command::{CommandResult, EffectCommand, reject_extra_arguments};

pub(super) fn parse_earwax(effect: &'static str, args: &[&str]) -> CommandResult<EffectCommand> {
    reject_extra_arguments(effect, args)?;

    Ok(EffectCommand::Earwax(Earwax::new()))
}

#[cfg(test)]
mod tests {
    use super::parse_earwax;
    use crate::{Earwax, EffectCommand, EffectCommandParseError};

    #[test]
    fn parses_bare_earwax() {
        assert_eq!(
            parse_earwax("earwax", &[]).unwrap(),
            EffectCommand::Earwax(Earwax::new())
        );
        assert_eq!(
            parse_earwax("earwax", &[]).unwrap().render_tokens(),
            ["earwax"]
        );
    }

    #[test]
    fn rejects_arguments_and_options() {
        assert_eq!(
            parse_earwax("earwax", &["extra"]).unwrap_err(),
            EffectCommandParseError::UnexpectedArgument {
                effect: "earwax",
                argument: "extra".to_owned(),
            }
        );
        assert_eq!(
            parse_earwax("earwax", &["-x"]).unwrap_err(),
            EffectCommandParseError::UnsupportedOption {
                effect: "earwax",
                option: "-x".to_owned(),
            }
        );
    }
}

use crate::Deemph;
use crate::command::{CommandResult, EffectCommand, reject_extra_arguments};

pub(super) fn parse_deemph(effect: &'static str, args: &[&str]) -> CommandResult<EffectCommand> {
    reject_extra_arguments(effect, args)?;

    Ok(EffectCommand::Deemph(Deemph::new()))
}

#[cfg(test)]
mod tests {
    use super::parse_deemph;
    use crate::{Deemph, EffectCommand, EffectCommandParseError};

    #[test]
    fn parses_bare_deemph() {
        assert_eq!(
            parse_deemph("deemph", &[]).unwrap(),
            EffectCommand::Deemph(Deemph::new())
        );
        assert_eq!(
            parse_deemph("deemph", &[]).unwrap().render_tokens(),
            ["deemph"]
        );
    }

    #[test]
    fn rejects_arguments_and_options() {
        assert_eq!(
            parse_deemph("deemph", &["extra"]).unwrap_err(),
            EffectCommandParseError::UnexpectedArgument {
                effect: "deemph",
                argument: "extra".to_owned(),
            }
        );
        assert_eq!(
            parse_deemph("deemph", &["-x"]).unwrap_err(),
            EffectCommandParseError::UnsupportedOption {
                effect: "deemph",
                option: "-x".to_owned(),
            }
        );
    }
}

use crate::Riaa;
use crate::command::{CommandResult, EffectCommand, reject_extra_arguments};

pub(super) fn parse_riaa(effect: &'static str, args: &[&str]) -> CommandResult<EffectCommand> {
    reject_extra_arguments(effect, args)?;

    Ok(EffectCommand::Riaa(Riaa::new()))
}

#[cfg(test)]
mod tests {
    use super::parse_riaa;
    use crate::{EffectCommand, EffectCommandParseError, Riaa};

    #[test]
    fn parses_bare_riaa() {
        assert_eq!(
            parse_riaa("riaa", &[]).unwrap(),
            EffectCommand::Riaa(Riaa::new())
        );
        assert_eq!(parse_riaa("riaa", &[]).unwrap().render_tokens(), ["riaa"]);
    }

    #[test]
    fn rejects_arguments_and_options() {
        assert_eq!(
            parse_riaa("riaa", &["extra"]).unwrap_err(),
            EffectCommandParseError::UnexpectedArgument {
                effect: "riaa",
                argument: "extra".to_owned(),
            }
        );
        assert_eq!(
            parse_riaa("riaa", &["-x"]).unwrap_err(),
            EffectCommandParseError::UnsupportedOption {
                effect: "riaa",
                option: "-x".to_owned(),
            }
        );
    }
}

use crate::Repeat;
use crate::command::{
    CommandResult, EffectCommand, EffectCommandParseError, parse_frame_count,
    reject_extra_arguments,
};

pub(super) fn parse_repeat(effect: &'static str, args: &[&str]) -> CommandResult<EffectCommand> {
    let count = args
        .first()
        .map(|value| parse_frame_count(effect, "count", value))
        .transpose()?
        .map_or(1, auralis_core::FrameCount::as_u64);
    reject_extra_arguments(effect, args.get(1..).unwrap_or_default())?;

    Repeat::new(count)
        .map(EffectCommand::Repeat)
        .map_err(|source| EffectCommandParseError::InvalidEffectConfig {
            effect,
            argument: "repeat",
            source,
        })
}

pub(super) fn render_repeat(repeat: Repeat) -> Vec<String> {
    vec!["repeat".to_owned(), repeat.count.to_string()]
}

#[cfg(test)]
mod tests {
    use super::parse_repeat;
    use crate::{EffectCommand, EffectCommandParseError, EffectError, Repeat};

    #[test]
    fn parses_default_and_explicit_count() {
        assert_eq!(
            parse_repeat("repeat", &[]).unwrap(),
            EffectCommand::Repeat(Repeat::default())
        );
        assert_eq!(
            parse_repeat("repeat", &["2"]).unwrap(),
            EffectCommand::Repeat(Repeat::new(2).unwrap())
        );
        assert_eq!(
            parse_repeat("repeat", &[]).unwrap().render_tokens(),
            ["repeat", "1"]
        );
    }

    #[test]
    fn rejects_indefinite_invalid_and_extra_arguments() {
        assert_eq!(
            parse_repeat("repeat", &["-"]).unwrap_err(),
            EffectCommandParseError::UnsupportedOption {
                effect: "repeat",
                option: "-".to_owned(),
            }
        );
        assert_eq!(
            parse_repeat("repeat", &["4294967295"]).unwrap_err(),
            EffectCommandParseError::InvalidEffectConfig {
                effect: "repeat",
                argument: "repeat",
                source: EffectError::InvalidRepeatCount,
            }
        );
        assert_eq!(
            parse_repeat("repeat", &["1", "extra"]).unwrap_err(),
            EffectCommandParseError::UnexpectedArgument {
                effect: "repeat",
                argument: "extra".to_owned(),
            }
        );
    }
}

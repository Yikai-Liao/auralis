use crate::Upsample;
use crate::command::{
    CommandResult, EffectCommand, EffectCommandParseError, is_option_like, reject_extra_arguments,
};

pub(super) fn parse_upsample(effect: &'static str, args: &[&str]) -> CommandResult<EffectCommand> {
    let factor =
        match args.first() {
            Some(value) if is_option_like(value) => {
                return Err(EffectCommandParseError::UnsupportedOption {
                    effect,
                    option: (*value).to_owned(),
                });
            }
            Some(value) => value.parse::<u32>().map_err(|source| {
                EffectCommandParseError::InvalidFrameCount {
                    effect,
                    argument: "factor",
                    value: (*value).to_owned(),
                    source,
                }
            })?,
            None => Upsample::default().factor,
        };
    reject_extra_arguments(effect, args.get(1..).unwrap_or_default())?;

    Upsample::new(factor)
        .map(EffectCommand::Upsample)
        .map_err(|source| EffectCommandParseError::InvalidEffectConfig {
            effect,
            argument: "factor",
            source,
        })
}

pub(super) fn render_upsample(upsample: Upsample) -> Vec<String> {
    vec!["upsample".to_owned(), upsample.factor.to_string()]
}

#[cfg(test)]
mod tests {
    use super::parse_upsample;
    use crate::{EffectCommand, EffectCommandParseError, EffectError, Upsample};

    #[test]
    fn parses_default_and_explicit_factor() {
        assert_eq!(
            parse_upsample("upsample", &[]).unwrap(),
            EffectCommand::Upsample(Upsample::default())
        );
        assert_eq!(
            parse_upsample("upsample", &["3"]).unwrap(),
            EffectCommand::Upsample(Upsample::new(3).unwrap())
        );
        assert_eq!(
            parse_upsample("upsample", &[]).unwrap().render_tokens(),
            ["upsample", "2"]
        );
    }

    #[test]
    fn rejects_invalid_factor_option_and_extra_argument() {
        assert_eq!(
            parse_upsample("upsample", &["0"]).unwrap_err(),
            EffectCommandParseError::InvalidEffectConfig {
                effect: "upsample",
                argument: "factor",
                source: EffectError::InvalidUpsampleFactor,
            }
        );
        assert!(matches!(
            parse_upsample("upsample", &["-1"]).unwrap_err(),
            EffectCommandParseError::InvalidFrameCount {
                effect: "upsample",
                argument: "factor",
                ..
            }
        ));
        assert_eq!(
            parse_upsample("upsample", &["--bad"]).unwrap_err(),
            EffectCommandParseError::UnsupportedOption {
                effect: "upsample",
                option: "--bad".to_owned(),
            }
        );
        assert_eq!(
            parse_upsample("upsample", &["2", "extra"]).unwrap_err(),
            EffectCommandParseError::UnexpectedArgument {
                effect: "upsample",
                argument: "extra".to_owned(),
            }
        );
    }
}

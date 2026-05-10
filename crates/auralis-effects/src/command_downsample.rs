use crate::Downsample;
use crate::command::{
    CommandResult, EffectCommand, EffectCommandParseError, is_option_like, reject_extra_arguments,
};

pub(super) fn parse_downsample(
    effect: &'static str,
    args: &[&str],
) -> CommandResult<EffectCommand> {
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
            None => Downsample::default().factor,
        };
    reject_extra_arguments(effect, args.get(1..).unwrap_or_default())?;

    Downsample::new(factor)
        .map(EffectCommand::Downsample)
        .map_err(|source| EffectCommandParseError::InvalidEffectConfig {
            effect,
            argument: "factor",
            source,
        })
}

pub(super) fn render_downsample(downsample: Downsample) -> Vec<String> {
    vec!["downsample".to_owned(), downsample.factor.to_string()]
}

#[cfg(test)]
mod tests {
    use super::parse_downsample;
    use crate::{Downsample, EffectCommand, EffectCommandParseError, EffectError};

    #[test]
    fn parses_default_and_explicit_factor() {
        assert_eq!(
            parse_downsample("downsample", &[]).unwrap(),
            EffectCommand::Downsample(Downsample::default())
        );
        assert_eq!(
            parse_downsample("downsample", &["3"]).unwrap(),
            EffectCommand::Downsample(Downsample::new(3).unwrap())
        );
        assert_eq!(
            parse_downsample("downsample", &[]).unwrap().render_tokens(),
            ["downsample", "2"]
        );
    }

    #[test]
    fn rejects_invalid_factor_option_and_extra_argument() {
        assert_eq!(
            parse_downsample("downsample", &["0"]).unwrap_err(),
            EffectCommandParseError::InvalidEffectConfig {
                effect: "downsample",
                argument: "factor",
                source: EffectError::InvalidDownsampleFactor,
            }
        );
        assert!(matches!(
            parse_downsample("downsample", &["-1"]).unwrap_err(),
            EffectCommandParseError::InvalidFrameCount {
                effect: "downsample",
                argument: "factor",
                ..
            }
        ));
        assert_eq!(
            parse_downsample("downsample", &["--bad"]).unwrap_err(),
            EffectCommandParseError::UnsupportedOption {
                effect: "downsample",
                option: "--bad".to_owned(),
            }
        );
        assert_eq!(
            parse_downsample("downsample", &["2", "extra"]).unwrap_err(),
            EffectCommandParseError::UnexpectedArgument {
                effect: "downsample",
                argument: "extra".to_owned(),
            }
        );
    }
}

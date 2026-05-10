use crate::MCompand;
use crate::command::{CommandResult, EffectCommand, EffectCommandParseError};
use crate::command_compand::{render_compand_args, render_frequency_hz};

pub(super) fn parse_mcompand(effect: &'static str, args: &[&str]) -> CommandResult<EffectCommand> {
    if args.is_empty() {
        return Err(EffectCommandParseError::MissingArgument {
            effect,
            argument: "quoted_compand_args",
        });
    }

    MCompand::parse_sox_args(args)
        .map(EffectCommand::MCompand)
        .map_err(|source| EffectCommandParseError::InvalidEffectConfig {
            effect,
            argument: "mcompand",
            source,
        })
}

pub(super) fn render_mcompand(mcompand: &MCompand) -> Vec<String> {
    let mut tokens = vec!["mcompand".to_owned()];

    for band in mcompand.bands() {
        tokens.push(render_compand_args(band.compand()).join(" "));
        if let Some(frequency_hz) = band.top_frequency_hz() {
            tokens.push(render_frequency_hz(frequency_hz));
        }
    }

    tokens
}

#[cfg(test)]
mod tests {
    use super::parse_mcompand;
    use crate::{EffectCommand, EffectCommandParseError, EffectError, MCompand};

    #[test]
    fn parses_and_renders_mcompand_arguments() {
        let command = parse_mcompand(
            "mcompand",
            &["0,0 -60,-60,0,0", "1k", "0.01,0.1 3:-70,-60,0,-3 -1 -20"],
        )
        .unwrap();

        assert_eq!(
            command,
            EffectCommand::MCompand(
                MCompand::parse_sox_args(&[
                    "0,0 -60,-60,0,0",
                    "1k",
                    "0.01,0.1 3:-70,-60,0,-3 -1 -20",
                ])
                .unwrap()
            )
        );
        assert_eq!(
            command.render_tokens(),
            [
                "mcompand",
                "0,0 -60,-60,0,0",
                "1000",
                "0.01,0.1 3:-70,-60,0,-3 -1 -20",
            ]
        );
    }

    #[test]
    fn rejects_missing_and_invalid_arguments() {
        assert_eq!(
            parse_mcompand("mcompand", &[]).unwrap_err(),
            EffectCommandParseError::MissingArgument {
                effect: "mcompand",
                argument: "quoted_compand_args",
            }
        );
        assert_eq!(
            parse_mcompand("mcompand", &["0,0 -60,-60,0,0", "100"]).unwrap_err(),
            EffectCommandParseError::InvalidEffectConfig {
                effect: "mcompand",
                argument: "mcompand",
                source: EffectError::InvalidMCompand,
            }
        );
    }
}

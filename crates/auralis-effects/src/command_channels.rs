use auralis_core::ChannelCount;

use crate::Channels;
use crate::command::{
    CommandResult, EffectCommand, EffectCommandParseError, reject_extra_arguments, required_arg,
};

pub(super) fn parse_channels(effect: &'static str, args: &[&str]) -> CommandResult<EffectCommand> {
    let value = required_arg(effect, args, "channels")?;
    let channels =
        value
            .parse::<u16>()
            .map_err(|source| EffectCommandParseError::InvalidFrameCount {
                effect,
                argument: "channels",
                value: value.to_owned(),
                source,
            })?;
    let target_channels = ChannelCount::new(channels).map_err(|source| {
        EffectCommandParseError::InvalidCoreValue {
            effect,
            argument: "channels",
            source,
        }
    })?;
    reject_extra_arguments(effect, args.get(1..).unwrap_or_default())?;

    Ok(EffectCommand::Channels(Channels::new(target_channels)))
}

pub(super) fn render_channels(channels: Channels) -> Vec<String> {
    vec![
        "channels".to_owned(),
        channels.target_channels.as_u16().to_string(),
    ]
}

#[cfg(test)]
mod tests {
    use super::parse_channels;
    use crate::{Channels, EffectCommand, EffectCommandParseError};
    use auralis_core::{AuralisError, ChannelCount};

    #[test]
    fn parses_required_target_channel_count() {
        assert_eq!(
            parse_channels("channels", &["2"]).unwrap(),
            EffectCommand::Channels(Channels::new(ChannelCount::new(2).unwrap()))
        );
        assert_eq!(
            parse_channels("channels", &["2"]).unwrap().render_tokens(),
            ["channels", "2"]
        );
    }

    #[test]
    fn rejects_missing_zero_invalid_and_extra_arguments() {
        assert_eq!(
            parse_channels("channels", &[]).unwrap_err(),
            EffectCommandParseError::MissingArgument {
                effect: "channels",
                argument: "channels",
            }
        );
        assert_eq!(
            parse_channels("channels", &["0"]).unwrap_err(),
            EffectCommandParseError::InvalidCoreValue {
                effect: "channels",
                argument: "channels",
                source: AuralisError::InvalidChannelCount,
            }
        );
        assert!(matches!(
            parse_channels("channels", &["70000"]).unwrap_err(),
            EffectCommandParseError::InvalidFrameCount {
                effect: "channels",
                argument: "channels",
                ..
            }
        ));
        assert_eq!(
            parse_channels("channels", &["1", "extra"]).unwrap_err(),
            EffectCommandParseError::UnexpectedArgument {
                effect: "channels",
                argument: "extra".to_owned(),
            }
        );
    }
}

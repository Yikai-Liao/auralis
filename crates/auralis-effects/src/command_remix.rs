use crate::command::{CommandResult, EffectCommand, EffectCommandParseError};
use crate::{Remix, RemixOutputSpec, RemixSource};

pub(super) fn parse_remix(effect: &'static str, args: &[&str]) -> CommandResult<EffectCommand> {
    if args.is_empty() {
        return Err(EffectCommandParseError::MissingArgument {
            effect,
            argument: "out-spec",
        });
    }
    if matches!(args.first().copied(), Some("-a" | "-m" | "-p")) {
        return Err(EffectCommandParseError::UnsupportedOption {
            effect,
            option: args[0].to_owned(),
        });
    }

    let outputs = args
        .iter()
        .map(|arg| parse_output_spec(effect, arg))
        .collect::<CommandResult<Vec<_>>>()?;
    Remix::new(outputs)
        .map(EffectCommand::Remix)
        .map_err(|source| EffectCommandParseError::InvalidEffectConfig {
            effect,
            argument: "out-spec",
            source,
        })
}

pub(super) fn render_remix(remix: &Remix) -> Vec<String> {
    std::iter::once("remix".to_owned())
        .chain(remix.outputs.iter().map(render_output_spec))
        .collect()
}

fn parse_output_spec(effect: &'static str, value: &str) -> CommandResult<RemixOutputSpec> {
    if value == "0" {
        return Ok(RemixOutputSpec::silent());
    }

    let sources = value
        .split(',')
        .map(|part| parse_source(effect, part))
        .collect::<CommandResult<Vec<_>>>()?;

    RemixOutputSpec::new(sources).map_err(|source| EffectCommandParseError::InvalidEffectConfig {
        effect,
        argument: "out-spec",
        source,
    })
}

fn parse_source(effect: &'static str, value: &str) -> CommandResult<RemixSource> {
    if let Some(option) = value.chars().find(|char| matches!(char, 'v' | 'p' | 'i')) {
        return Err(EffectCommandParseError::UnsupportedOption {
            effect,
            option: option.to_string(),
        });
    }

    if value == "-" {
        return Ok(RemixSource::all());
    }

    if let Some((start, end)) = value.split_once('-') {
        let source = RemixSource::range(
            parse_optional_channel(effect, start)?,
            parse_optional_channel(effect, end)?,
        )
        .map_err(|source| EffectCommandParseError::InvalidEffectConfig {
            effect,
            argument: "out-spec",
            source,
        })?;
        return Ok(source);
    }

    let channel = parse_channel(effect, value)?;
    RemixSource::channel(channel).map_err(|source| EffectCommandParseError::InvalidEffectConfig {
        effect,
        argument: "out-spec",
        source,
    })
}

fn parse_optional_channel(effect: &'static str, value: &str) -> CommandResult<Option<u16>> {
    if value.is_empty() {
        Ok(None)
    } else {
        parse_channel(effect, value).map(Some)
    }
}

fn parse_channel(effect: &'static str, value: &str) -> CommandResult<u16> {
    value
        .parse::<u16>()
        .map_err(|source| EffectCommandParseError::InvalidFrameCount {
            effect,
            argument: "channel",
            value: value.to_owned(),
            source,
        })
}

fn render_output_spec(output: &RemixOutputSpec) -> String {
    if output.sources.is_empty() {
        return "0".to_owned();
    }

    output
        .sources
        .iter()
        .map(|source| match source {
            RemixSource::Channel(channel) => channel.to_string(),
            RemixSource::Range { start, end } => match (start, end) {
                (None, None) => "-".to_owned(),
                (Some(start), None) => format!("{start}-"),
                (None, Some(end)) => format!("-{end}"),
                (Some(start), Some(end)) => format!("{start}-{end}"),
            },
        })
        .collect::<Vec<_>>()
        .join(",")
}

#[cfg(test)]
mod tests {
    use super::parse_remix;
    use crate::{EffectCommand, EffectCommandParseError, Remix, RemixOutputSpec, RemixSource};

    #[test]
    fn parses_basic_routing_specs() {
        let expected = Remix::new([
            RemixOutputSpec::new([
                RemixSource::channel(1).unwrap(),
                RemixSource::range(Some(3), Some(2)).unwrap(),
                RemixSource::all(),
            ])
            .unwrap(),
            RemixOutputSpec::silent(),
            RemixOutputSpec::new([RemixSource::range(None, Some(2)).unwrap()]).unwrap(),
        ])
        .unwrap();

        assert_eq!(
            parse_remix("remix", &["1,3-2,-", "0", "-2"]).unwrap(),
            EffectCommand::Remix(expected)
        );
        assert_eq!(
            parse_remix("remix", &["1,3-2,-", "0", "-2"])
                .unwrap()
                .render_tokens(),
            ["remix", "1,3-2,-", "0", "-2"]
        );
    }

    #[test]
    fn rejects_missing_invalid_and_future_gain_modifier_forms() {
        assert_eq!(
            parse_remix("remix", &[]).unwrap_err(),
            EffectCommandParseError::MissingArgument {
                effect: "remix",
                argument: "out-spec",
            }
        );
        assert!(matches!(
            parse_remix("remix", &["1v0.5"]).unwrap_err(),
            EffectCommandParseError::UnsupportedOption { effect: "remix", option }
                if option == "v"
        ));
        assert!(matches!(
            parse_remix("remix", &["-p", "1,2"]).unwrap_err(),
            EffectCommandParseError::UnsupportedOption { effect: "remix", option }
                if option == "-p"
        ));
        assert!(matches!(
            parse_remix("remix", &["0,1"]).unwrap_err(),
            EffectCommandParseError::InvalidEffectConfig {
                effect: "remix",
                argument: "out-spec",
                source: crate::EffectError::InvalidRemixRouting,
            }
        ));
    }
}

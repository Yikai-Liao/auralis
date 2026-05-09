use crate::command::{CommandResult, EffectCommand, EffectCommandParseError};
use crate::{Remix, RemixGain, RemixLevelMode, RemixOutputSpec, RemixSource};
use auralis_core::Decibels;

pub(super) fn parse_remix(effect: &'static str, args: &[&str]) -> CommandResult<EffectCommand> {
    if args.is_empty() {
        return Err(EffectCommandParseError::MissingArgument {
            effect,
            argument: "out-spec",
        });
    }

    let (level_mode, mix_power, args) = parse_options(effect, args)?;
    if args.is_empty() {
        return Err(EffectCommandParseError::MissingArgument {
            effect,
            argument: "out-spec",
        });
    }

    let outputs = args
        .iter()
        .map(|arg| parse_output_spec(effect, arg))
        .collect::<CommandResult<Vec<_>>>()?;
    Remix::with_level_options(outputs, level_mode, mix_power)
        .map(EffectCommand::Remix)
        .map_err(|source| EffectCommandParseError::InvalidEffectConfig {
            effect,
            argument: "out-spec",
            source,
        })
}

pub(super) fn render_remix(remix: &Remix) -> Vec<String> {
    let mut tokens = Vec::with_capacity(1 + remix.outputs.len() + 2);
    tokens.push("remix".to_owned());
    match remix.level_mode {
        RemixLevelMode::SemiAutomatic => {}
        RemixLevelMode::Automatic => tokens.push("-a".to_owned()),
        RemixLevelMode::Manual => tokens.push("-m".to_owned()),
    }
    if remix.mix_power {
        tokens.push("-p".to_owned());
    }
    tokens.extend(remix.outputs.iter().map(render_output_spec));
    tokens
}

fn parse_options<'args>(
    effect: &'static str,
    args: &'args [&'args str],
) -> CommandResult<(RemixLevelMode, bool, &'args [&'args str])> {
    let mut level_mode = RemixLevelMode::SemiAutomatic;
    let mut mix_power = false;
    let mut start = 0;

    if matches!(args.first().copied(), Some("-a" | "-m")) {
        level_mode = if args[0] == "-a" {
            RemixLevelMode::Automatic
        } else {
            RemixLevelMode::Manual
        };
        start += 1;
    }

    if matches!(args.get(start).copied(), Some("-p")) {
        mix_power = true;
        start += 1;
    }

    if matches!(args.get(start).copied(), Some("-a" | "-m" | "-p")) {
        return Err(EffectCommandParseError::InvalidOptionCombination {
            effect,
            options: "remix accepts at most one of -a/-m followed by optional -p",
        });
    }

    Ok((level_mode, mix_power, &args[start..]))
}

fn parse_output_spec(effect: &'static str, value: &str) -> CommandResult<RemixOutputSpec> {
    if value == "0" {
        return Ok(RemixOutputSpec::silent());
    }

    let specs = value
        .split(',')
        .map(|part| parse_source(effect, part))
        .collect::<CommandResult<Vec<_>>>()?;
    let (sources, gains): (Vec<_>, Vec<_>) = specs.into_iter().unzip();

    RemixOutputSpec::with_gains(sources, gains).map_err(|source| {
        EffectCommandParseError::InvalidEffectConfig {
            effect,
            argument: "out-spec",
            source,
        }
    })
}

fn parse_source(
    effect: &'static str,
    value: &str,
) -> CommandResult<(RemixSource, Option<RemixGain>)> {
    let (source_value, gain) = parse_gain_modifier(effect, value)?;

    if source_value == "-" {
        return Ok((RemixSource::all(), gain));
    }

    if let Some((start, end)) = source_value.split_once('-') {
        let source = RemixSource::range(
            parse_optional_channel(effect, start)?,
            parse_optional_channel(effect, end)?,
        )
        .map_err(|source| EffectCommandParseError::InvalidEffectConfig {
            effect,
            argument: "out-spec",
            source,
        })?;
        return Ok((source, gain));
    }

    let channel = parse_channel(effect, source_value)?;
    RemixSource::channel(channel)
        .map(|source| (source, gain))
        .map_err(|source| EffectCommandParseError::InvalidEffectConfig {
            effect,
            argument: "out-spec",
            source,
        })
}

fn parse_gain_modifier<'value>(
    effect: &'static str,
    value: &'value str,
) -> CommandResult<(&'value str, Option<RemixGain>)> {
    let Some((index, marker)) = value
        .char_indices()
        .find(|(_, char)| matches!(char, 'v' | 'p' | 'i'))
    else {
        return Ok((value, None));
    };

    let source = &value[..index];
    if source.is_empty() {
        return Err(EffectCommandParseError::InvalidEffectConfig {
            effect,
            argument: "out-spec",
            source: crate::EffectError::InvalidRemixRouting,
        });
    }

    let argument = &value[index + marker.len_utf8()..];
    let gain = match marker {
        'v' => {
            let multiplier = if argument.is_empty() {
                1.0
            } else {
                parse_number(effect, "voltage", argument)?
            };
            RemixGain::voltage(multiplier).map_err(|source| {
                EffectCommandParseError::InvalidEffectConfig {
                    effect,
                    argument: "voltage",
                    source,
                }
            })?
        }
        'p' | 'i' => {
            let decibels = if argument.is_empty() {
                Decibels::new(0.0).expect("zero dB is finite")
            } else {
                Decibels::new(parse_number(effect, "power", argument)?).map_err(|source| {
                    EffectCommandParseError::InvalidCoreValue {
                        effect,
                        argument: "power",
                        source,
                    }
                })?
            };
            if marker == 'p' {
                RemixGain::PowerDb(decibels)
            } else {
                RemixGain::InvertedPowerDb(decibels)
            }
        }
        _ => unreachable!("modifier marker is filtered above"),
    };

    Ok((source, Some(gain)))
}

fn parse_number(effect: &'static str, argument: &'static str, value: &str) -> CommandResult<f64> {
    value
        .parse::<f64>()
        .map_err(|source| EffectCommandParseError::InvalidNumber {
            effect,
            argument,
            value: value.to_owned(),
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
        .zip(&output.gains)
        .map(|(source, gain)| {
            let mut rendered = match source {
                RemixSource::Channel(channel) => channel.to_string(),
                RemixSource::Range { start, end } => match (start, end) {
                    (None, None) => "-".to_owned(),
                    (Some(start), None) => format!("{start}-"),
                    (None, Some(end)) => format!("-{end}"),
                    (Some(start), Some(end)) => format!("{start}-{end}"),
                },
            };
            rendered.push_str(&render_gain_modifier(*gain));
            rendered
        })
        .collect::<Vec<_>>()
        .join(",")
}

fn render_gain_modifier(gain: Option<RemixGain>) -> String {
    match gain {
        None => String::new(),
        Some(RemixGain::Voltage(multiplier)) => format!("v{multiplier}"),
        Some(RemixGain::PowerDb(decibels)) => format!("p{}", decibels.as_f64()),
        Some(RemixGain::InvertedPowerDb(decibels)) => format!("i{}", decibels.as_f64()),
    }
}

#[cfg(test)]
mod tests {
    use super::parse_remix;
    use crate::{
        EffectCommand, EffectCommandParseError, Remix, RemixGain, RemixLevelMode, RemixOutputSpec,
        RemixSource,
    };
    use auralis_core::Decibels;

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
    fn parses_gain_modifiers_and_level_options() {
        let expected = Remix::with_level_options(
            [
                RemixOutputSpec::with_gains(
                    [
                        RemixSource::channel(1).unwrap(),
                        RemixSource::range(Some(3), Some(2)).unwrap(),
                        RemixSource::all(),
                    ],
                    [
                        Some(RemixGain::voltage(0.5).unwrap()),
                        Some(RemixGain::PowerDb(Decibels::new(-6.0).unwrap())),
                        Some(RemixGain::InvertedPowerDb(Decibels::new(0.0).unwrap())),
                    ],
                )
                .unwrap(),
                RemixOutputSpec::new([RemixSource::range(None, Some(2)).unwrap()]).unwrap(),
            ],
            RemixLevelMode::Automatic,
            true,
        )
        .unwrap();

        assert_eq!(
            parse_remix("remix", &["-a", "-p", "1v0.5,3-2p-6,-i0", "-2"]).unwrap(),
            EffectCommand::Remix(expected)
        );
        assert_eq!(
            parse_remix("remix", &["-a", "-p", "1v0.5,3-2p-6,-i0", "-2"])
                .unwrap()
                .render_tokens(),
            ["remix", "-a", "-p", "1v0.5,3-2p-6,-i0", "-2"]
        );
    }

    #[test]
    fn rejects_missing_invalid_and_bad_option_combinations() {
        assert_eq!(
            parse_remix("remix", &[]).unwrap_err(),
            EffectCommandParseError::MissingArgument {
                effect: "remix",
                argument: "out-spec",
            }
        );
        assert!(matches!(
            parse_remix("remix", &["-a", "-m", "1"]).unwrap_err(),
            EffectCommandParseError::InvalidOptionCombination {
                effect: "remix",
                ..
            }
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

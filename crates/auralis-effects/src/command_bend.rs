use auralis_core::FrameCount;

use crate::command::{
    CommandResult, EffectCommand, EffectCommandParseError, parse_f64, render_f64, required_arg,
};
use crate::{Bend, BendAmount, BendAnchor, BendPosition, BendSegment};

pub(super) fn parse_bend(effect: &'static str, args: &[&str]) -> CommandResult<EffectCommand> {
    let (frame_rate, oversample, args) = parse_options(effect, args)?;
    required_arg(effect, args, "start,cents,end")?;
    let segments = args
        .iter()
        .copied()
        .map(|arg| parse_segment(effect, arg))
        .collect::<CommandResult<Vec<_>>>()?;

    Bend::with_options(frame_rate, oversample, segments)
        .map(EffectCommand::Bend)
        .map_err(|source| EffectCommandParseError::InvalidEffectConfig {
            effect,
            argument: "bend",
            source,
        })
}

pub(super) fn render_bend(bend: &Bend) -> Vec<String> {
    let mut tokens = vec!["bend".to_owned()];
    if bend.frame_rate != 25 {
        tokens.push("-f".to_owned());
        tokens.push(bend.frame_rate.to_string());
    }
    if bend.oversample != 16 {
        tokens.push("-o".to_owned());
        tokens.push(bend.oversample.to_string());
    }
    tokens.extend(bend.segments().iter().copied().map(render_segment));
    tokens
}

fn parse_options<'args>(
    effect: &'static str,
    args: &'args [&'args str],
) -> CommandResult<(u32, u32, &'args [&'args str])> {
    let mut frame_rate = 25;
    let mut oversample = 16;
    let mut index = 0;

    while let Some(option) = args.get(index).copied() {
        match option {
            "-f" => {
                index += 1;
                frame_rate = parse_u32_option(effect, "frame-rate", args.get(index).copied())?;
            }
            "-o" => {
                index += 1;
                oversample = parse_u32_option(effect, "oversample", args.get(index).copied())?;
            }
            option if crate::command::is_option_like(option) => {
                return Err(EffectCommandParseError::UnsupportedOption {
                    effect,
                    option: option.to_owned(),
                });
            }
            _ => break,
        }
        index += 1;
    }

    Ok((frame_rate, oversample, &args[index..]))
}

fn parse_u32_option(
    effect: &'static str,
    argument: &'static str,
    value: Option<&str>,
) -> CommandResult<u32> {
    let value = value.ok_or(EffectCommandParseError::MissingArgument { effect, argument })?;
    let parsed = parse_f64(effect, argument, value)?;
    if parsed.fract() == 0.0 && parsed >= 0.0 && parsed <= f64::from(u32::MAX) {
        #[allow(
            clippy::cast_possible_truncation,
            clippy::cast_sign_loss,
            reason = "value was checked as an integer in the u32 range"
        )]
        {
            Ok(parsed as u32)
        }
    } else {
        Err(EffectCommandParseError::InvalidEffectConfig {
            effect,
            argument: "bend",
            source: crate::EffectError::InvalidBend,
        })
    }
}

fn parse_segment(effect: &'static str, value: &str) -> CommandResult<BendSegment> {
    let parts = value.split(',').collect::<Vec<_>>();
    if parts.len() != 3 || parts.iter().any(|part| part.is_empty()) {
        return Err(EffectCommandParseError::InvalidEffectConfig {
            effect,
            argument: "bend",
            source: crate::EffectError::InvalidBend,
        });
    }

    BendSegment::new(
        parse_position(effect, parts[0])?,
        parse_f64(effect, "cents", parts[1])?,
        parse_position(effect, parts[2])?,
    )
    .map_err(|source| EffectCommandParseError::InvalidEffectConfig {
        effect,
        argument: "bend",
        source,
    })
}

fn parse_position(effect: &'static str, value: &str) -> CommandResult<BendPosition> {
    let (anchor, amount_text) = match value.as_bytes().first().copied() {
        Some(b'+') => (BendAnchor::Previous, &value[1..]),
        Some(_) => (BendAnchor::Start, value),
        None => {
            return Err(EffectCommandParseError::InvalidEffectConfig {
                effect,
                argument: "bend",
                source: crate::EffectError::InvalidBend,
            });
        }
    };
    let amount = if let Some(frames) = amount_text.strip_suffix('s') {
        BendAmount::Frames(parse_frame_amount(effect, frames)?)
    } else {
        BendAmount::Seconds(parse_f64(effect, "position", amount_text)?)
    };
    Ok(BendPosition::new(anchor, amount))
}

fn parse_frame_amount(effect: &'static str, value: &str) -> CommandResult<FrameCount> {
    value.parse::<u64>().map(FrameCount::new).map_err(|source| {
        EffectCommandParseError::InvalidFrameCount {
            effect,
            argument: "position",
            value: value.to_owned(),
            source,
        }
    })
}

fn render_segment(segment: BendSegment) -> String {
    format!(
        "{},{},{}",
        render_position(segment.start),
        render_f64(segment.cents),
        render_position(segment.end)
    )
}

fn render_position(position: BendPosition) -> String {
    let anchor = match position.anchor() {
        BendAnchor::Start => "",
        BendAnchor::Previous => "+",
    };
    let amount = match position.amount() {
        BendAmount::Frames(frames) => format!("{}s", frames.as_u64()),
        BendAmount::Seconds(seconds) => render_f64(seconds),
    };
    format!("{anchor}{amount}")
}

#[cfg(test)]
mod tests {
    use auralis_core::FrameCount;

    use super::parse_bend;
    use crate::{
        Bend, BendAmount, BendAnchor, BendPosition, BendSegment, EffectCommand,
        EffectCommandParseError, EffectError,
    };

    #[test]
    fn parses_default_and_explicit_options() {
        assert_eq!(
            parse_bend("bend", &["0,100,0.1"]).unwrap(),
            EffectCommand::Bend(
                Bend::new([BendSegment::new(
                    BendPosition::seconds(0.0).unwrap(),
                    100.0,
                    BendPosition::seconds(0.1).unwrap()
                )
                .unwrap()])
                .unwrap()
            )
        );
        assert_eq!(
            parse_bend("bend", &["-f", "40", "-o", "8", "0s,-50,+100s"]).unwrap(),
            EffectCommand::Bend(
                Bend::with_options(
                    40,
                    8,
                    [BendSegment::new(
                        BendPosition::frames(FrameCount::new(0)),
                        -50.0,
                        BendPosition::new(
                            BendAnchor::Previous,
                            BendAmount::Frames(FrameCount::new(100))
                        )
                    )
                    .unwrap()]
                )
                .unwrap()
            )
        );
    }

    #[test]
    fn renders_canonical_tokens() {
        assert_eq!(
            parse_bend("bend", &["-f", "40", "-o", "8", "0s,-50,+100s"])
                .unwrap()
                .render_tokens(),
            ["bend", "-f", "40", "-o", "8", "0s,-50,+100s"]
        );
    }

    #[test]
    fn rejects_missing_invalid_options_and_malformed_segments() {
        assert_eq!(
            parse_bend("bend", &[]).unwrap_err(),
            EffectCommandParseError::MissingArgument {
                effect: "bend",
                argument: "start,cents,end"
            }
        );
        assert_eq!(
            parse_bend("bend", &["-x", "0,0,0"]).unwrap_err(),
            EffectCommandParseError::UnsupportedOption {
                effect: "bend",
                option: "-x".to_owned()
            }
        );
        assert_eq!(
            parse_bend("bend", &["-f", "9", "0,0,0"]).unwrap_err(),
            EffectCommandParseError::InvalidEffectConfig {
                effect: "bend",
                argument: "bend",
                source: EffectError::InvalidBend
            }
        );
        assert_eq!(
            parse_bend("bend", &["0,not-number,0"]).unwrap_err(),
            EffectCommandParseError::InvalidNumber {
                effect: "bend",
                argument: "cents",
                value: "not-number".to_owned(),
                source: "not-number".parse::<f64>().unwrap_err()
            }
        );
        assert!(matches!(
            parse_bend("bend", &["bad"]).unwrap_err(),
            EffectCommandParseError::InvalidEffectConfig { .. }
        ));
    }
}

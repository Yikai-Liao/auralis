use auralis_core::FrameCount;

use crate::{
    Silence, SilenceDuration, SilenceThreshold,
    command::{CommandResult, EffectCommand, EffectCommandParseError, render_f64, required_arg},
};

pub(super) fn parse_silence(effect: &'static str, args: &[&str]) -> CommandResult<EffectCommand> {
    let (leave_silence, args) = parse_options(effect, args)?;
    let above_text = required_arg(effect, args, "above-periods")?;
    let above_periods = parse_above_periods(effect, above_text)?;
    let mut offset = 1;

    let above = if above_periods == 0 {
        None
    } else {
        if args.len() < offset + 2 {
            return Err(EffectCommandParseError::MissingArgument {
                effect,
                argument: "duration threshold",
            });
        }
        let duration = parse_duration(effect, args[offset])?;
        let threshold = parse_threshold(effect, args[offset + 1])?;
        offset += 2;
        Some((duration, threshold))
    };

    let below = if offset < args.len() {
        if args.len() < offset + 3 {
            return Err(EffectCommandParseError::MissingArgument {
                effect,
                argument: "below-periods duration threshold",
            });
        }
        let below_periods = parse_below_periods(effect, args[offset])?;
        let duration = parse_duration(effect, args[offset + 1])?;
        let threshold = parse_threshold(effect, args[offset + 2])?;
        offset += 3;
        Some((below_periods, duration, threshold))
    } else {
        None
    };

    if let Some(argument) = args.get(offset) {
        return Err(EffectCommandParseError::UnexpectedArgument {
            effect,
            argument: (*argument).to_owned(),
        });
    }

    Silence::new(above_periods, above, below, leave_silence)
        .map(EffectCommand::Silence)
        .map_err(|source| EffectCommandParseError::InvalidEffectConfig {
            effect,
            argument: "silence",
            source,
        })
}

pub(super) fn render_silence(silence: &Silence) -> Vec<String> {
    let mut tokens = vec!["silence".to_owned()];
    if silence.leave_silence() {
        tokens.push("-l".to_owned());
    }
    tokens.push(silence.above_periods().to_string());
    if let Some(above) = silence.above() {
        tokens.push(render_duration(above.duration));
        tokens.push(render_threshold(above.threshold));
    }
    if let Some(below) = silence.below() {
        let periods = if silence.restart() {
            format!("-{}", below.periods)
        } else {
            below.periods.to_string()
        };
        tokens.push(periods);
        tokens.push(render_duration(below.duration));
        tokens.push(render_threshold(below.threshold));
    }
    tokens
}

fn parse_options<'args>(
    effect: &'static str,
    args: &'args [&'args str],
) -> CommandResult<(bool, &'args [&'args str])> {
    match args.first().copied() {
        Some("-l") => Ok((true, &args[1..])),
        Some(option) if crate::command::is_option_like(option) => {
            Err(EffectCommandParseError::UnsupportedOption {
                effect,
                option: option.to_owned(),
            })
        }
        _ => Ok((false, args)),
    }
}

fn parse_above_periods(effect: &'static str, value: &str) -> CommandResult<u32> {
    value
        .parse::<u32>()
        .map_err(|source| EffectCommandParseError::InvalidFrameCount {
            effect,
            argument: "above-periods",
            value: value.to_owned(),
            source,
        })
}

fn parse_below_periods(effect: &'static str, value: &str) -> CommandResult<i32> {
    value
        .parse::<i32>()
        .map_err(|_| EffectCommandParseError::InvalidEffectConfig {
            effect,
            argument: "silence",
            source: crate::EffectError::InvalidSilence,
        })
}

fn parse_duration(effect: &'static str, value: &str) -> CommandResult<SilenceDuration> {
    if value.is_empty() {
        return invalid_silence(effect);
    }
    let parse_seconds = value.contains(':')
        || (value.contains('.') && !value.contains('e') && !value.contains('E'))
        || value.ends_with('t');
    if parse_seconds {
        let seconds = value.strip_suffix('t').unwrap_or(value);
        return parse_seconds_duration(effect, seconds);
    }

    let frames = value.strip_suffix('s').unwrap_or(value);
    parse_frame_duration(effect, frames)
}

#[allow(
    clippy::cast_possible_truncation,
    clippy::cast_precision_loss,
    clippy::cast_sign_loss,
    reason = "SoX-ng accepts floating sample counts and rounds after validation"
)]
fn parse_frame_duration(effect: &'static str, frames: &str) -> CommandResult<SilenceDuration> {
    frames
        .parse::<f64>()
        .ok()
        .filter(|frames| frames.is_finite() && *frames >= 0.0)
        .and_then(|frames| {
            let rounded = (frames + 0.5).floor();
            (rounded <= u64::MAX as f64)
                .then(|| SilenceDuration::Frames(FrameCount::new(rounded as u64)))
        })
        .ok_or(EffectCommandParseError::InvalidEffectConfig {
            effect,
            argument: "silence",
            source: crate::EffectError::InvalidSilence,
        })
}

fn parse_seconds_duration(effect: &'static str, value: &str) -> CommandResult<SilenceDuration> {
    let seconds = if value.contains(':') {
        parse_colon_time(effect, value)?
    } else {
        value
            .parse::<f64>()
            .map_err(|source| EffectCommandParseError::InvalidNumber {
                effect,
                argument: "duration",
                value: value.to_owned(),
                source,
            })?
    };
    SilenceDuration::seconds(seconds).map_err(|source| {
        EffectCommandParseError::InvalidEffectConfig {
            effect,
            argument: "silence",
            source,
        }
    })
}

fn parse_colon_time(effect: &'static str, value: &str) -> CommandResult<f64> {
    let parts = value.split(':').collect::<Vec<_>>();
    if parts.len() > 3 || parts.iter().any(|part| part.is_empty()) {
        return invalid_silence(effect);
    }
    let mut seconds = 0.0;
    for part in parts {
        seconds = seconds * 60.0
            + part
                .parse::<f64>()
                .map_err(|source| EffectCommandParseError::InvalidNumber {
                    effect,
                    argument: "duration",
                    value: value.to_owned(),
                    source,
                })?;
    }
    if seconds.is_finite() && seconds >= 0.0 {
        Ok(seconds)
    } else {
        invalid_silence(effect)
    }
}

fn parse_threshold(effect: &'static str, value: &str) -> CommandResult<SilenceThreshold> {
    if let Some(db) = value.strip_suffix('d') {
        let db = db
            .parse::<f64>()
            .map_err(|source| EffectCommandParseError::InvalidNumber {
                effect,
                argument: "threshold",
                value: value.to_owned(),
                source,
            })?;
        return SilenceThreshold::decibels(db).map_err(|source| {
            EffectCommandParseError::InvalidEffectConfig {
                effect,
                argument: "silence",
                source,
            }
        });
    }
    let percent = value.strip_suffix('%').unwrap_or(value);
    let percent =
        percent
            .parse::<f64>()
            .map_err(|source| EffectCommandParseError::InvalidNumber {
                effect,
                argument: "threshold",
                value: value.to_owned(),
                source,
            })?;
    SilenceThreshold::percent(percent).map_err(|source| {
        EffectCommandParseError::InvalidEffectConfig {
            effect,
            argument: "silence",
            source,
        }
    })
}

fn render_duration(duration: SilenceDuration) -> String {
    match duration {
        SilenceDuration::Frames(frames) => format!("{}s", frames.as_u64()),
        SilenceDuration::Seconds(seconds) => render_f64(seconds),
    }
}

fn render_threshold(threshold: SilenceThreshold) -> String {
    match threshold {
        SilenceThreshold::Percent(percent) => format!("{}%", render_f64(percent)),
        SilenceThreshold::Decibels(db) => format!("{}d", render_f64(db)),
    }
}

fn invalid_silence<T>(effect: &'static str) -> CommandResult<T> {
    Err(EffectCommandParseError::InvalidEffectConfig {
        effect,
        argument: "silence",
        source: crate::EffectError::InvalidSilence,
    })
}

#[cfg(test)]
mod tests {
    use auralis_core::FrameCount;

    use super::parse_silence;
    use crate::{
        EffectCommand, EffectCommandParseError, EffectError, Silence, SilenceDuration,
        SilenceThreshold,
    };

    #[test]
    fn parses_copy_through_and_start_stop_forms() {
        assert_eq!(
            parse_silence("silence", &["0"]).unwrap(),
            EffectCommand::Silence(Silence::copy_through())
        );

        let parsed =
            parse_silence("silence", &["-l", "1", "2s", "0%", "-1", "0.5", "-40d"]).unwrap();
        assert_eq!(
            parsed.render_tokens(),
            ["silence", "-l", "1", "2s", "0%", "-1", "0.5", "-40d"]
        );
        assert_eq!(
            parsed,
            EffectCommand::Silence(
                Silence::new(
                    1,
                    Some((
                        SilenceDuration::frames(FrameCount::new(2)),
                        SilenceThreshold::percent(0.0).unwrap(),
                    )),
                    Some((
                        -1,
                        SilenceDuration::seconds(0.5).unwrap(),
                        SilenceThreshold::decibels(-40.0).unwrap(),
                    )),
                    true,
                )
                .unwrap()
            )
        );
    }

    #[test]
    fn rejects_invalid_forms() {
        assert!(matches!(
            parse_silence("silence", &[]).unwrap_err(),
            EffectCommandParseError::MissingArgument {
                argument: "above-periods",
                ..
            }
        ));
        assert_eq!(
            parse_silence("silence", &["-l", "0"]).unwrap_err(),
            EffectCommandParseError::InvalidEffectConfig {
                effect: "silence",
                argument: "silence",
                source: EffectError::InvalidSilence,
            }
        );
        assert_eq!(
            parse_silence("silence", &["1", "1s", "101%"]).unwrap_err(),
            EffectCommandParseError::InvalidEffectConfig {
                effect: "silence",
                argument: "silence",
                source: EffectError::InvalidSilence,
            }
        );
    }
}

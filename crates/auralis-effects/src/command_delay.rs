use auralis_core::FrameCount;

use crate::{
    Delay, DelayAmount, DelayAnchor, DelayPosition,
    command::{CommandResult, EffectCommand, EffectCommandParseError, render_f64},
};

pub(super) fn parse_delay(effect: &'static str, args: &[&str]) -> CommandResult<EffectCommand> {
    let positions = args
        .iter()
        .copied()
        .map(|arg| parse_delay_position(effect, arg))
        .collect::<CommandResult<Vec<_>>>()?;

    Ok(EffectCommand::Delay(Delay::with_positions(positions)))
}

pub(super) fn render_delay(delay: &Delay) -> Vec<String> {
    let mut tokens = vec!["delay".to_owned()];
    tokens.extend(delay.positions().iter().copied().map(render_delay_position));
    tokens
}

fn parse_delay_position(effect: &'static str, value: &str) -> CommandResult<DelayPosition> {
    let (anchor, amount_text) = match value.as_bytes().first().copied() {
        Some(b'=') => (DelayAnchor::Start, &value[1..]),
        Some(b'+') => (DelayAnchor::Previous, &value[1..]),
        Some(b'-') => (DelayAnchor::End, &value[1..]),
        Some(_) => (DelayAnchor::Start, value),
        None => {
            return Err(EffectCommandParseError::InvalidDelayPosition {
                effect,
                value: value.to_owned(),
            });
        }
    };
    let amount = parse_delay_amount(effect, value, amount_text)?;
    Ok(DelayPosition::new(anchor, amount))
}

fn parse_delay_amount(
    effect: &'static str,
    original: &str,
    amount_text: &str,
) -> CommandResult<DelayAmount> {
    if amount_text.is_empty() {
        return Err(EffectCommandParseError::InvalidDelayPosition {
            effect,
            value: original.to_owned(),
        });
    }
    if let Some(frames) = amount_text.strip_suffix('s') {
        return parse_frame_amount(effect, original, frames);
    }
    if let Some(seconds) = amount_text.strip_suffix('t') {
        return parse_seconds_amount(effect, original, seconds);
    }
    if amount_text.contains(':') {
        return parse_colon_time_amount(effect, original, amount_text);
    }
    parse_seconds_amount(effect, original, amount_text)
}

fn parse_frame_amount(
    effect: &'static str,
    original: &str,
    frames: &str,
) -> CommandResult<DelayAmount> {
    frames
        .parse::<u64>()
        .map(|frames| DelayAmount::Frames(FrameCount::new(frames)))
        .map_err(|_| EffectCommandParseError::InvalidDelayPosition {
            effect,
            value: original.to_owned(),
        })
}

fn parse_seconds_amount(
    effect: &'static str,
    original: &str,
    seconds: &str,
) -> CommandResult<DelayAmount> {
    let seconds =
        seconds
            .parse::<f64>()
            .map_err(|_| EffectCommandParseError::InvalidDelayPosition {
                effect,
                value: original.to_owned(),
            })?;
    if seconds.is_finite() && seconds >= 0.0 {
        Ok(DelayAmount::Seconds(seconds))
    } else {
        Err(EffectCommandParseError::InvalidDelayPosition {
            effect,
            value: original.to_owned(),
        })
    }
}

fn parse_colon_time_amount(
    effect: &'static str,
    original: &str,
    amount_text: &str,
) -> CommandResult<DelayAmount> {
    let parts = amount_text.split(':').collect::<Vec<_>>();
    if parts.len() > 3 || parts.iter().any(|part| part.is_empty()) {
        return Err(EffectCommandParseError::InvalidDelayPosition {
            effect,
            value: original.to_owned(),
        });
    }

    let mut seconds = 0.0;
    for part in parts {
        seconds = seconds * 60.0
            + part
                .parse::<f64>()
                .map_err(|_| EffectCommandParseError::InvalidDelayPosition {
                    effect,
                    value: original.to_owned(),
                })?;
    }
    if seconds.is_finite() && seconds >= 0.0 {
        Ok(DelayAmount::Seconds(seconds))
    } else {
        Err(EffectCommandParseError::InvalidDelayPosition {
            effect,
            value: original.to_owned(),
        })
    }
}

fn render_delay_position(position: DelayPosition) -> String {
    let anchor = match position.anchor() {
        DelayAnchor::Start => "",
        DelayAnchor::Previous => "+",
        DelayAnchor::End => "-",
    };
    let amount = match position.amount() {
        DelayAmount::Frames(frames) => format!("{}s", frames.as_u64()),
        DelayAmount::Seconds(seconds) => render_f64(seconds),
    };
    format!("{anchor}{amount}")
}

#[cfg(test)]
mod tests {
    use crate::{
        Delay, DelayAmount, DelayAnchor, DelayPosition, EffectCommand, EffectCommandParseError,
    };
    use auralis_core::FrameCount;

    use super::parse_delay;

    #[test]
    fn parses_frame_and_time_positions() {
        assert_eq!(
            parse_delay("delay", &["2s", "0.001"]).unwrap(),
            EffectCommand::Delay(Delay::with_positions([
                DelayPosition::frames(FrameCount::new(2)),
                DelayPosition::seconds(0.001).unwrap(),
            ]))
        );
    }

    #[test]
    fn parses_anchors_and_colon_time() {
        assert_eq!(
            parse_delay("delay", &["=1s", "+2s", "-0:00.003"]).unwrap(),
            EffectCommand::Delay(Delay::with_positions([
                DelayPosition::new(DelayAnchor::Start, DelayAmount::Frames(FrameCount::new(1))),
                DelayPosition::new(
                    DelayAnchor::Previous,
                    DelayAmount::Frames(FrameCount::new(2)),
                ),
                DelayPosition::new(DelayAnchor::End, DelayAmount::Seconds(0.003)),
            ]))
        );
    }

    #[test]
    fn rejects_invalid_positions() {
        assert!(matches!(
            parse_delay("delay", &["not-a-position"]).unwrap_err(),
            EffectCommandParseError::InvalidDelayPosition { .. }
        ));
    }
}

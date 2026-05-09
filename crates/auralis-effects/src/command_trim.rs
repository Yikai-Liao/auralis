use auralis_core::FrameCount;

use crate::{
    Trim, TrimPosition,
    command::{CommandResult, EffectCommand, EffectCommandParseError},
};

pub(super) fn parse_trim(effect: &'static str, args: &[&str]) -> CommandResult<EffectCommand> {
    if args.is_empty() {
        return Err(EffectCommandParseError::MissingArgument {
            effect,
            argument: "position",
        });
    }

    let positions = args
        .iter()
        .copied()
        .enumerate()
        .map(|(index, arg)| parse_trim_position(effect, index, arg))
        .collect::<CommandResult<Vec<_>>>()?;

    Trim::with_positions(positions)
        .map(EffectCommand::Trim)
        .map_err(|source| EffectCommandParseError::InvalidEffectConfig {
            effect,
            argument: "position",
            source,
        })
}

pub(super) fn render_trim(trim: &Trim) -> Vec<String> {
    let mut tokens = vec!["trim".to_owned()];
    for (index, position) in trim.positions().iter().copied().enumerate() {
        tokens.push(render_trim_position(index, position));
    }
    tokens
}

fn parse_trim_position(
    effect: &'static str,
    index: usize,
    value: &str,
) -> CommandResult<TrimPosition> {
    let (anchor, frame_text) = match value.as_bytes().first().copied() {
        Some(b'=' | b'+' | b'-') => (value.as_bytes()[0] as char, &value[1..]),
        Some(_) | None => ('+', value),
    };

    let frame = parse_trim_frame_count(effect, value, frame_text)?;
    Ok(match anchor {
        '=' => TrimPosition::Absolute(frame),
        '-' if frame.as_u64() == 0 => TrimPosition::End,
        '-' => TrimPosition::BeforeEnd(frame),
        '+' if index == 0 => TrimPosition::Absolute(frame),
        '+' => TrimPosition::Relative(frame),
        _ => unreachable!("trim anchor is constrained above"),
    })
}

fn parse_trim_frame_count(
    effect: &'static str,
    original: &str,
    frame_text: &str,
) -> CommandResult<FrameCount> {
    let frame_text = frame_text.strip_suffix('s').unwrap_or(frame_text);
    frame_text
        .parse::<u64>()
        .map(FrameCount::new)
        .map_err(|source| EffectCommandParseError::InvalidFrameCount {
            effect,
            argument: "position",
            value: original.to_owned(),
            source,
        })
}

fn render_trim_position(index: usize, position: TrimPosition) -> String {
    match position {
        TrimPosition::Absolute(frame) if index == 0 => frame.as_u64().to_string(),
        TrimPosition::Absolute(frame) => format!("={}", frame.as_u64()),
        TrimPosition::Relative(frames) => frames.as_u64().to_string(),
        TrimPosition::BeforeEnd(frames) => format!("-{}", frames.as_u64()),
        TrimPosition::End => "-0".to_owned(),
    }
}

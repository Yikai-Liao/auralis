use auralis_core::FrameCount;

use crate::{
    command::{CommandResult, EffectCommand, EffectCommandParseError},
    Trim, TrimPosition,
};

pub(super) fn parse_trim(effect: &'static str, args: &[&str]) -> CommandResult<EffectCommand> {
    if args.is_empty() {
        return Err(EffectCommandParseError::MissingArgument {
            effect,
            argument: "position",
        });
    }

    let range_args;
    let args = if args.len() == 1 {
        if let Some(expanded) = expand_range_arg(args[0]) {
            range_args = expanded;
            range_args.iter().map(String::as_str).collect::<Vec<_>>()
        } else {
            args.to_vec()
        }
    } else {
        args.to_vec()
    };

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

fn expand_range_arg(value: &str) -> Option<Vec<String>> {
    let (start, end) = value.split_once("..")?;
    let mut args = Vec::new();

    args.push(if start.is_empty() {
        "0".to_owned()
    } else {
        start.to_owned()
    });
    args.push(if end.is_empty() {
        "-0".to_owned()
    } else {
        format!("={end}")
    });

    Some(args)
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
    let frame_text = frame_text
        .strip_suffix('s')
        .or_else(|| frame_text.strip_suffix('f'))
        .unwrap_or(frame_text);
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

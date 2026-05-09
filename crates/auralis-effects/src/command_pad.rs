use auralis_core::FrameCount;

use crate::{
    Pad, PositionedPad,
    command::{CommandResult, EffectCommand, EffectCommandParseError, parse_frame_count},
};

pub(super) fn parse_pad(effect: &'static str, args: &[&str]) -> CommandResult<EffectCommand> {
    let mut start = FrameCount::new(0);
    let mut end = FrameCount::new(0);
    let mut positioned = Vec::new();
    let mut saw_end = false;

    for (index, arg) in args.iter().copied().enumerate() {
        let parsed = parse_pad_arg(effect, index, arg)?;
        match parsed {
            ParsedPadArg::Start(length) => start = length,
            ParsedPadArg::End(length) => {
                if saw_end {
                    return Err(EffectCommandParseError::InvalidOptionCombination {
                        effect,
                        options: "only one unpositioned end pad may be given",
                    });
                }
                end = length;
                saw_end = true;
            }
            ParsedPadArg::Positioned(pad) => {
                if saw_end {
                    return Err(EffectCommandParseError::InvalidOptionCombination {
                        effect,
                        options: "positioned padding cannot follow end padding",
                    });
                }
                positioned.push(pad);
            }
        }
    }

    Pad::with_positioned(start, end, positioned)
        .map(EffectCommand::Pad)
        .map_err(|source| EffectCommandParseError::InvalidEffectConfig {
            effect,
            argument: "pad-position",
            source,
        })
}

pub(super) fn render_pad(pad: &Pad) -> Vec<String> {
    if pad.positioned().is_empty() {
        return vec![
            "pad".to_owned(),
            pad.start.as_u64().to_string(),
            pad.end.as_u64().to_string(),
        ];
    }

    let mut tokens = vec!["pad".to_owned()];
    if pad.start.as_u64() != 0 {
        tokens.push(pad.start.as_u64().to_string());
    }
    for positioned in pad.positioned() {
        tokens.push(format!(
            "{}@{}",
            positioned.length.as_u64(),
            positioned.position.as_u64()
        ));
    }
    if pad.end.as_u64() != 0 {
        tokens.push(pad.end.as_u64().to_string());
    }
    tokens
}

enum ParsedPadArg {
    Start(FrameCount),
    End(FrameCount),
    Positioned(PositionedPad),
}

fn parse_pad_arg(effect: &'static str, index: usize, value: &str) -> CommandResult<ParsedPadArg> {
    if value.starts_with('%') {
        return Err(EffectCommandParseError::UnsupportedOption {
            effect,
            option: "%".to_owned(),
        });
    }

    let Some((length, position)) = value.split_once('@') else {
        let length = parse_frame_count(
            effect,
            if index == 0 {
                "start-frame"
            } else {
                "end-frame"
            },
            value,
        )?;
        return Ok(if index == 0 {
            ParsedPadArg::Start(length)
        } else {
            ParsedPadArg::End(length)
        });
    };

    let length = parse_frame_count(effect, "pad-frame", length)?;
    if position == "-0" {
        return Ok(ParsedPadArg::End(length));
    }

    let position = parse_frame_count(effect, "pad-position", position)?;
    Ok(ParsedPadArg::Positioned(PositionedPad::new(
        length, position,
    )))
}

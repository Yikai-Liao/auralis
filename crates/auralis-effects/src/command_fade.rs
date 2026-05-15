use auralis_core::FrameCount;

use crate::command::{
    CommandResult, EffectCommand, EffectCommandParseError, parse_frame_count,
    reject_extra_arguments,
};
use crate::{Fade, FadeCurve};

pub(super) fn parse_fade(effect: &'static str, args: &[&str]) -> CommandResult<EffectCommand> {
    let named_args;
    let args = if args.iter().any(|arg| arg.contains('=')) {
        named_args = named_fade_args(effect, args)?;
        named_args.iter().map(String::as_str).collect::<Vec<_>>()
    } else {
        args.to_vec()
    };

    let (curve, args) = match args.first().and_then(|token| FadeCurve::from_token(token)) {
        Some(curve) => (curve, &args[1..]),
        None => (FadeCurve::Logarithmic, args.as_slice()),
    };

    let fade_in = crate::command::required_arg(effect, args, "fade-in-frame")?;
    let fade_in = parse_fade_frame_count(effect, "fade-in-frame", fade_in)?;
    let fade = if let Some(stop_position) = args.get(1).copied() {
        let stop_position = parse_fade_stop_position(effect, stop_position)?;
        let fade_out = match args.get(2).copied() {
            Some(fade_out) => parse_fade_frame_count(effect, "fade-out-frame", fade_out)?,
            None => fade_in,
        };
        reject_extra_arguments(effect, args.get(3..).unwrap_or_default())?;
        Fade::with_stop_position(curve, fade_in, stop_position, fade_out)
    } else {
        reject_extra_arguments(effect, args.get(1..).unwrap_or_default())?;
        Fade::with_curve(curve, fade_in, FrameCount::new(0))
    };

    Ok(EffectCommand::Fade(fade))
}

pub(super) fn render_fade(fade: Fade) -> Vec<String> {
    let mut tokens = vec![
        "fade".to_owned(),
        fade.curve.token().to_owned(),
        fade.fade_in.as_u64().to_string(),
    ];

    match fade.stop_position {
        Some(stop_position) => {
            tokens.push(stop_position.as_u64().to_string());
            tokens.push(fade.fade_out.as_u64().to_string());
        }
        None if fade.fade_out.as_u64() != 0 => {
            tokens.push("0".to_owned());
            tokens.push(fade.fade_out.as_u64().to_string());
        }
        None => {}
    }

    tokens
}

fn parse_fade_stop_position(effect: &'static str, value: &str) -> CommandResult<FrameCount> {
    if value == "-0" {
        return Ok(FrameCount::new(0));
    }

    parse_fade_frame_count(effect, "stop-position", value)
}

fn parse_fade_frame_count(
    effect: &'static str,
    argument: &'static str,
    value: &str,
) -> CommandResult<FrameCount> {
    parse_frame_count(effect, argument, value.strip_suffix('f').unwrap_or(value))
}

fn named_fade_args(effect: &'static str, args: &[&str]) -> CommandResult<Vec<String>> {
    let mut fade_in = None;
    let mut fade_out = None;
    let mut curve = None;

    for arg in args {
        let Some((name, value)) = arg.split_once('=') else {
            return Err(EffectCommandParseError::UnexpectedArgument {
                effect,
                argument: (*arg).to_owned(),
            });
        };
        match name {
            "in" | "fade_in" => fade_in = Some(value.to_owned()),
            "out" | "fade_out" => fade_out = Some(value.to_owned()),
            "curve" => curve = Some(named_fade_curve(effect, arg, value)?),
            _ => {
                return Err(EffectCommandParseError::UnexpectedArgument {
                    effect,
                    argument: (*arg).to_owned(),
                });
            }
        }
    }

    if fade_in.is_none() && fade_out.is_none() {
        return Err(EffectCommandParseError::MissingArgument {
            effect,
            argument: "in",
        });
    }
    let fade_in = fade_in.unwrap_or_else(|| "0".to_owned());
    let mut parsed = Vec::new();
    if let Some(curve) = curve {
        parsed.push(curve);
    }
    parsed.push(fade_in);
    if let Some(fade_out) = fade_out {
        parsed.push("0".to_owned());
        parsed.push(fade_out);
    }
    Ok(parsed)
}

fn named_fade_curve(effect: &'static str, original: &str, value: &str) -> CommandResult<String> {
    match value {
        "quarter-sine" | "quarter_sine" | "q" => Ok("q".to_owned()),
        "half-sine" | "half_sine" | "h" => Ok("h".to_owned()),
        "log" | "logarithmic" | "l" => Ok("l".to_owned()),
        "linear" | "t" => Ok("t".to_owned()),
        "parabola" | "inverted-parabola" | "inverted_parabola" | "p" => Ok("p".to_owned()),
        _ => Err(EffectCommandParseError::UnexpectedArgument {
            effect,
            argument: original.to_owned(),
        }),
    }
}

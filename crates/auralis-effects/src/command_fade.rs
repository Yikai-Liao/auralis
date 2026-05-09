use auralis_core::FrameCount;

use crate::command::{CommandResult, EffectCommand, parse_frame_count, reject_extra_arguments};
use crate::{Fade, FadeCurve};

pub(super) fn parse_fade(effect: &'static str, args: &[&str]) -> CommandResult<EffectCommand> {
    let (curve, args) = match args.first().and_then(|token| FadeCurve::from_token(token)) {
        Some(curve) => (curve, &args[1..]),
        None => (FadeCurve::Logarithmic, args),
    };

    let fade_in = crate::command::required_arg(effect, args, "fade-in-frame")?;
    let fade_in = parse_frame_count(effect, "fade-in-frame", fade_in)?;
    let fade = if let Some(stop_position) = args.get(1).copied() {
        let stop_position = parse_fade_stop_position(effect, stop_position)?;
        let fade_out = match args.get(2).copied() {
            Some(fade_out) => parse_frame_count(effect, "fade-out-frame", fade_out)?,
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

    parse_frame_count(effect, "stop-position", value)
}

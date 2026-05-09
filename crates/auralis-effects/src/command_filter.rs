use crate::BiquadWidth;
use crate::command::{
    CommandResult, EffectCommandParseError, is_option_like, parse_f64, render_f64,
};

pub(super) fn parse_frequency_hz(effect: &'static str, value: &str) -> CommandResult<f64> {
    if let Some(kilohertz) = value.strip_suffix(['k', 'K']) {
        Ok(parse_f64(effect, "frequency", kilohertz)? * 1000.0)
    } else {
        parse_f64(effect, "frequency", value)
    }
}

pub(super) fn parse_width(effect: &'static str, value: &str) -> CommandResult<BiquadWidth> {
    if is_option_like(value) {
        return Err(EffectCommandParseError::UnsupportedOption {
            effect,
            option: value.to_owned(),
        });
    }

    let (number, suffix) = split_width_suffix(value);
    let width = parse_f64(effect, "width", number)?;

    match suffix.unwrap_or('h') {
        'h' => Ok(BiquadWidth::hertz(width)),
        'k' => Ok(BiquadWidth::kilohertz(width)),
        'q' => Ok(BiquadWidth::q(width)),
        'o' => Ok(BiquadWidth::octaves(width)),
        _ => Err(EffectCommandParseError::InvalidEffectConfig {
            effect,
            argument: "width",
            source: crate::EffectError::InvalidBiquadDesign,
        }),
    }
}

pub(super) fn render_width(width: BiquadWidth) -> String {
    match width {
        BiquadWidth::Hertz(value) => format!("{}h", render_f64(value)),
        BiquadWidth::Kilohertz(value) => format!("{}k", render_f64(value)),
        BiquadWidth::Q(value) => format!("{}q", render_f64(value)),
        BiquadWidth::Octaves(value) => format!("{}o", render_f64(value)),
        BiquadWidth::Slope(value) => format!("{}s", render_f64(value)),
    }
}

fn split_width_suffix(value: &str) -> (&str, Option<char>) {
    let Some(suffix) = value.chars().last().filter(char::is_ascii_alphabetic) else {
        return (value, None);
    };

    (&value[..value.len() - suffix.len_utf8()], Some(suffix))
}

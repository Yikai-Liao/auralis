use crate::DcShift;
use crate::command::{
    CommandResult, EffectCommand, EffectCommandParseError, parse_f32, reject_extra_arguments,
    render_f32, required_arg,
};

pub(super) fn parse_dc_shift(effect: &'static str, args: &[&str]) -> CommandResult<EffectCommand> {
    let shift = required_arg(effect, args, "shift")?;
    let shift = parse_f32(effect, "shift", shift)?;
    let limiter_gain = args
        .get(1)
        .map(|value| parse_f32(effect, "limiter-gain", value))
        .transpose()?;
    reject_extra_arguments(effect, args.get(2..).unwrap_or_default())?;

    let dc_shift = match limiter_gain {
        Some(limiter_gain) => DcShift::with_limiter_gain(shift, limiter_gain),
        None => DcShift::new(shift),
    };
    dc_shift.map(EffectCommand::DcShift).map_err(|source| {
        EffectCommandParseError::InvalidEffectConfig {
            effect,
            argument: if limiter_gain.is_some() {
                "limiter-gain"
            } else {
                "shift"
            },
            source,
        }
    })
}

pub(super) fn render_dc_shift(dc_shift: DcShift) -> Vec<String> {
    let mut tokens = vec!["dcshift".to_owned(), render_f32(dc_shift.shift)];
    if let Some(limiter_gain) = dc_shift.limiter_gain {
        tokens.push(render_f32(limiter_gain));
    }
    tokens
}

use crate::Reverse;
use crate::command::{CommandResult, EffectCommand, reject_extra_arguments};

pub(super) fn parse_reverse(effect: &'static str, args: &[&str]) -> CommandResult<EffectCommand> {
    reject_extra_arguments(effect, args)?;

    Ok(EffectCommand::Reverse(Reverse::new()))
}

use crate::{
    EffectCommand, FirFit,
    command::{CommandResult, EffectCommandParseError},
};

pub(crate) fn parse_firfit(effect: &'static str, args: &[&str]) -> CommandResult<EffectCommand> {
    FirFit::parse_sox_args(args)
        .map(EffectCommand::FirFit)
        .map_err(|source| EffectCommandParseError::InvalidEffectConfig {
            effect,
            argument: "knots",
            source,
        })
}

pub(crate) fn render_firfit(firfit: &FirFit) -> Vec<String> {
    firfit.render_tokens()
}

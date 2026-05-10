use crate::{
    EffectCommand, Fir,
    command::{CommandResult, EffectCommandParseError},
};

pub(crate) fn parse_fir(effect: &'static str, args: &[&str]) -> CommandResult<EffectCommand> {
    Fir::parse_sox_args(args)
        .map(EffectCommand::Fir)
        .map_err(|source| EffectCommandParseError::InvalidEffectConfig {
            effect,
            argument: "coefficients",
            source,
        })
}

pub(crate) fn render_fir(fir: &Fir) -> Vec<String> {
    fir.render_tokens()
}

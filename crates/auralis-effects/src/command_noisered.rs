use crate::{
    NoiseRed,
    command::{CommandResult, EffectCommand, EffectCommandParseError, parse_f64, render_f64},
};

pub(super) fn parse_noisered(effect: &'static str, args: &[&str]) -> CommandResult<EffectCommand> {
    let (profile_path, amount) = match args {
        [] => ("-", NoiseRed::DEFAULT_AMOUNT),
        [path] => (*path, NoiseRed::DEFAULT_AMOUNT),
        [path, amount, ..] => (*path, parse_f64(effect, "amount", amount)?),
    };

    if let Some(argument) = args.get(2) {
        return Err(if crate::command::is_option_like(argument) {
            EffectCommandParseError::UnsupportedOption {
                effect,
                option: (*argument).to_owned(),
            }
        } else {
            EffectCommandParseError::UnexpectedArgument {
                effect,
                argument: (*argument).to_owned(),
            }
        });
    }
    if crate::command::is_option_like(profile_path) && profile_path != "-" {
        return Err(EffectCommandParseError::UnsupportedOption {
            effect,
            option: profile_path.to_owned(),
        });
    }

    NoiseRed::from_profile_path(
        (profile_path != "-").then(|| profile_path.to_owned()),
        amount,
    )
    .map(EffectCommand::NoiseRed)
    .map_err(|source| EffectCommandParseError::InvalidEffectConfig {
        effect,
        argument: "profile-file",
        source,
    })
}

pub(super) fn render_noisered(noisered: &NoiseRed) -> Vec<String> {
    vec![
        "noisered".to_owned(),
        noisered.profile_path().unwrap_or("-").to_owned(),
        render_f64(noisered.amount()),
    ]
}

#[cfg(test)]
mod tests {
    use super::parse_noisered;
    use crate::{EffectCommand, EffectCommandParseError, NoiseRed};

    #[test]
    fn parses_default_path_and_amount() {
        assert_eq!(
            parse_noisered("noisered", &[]).unwrap(),
            EffectCommand::NoiseRed(NoiseRed::from_profile_path(None, 0.5).unwrap())
        );
        assert_eq!(
            parse_noisered("noisered", &["profile.prof"])
                .unwrap()
                .render_tokens(),
            ["noisered", "profile.prof", "0.5"]
        );
        assert_eq!(
            parse_noisered("noisered", &["profile.prof", "0.25"])
                .unwrap()
                .render_tokens(),
            ["noisered", "profile.prof", "0.25"]
        );
    }

    #[test]
    fn rejects_invalid_amount_and_extra_arguments() {
        assert!(matches!(
            parse_noisered("noisered", &["profile.prof", "2"]).unwrap_err(),
            EffectCommandParseError::InvalidEffectConfig { .. }
        ));
        assert_eq!(
            parse_noisered("noisered", &["profile.prof", "0.5", "extra"]).unwrap_err(),
            EffectCommandParseError::UnexpectedArgument {
                effect: "noisered",
                argument: "extra".to_owned(),
            }
        );
    }
}

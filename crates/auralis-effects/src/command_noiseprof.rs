use crate::NoiseProf;
use crate::command::{CommandResult, EffectCommand, EffectCommandParseError, is_option_like};

pub(super) fn parse_noiseprof(effect: &'static str, args: &[&str]) -> CommandResult<EffectCommand> {
    match args {
        [] => Ok(EffectCommand::NoiseProf(NoiseProf::stdout())),
        [path] if is_option_like(path) && *path != "-" => {
            Err(EffectCommandParseError::UnsupportedOption {
                effect,
                option: (*path).to_owned(),
            })
        }
        [path] => NoiseProf::new((*path != "-").then(|| (*path).to_owned()))
            .map(EffectCommand::NoiseProf)
            .map_err(|source| EffectCommandParseError::InvalidEffectConfig {
                effect,
                argument: "profile-file",
                source,
            }),
        [_, extra, ..] if is_option_like(extra) => {
            Err(EffectCommandParseError::UnsupportedOption {
                effect,
                option: (*extra).to_owned(),
            })
        }
        [_, extra, ..] => Err(EffectCommandParseError::UnexpectedArgument {
            effect,
            argument: (*extra).to_owned(),
        }),
    }
}

pub(super) fn render_noiseprof(noiseprof: &NoiseProf) -> Vec<String> {
    vec![
        "noiseprof".to_owned(),
        noiseprof.output_path().unwrap_or("-").to_owned(),
    ]
}

#[cfg(test)]
mod tests {
    use super::parse_noiseprof;
    use crate::{EffectCommand, EffectCommandParseError, NoiseProf};

    #[test]
    fn parses_default_stdout_and_explicit_path() {
        assert_eq!(
            parse_noiseprof("noiseprof", &[]).unwrap(),
            EffectCommand::NoiseProf(NoiseProf::stdout())
        );
        assert_eq!(
            parse_noiseprof("noiseprof", &["profile.prof"])
                .unwrap()
                .render_tokens(),
            ["noiseprof", "profile.prof"]
        );
        assert_eq!(
            parse_noiseprof("noiseprof", &["-"])
                .unwrap()
                .render_tokens(),
            ["noiseprof", "-"]
        );
    }

    #[test]
    fn rejects_options_and_extra_arguments() {
        assert_eq!(
            parse_noiseprof("noiseprof", &["-q"]).unwrap_err(),
            EffectCommandParseError::UnsupportedOption {
                effect: "noiseprof",
                option: "-q".to_owned(),
            }
        );
        assert_eq!(
            parse_noiseprof("noiseprof", &["profile.prof", "extra"]).unwrap_err(),
            EffectCommandParseError::UnexpectedArgument {
                effect: "noiseprof",
                argument: "extra".to_owned(),
            }
        );
    }
}

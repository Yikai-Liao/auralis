use crate::{
    Dither, DitherMode, DitherNoiseShape,
    command::{CommandResult, EffectCommand, EffectCommandParseError},
};

pub(super) fn parse_dither(effect: &'static str, args: &[&str]) -> CommandResult<EffectCommand> {
    let mut dither = Dither::new();
    let mut index = 0;

    while index < args.len() {
        match args[index] {
            "-S" => {
                let mut updated = Dither::sloped_tpdf()
                    .with_precision(dither.precision_bits())
                    .expect("existing precision remains valid")
                    .with_seed(dither.seed());
                if let Some(noise_shape) = dither.noise_shape() {
                    updated = updated.with_noise_shape(noise_shape);
                }
                dither = updated;
                index += 1;
            }
            "-s" => {
                dither = with_noise_shape(dither, DitherNoiseShape::Shibata);
                index += 1;
            }
            "-f" => {
                let Some(shape) = args.get(index + 1) else {
                    return Err(EffectCommandParseError::MissingArgument {
                        effect,
                        argument: "filter",
                    });
                };
                dither = with_noise_shape(dither, parse_noise_shape(effect, shape)?);
                index += 2;
            }
            "-p" => {
                let Some(bits) = args.get(index + 1) else {
                    return Err(EffectCommandParseError::MissingArgument {
                        effect,
                        argument: "precision",
                    });
                };
                dither = with_precision(effect, dither, parse_precision(effect, bits)?)?;
                index += 2;
            }
            option if option.starts_with("-p") && option.len() > 2 => {
                dither = with_precision(effect, dither, parse_precision(effect, &option[2..])?)?;
                index += 1;
            }
            "-a" => {
                return Err(EffectCommandParseError::UnsupportedOption {
                    effect,
                    option: args[index].to_owned(),
                });
            }
            option if crate::command::is_option_like(option) => {
                return Err(EffectCommandParseError::UnsupportedOption {
                    effect,
                    option: option.to_owned(),
                });
            }
            argument => {
                return Err(EffectCommandParseError::UnexpectedArgument {
                    effect,
                    argument: argument.to_owned(),
                });
            }
        }
    }

    Ok(EffectCommand::Dither(dither))
}

pub(super) fn render_dither(dither: Dither) -> Vec<String> {
    let mut tokens = vec!["dither".to_owned()];
    match dither.noise_shape() {
        Some(_) => tokens.push("-s".to_owned()),
        None if dither.mode() == DitherMode::SlopedTpdf => tokens.push("-S".to_owned()),
        None => {}
    }
    tokens.push("-p".to_owned());
    tokens.push(dither.precision_bits().to_string());
    tokens
}

fn parse_precision(effect: &'static str, value: &str) -> CommandResult<u8> {
    let precision =
        value
            .parse::<u8>()
            .map_err(|source| EffectCommandParseError::InvalidFrameCount {
                effect,
                argument: "precision",
                value: value.to_owned(),
                source,
            })?;

    with_precision(effect, Dither::new(), precision).map(|_| precision)
}

fn with_precision(effect: &'static str, dither: Dither, precision: u8) -> CommandResult<Dither> {
    dither.with_precision(precision).map_err(|source| {
        EffectCommandParseError::InvalidEffectConfig {
            effect,
            argument: "precision",
            source,
        }
    })
}

fn parse_noise_shape(effect: &'static str, value: &str) -> CommandResult<DitherNoiseShape> {
    match value {
        "shibata" => Ok(DitherNoiseShape::Shibata),
        other => Err(EffectCommandParseError::UnsupportedOption {
            effect,
            option: format!("-f {other}"),
        }),
    }
}

fn with_noise_shape(dither: Dither, noise_shape: DitherNoiseShape) -> Dither {
    dither
        .with_noise_shape(noise_shape)
        .with_precision(dither.precision_bits())
        .expect("existing precision remains valid")
        .with_seed(dither.seed())
}

#[cfg(test)]
mod tests {
    use super::parse_dither;
    use crate::{Dither, DitherNoiseShape, EffectCommand, EffectCommandParseError, EffectError};

    #[test]
    fn parses_plain_and_sloped_tpdf() {
        assert_eq!(
            parse_dither("dither", &[]).unwrap(),
            EffectCommand::Dither(Dither::new())
        );
        assert_eq!(
            parse_dither("dither", &["-S", "-p", "8"])
                .unwrap()
                .render_tokens(),
            ["dither", "-S", "-p", "8"]
        );
        assert_eq!(
            parse_dither("dither", &["-p12"]).unwrap().render_tokens(),
            ["dither", "-p", "12"]
        );
    }

    #[test]
    fn parses_shibata_noise_shaping() {
        assert_eq!(
            parse_dither("dither", &["-s", "-p", "8"])
                .unwrap()
                .render_tokens(),
            ["dither", "-s", "-p", "8"]
        );
        assert_eq!(
            parse_dither("dither", &["-f", "shibata"]).unwrap(),
            EffectCommand::Dither(Dither::new().with_noise_shape(DitherNoiseShape::Shibata))
        );
        assert_eq!(
            parse_dither("dither", &["-s", "-S"])
                .unwrap()
                .render_tokens(),
            ["dither", "-s", "-p", "16"]
        );
    }

    #[test]
    fn rejects_auto_detect_unsupported_shapes_and_bad_precision() {
        assert_eq!(
            parse_dither("dither", &["-f"]).unwrap_err(),
            EffectCommandParseError::MissingArgument {
                effect: "dither",
                argument: "filter",
            }
        );
        assert_eq!(
            parse_dither("dither", &["-f", "gesemann"]).unwrap_err(),
            EffectCommandParseError::UnsupportedOption {
                effect: "dither",
                option: "-f gesemann".to_owned(),
            }
        );
        assert_eq!(
            parse_dither("dither", &["-a"]).unwrap_err(),
            EffectCommandParseError::UnsupportedOption {
                effect: "dither",
                option: "-a".to_owned(),
            }
        );
        assert!(matches!(
            parse_dither("dither", &["-p", "1"]).unwrap_err(),
            EffectCommandParseError::InvalidEffectConfig {
                source: EffectError::InvalidDither,
                ..
            }
        ));
        assert_eq!(
            parse_dither("dither", &["extra"]).unwrap_err(),
            EffectCommandParseError::UnexpectedArgument {
                effect: "dither",
                argument: "extra".to_owned(),
            }
        );
    }
}

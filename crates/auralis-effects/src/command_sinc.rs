use crate::command::{
    CommandResult, EffectCommand, EffectCommandParseError, parse_f64, render_f64,
};
use crate::command_filter::parse_frequency_hz;
use crate::{EffectError, Sinc, SincBand, SincOptions};

const DEFAULT_SINC_ATTENUATION_DB: f64 = 120.0;

pub(super) fn parse_sinc(effect: &'static str, args: &[&str]) -> CommandResult<EffectCommand> {
    let mut options = SincOptions::default();
    let mut band = None;
    let mut delete_at_nyquist = false;
    let mut attenuation_set = false;
    let mut index = 0;

    while index < args.len() {
        let arg = args[index];
        match arg {
            "-r" => {
                options.round_taps = true;
                index += 1;
            }
            "-d" => {
                delete_at_nyquist = true;
                index += 1;
            }
            "-a" => {
                options.attenuation_db =
                    parse_option_f64(effect, args, &mut index, "-a", "attenuation")?;
                attenuation_set = true;
            }
            "-b" => {
                let beta = parse_option_f64(effect, args, &mut index, "-b", "beta")?;
                options.beta = Some(beta);
            }
            "-t" => {
                let width =
                    parse_option_frequency(effect, args, &mut index, "-t", "transition-width")?;
                options.transition_width_hz = Some(width);
            }
            "-n" => {
                let taps = parse_option_u32(effect, args, &mut index, "-n", "taps")?;
                options.taps = Some(taps);
            }
            "-M" | "-I" | "-L" => {
                return Err(EffectCommandParseError::UnsupportedOption {
                    effect,
                    option: arg.to_owned(),
                });
            }
            _ if arg.starts_with("-a") && arg.len() > 2 => {
                options.attenuation_db = parse_inline_f64(effect, arg, "-a", "attenuation")?;
                attenuation_set = true;
                index += 1;
            }
            _ if arg.starts_with("-b") && arg.len() > 2 => {
                options.beta = Some(parse_inline_f64(effect, arg, "-b", "beta")?);
                index += 1;
            }
            _ if arg.starts_with("-t") && arg.len() > 2 => {
                options.transition_width_hz = Some(parse_inline_frequency(
                    effect,
                    arg,
                    "-t",
                    "transition-width",
                )?);
                index += 1;
            }
            _ if arg.starts_with("-n") && arg.len() > 2 => {
                options.taps = Some(parse_inline_u32(effect, arg, "-n", "taps")?);
                index += 1;
            }
            _ if band.is_none() => {
                band = Some(parse_sinc_band(effect, arg)?);
                index += 1;
            }
            _ => {
                return Err(EffectCommandParseError::UnexpectedArgument {
                    effect,
                    argument: arg.to_owned(),
                });
            }
        }
    }

    let band = apply_delete_option(
        effect,
        band.ok_or(EffectCommandParseError::MissingArgument {
            effect,
            argument: "frequency-range",
        })?,
        delete_at_nyquist,
    )?;
    validate_option_combination(effect, options, attenuation_set)?;

    Sinc::with_options(band, options)
        .map(EffectCommand::Sinc)
        .map_err(|source| EffectCommandParseError::InvalidEffectConfig {
            effect,
            argument: "frequency-range",
            source,
        })
}

fn apply_delete_option(
    effect: &'static str,
    band: SincBand,
    delete_at_nyquist: bool,
) -> CommandResult<SincBand> {
    if !delete_at_nyquist {
        return Ok(band);
    }
    match band {
        SincBand::LowPass { frequency_hz, .. } => Ok(SincBand::low_pass(frequency_hz, true)),
        SincBand::HighPass { .. } => Err(EffectCommandParseError::UnsupportedOption {
            effect,
            option: "-d".to_owned(),
        }),
    }
}

fn validate_option_combination(
    effect: &'static str,
    options: SincOptions,
    attenuation_set: bool,
) -> CommandResult<()> {
    if options.beta.is_some() && attenuation_set {
        return Err(EffectCommandParseError::InvalidEffectConfig {
            effect,
            argument: "attenuation",
            source: EffectError::InvalidSinc,
        });
    }
    options
        .validate()
        .map_err(|source| EffectCommandParseError::InvalidEffectConfig {
            effect,
            argument: "options",
            source,
        })
}

pub(super) fn render_sinc(sinc: Sinc) -> Vec<String> {
    let mut tokens = vec!["sinc".to_owned()];
    let options = sinc.options;

    if let Some(beta) = options.beta {
        tokens.push("-b".to_owned());
        tokens.push(render_f64(beta));
    } else if options.attenuation_db.to_bits() != DEFAULT_SINC_ATTENUATION_DB.to_bits() {
        tokens.push("-a".to_owned());
        tokens.push(render_f64(options.attenuation_db));
    }
    if let Some(width) = options.transition_width_hz {
        tokens.push("-t".to_owned());
        tokens.push(render_f64(width));
    }
    if let Some(taps) = options.taps {
        tokens.push("-n".to_owned());
        tokens.push(taps.to_string());
    }
    if options.round_taps {
        tokens.push("-r".to_owned());
    }

    match sinc.band {
        SincBand::LowPass {
            frequency_hz,
            delete_at_nyquist,
        } => {
            tokens.push(format!("-{}", render_f64(frequency_hz)));
            if delete_at_nyquist {
                tokens.push("-d".to_owned());
            }
        }
        SincBand::HighPass { frequency_hz } => tokens.push(render_f64(frequency_hz)),
    }
    tokens
}

fn parse_sinc_band(effect: &'static str, arg: &str) -> CommandResult<SincBand> {
    if let Some(low_pass) = arg.strip_prefix('-') {
        if low_pass.is_empty() || !starts_frequency(low_pass) {
            return Err(EffectCommandParseError::UnsupportedOption {
                effect,
                option: arg.to_owned(),
            });
        }
        return Ok(SincBand::low_pass(
            parse_frequency_hz(effect, low_pass)?,
            false,
        ));
    }

    if arg.contains('-') {
        return Err(EffectCommandParseError::InvalidEffectConfig {
            effect,
            argument: "frequency-range",
            source: EffectError::InvalidSinc,
        });
    }

    Ok(SincBand::HighPass {
        frequency_hz: parse_frequency_hz(effect, arg)?,
    })
}

fn parse_option_f64(
    effect: &'static str,
    args: &[&str],
    index: &mut usize,
    option: &'static str,
    argument: &'static str,
) -> CommandResult<f64> {
    let value = args
        .get(*index + 1)
        .copied()
        .ok_or(EffectCommandParseError::MissingArgument { effect, argument })?;
    *index += 2;
    let _ = option;
    parse_f64(effect, argument, value)
}

fn parse_inline_f64(
    effect: &'static str,
    arg: &str,
    option: &'static str,
    argument: &'static str,
) -> CommandResult<f64> {
    parse_f64(effect, argument, &arg[option.len()..])
}

fn parse_option_frequency(
    effect: &'static str,
    args: &[&str],
    index: &mut usize,
    option: &'static str,
    argument: &'static str,
) -> CommandResult<f64> {
    let value = args
        .get(*index + 1)
        .copied()
        .ok_or(EffectCommandParseError::MissingArgument { effect, argument })?;
    let _ = option;
    *index += 2;
    parse_frequency_hz(effect, value)
}

fn parse_inline_frequency(
    effect: &'static str,
    arg: &str,
    option: &'static str,
    _argument: &'static str,
) -> CommandResult<f64> {
    parse_frequency_hz(effect, &arg[option.len()..])
}

fn parse_option_u32(
    effect: &'static str,
    args: &[&str],
    index: &mut usize,
    option: &'static str,
    argument: &'static str,
) -> CommandResult<u32> {
    let value = args
        .get(*index + 1)
        .copied()
        .ok_or(EffectCommandParseError::MissingArgument { effect, argument })?;
    *index += 2;
    parse_u32(effect, value, option, argument)
}

fn parse_inline_u32(
    effect: &'static str,
    arg: &str,
    option: &'static str,
    argument: &'static str,
) -> CommandResult<u32> {
    parse_u32(effect, &arg[option.len()..], option, argument)
}

fn parse_u32(
    effect: &'static str,
    value: &str,
    option: &'static str,
    argument: &'static str,
) -> CommandResult<u32> {
    value
        .parse::<u32>()
        .map_err(|source| EffectCommandParseError::InvalidFrameCount {
            effect,
            argument,
            value: value.to_owned(),
            source,
        })
        .map_err(|source| remap_invalid_number(source, option, argument))
}

fn remap_invalid_number(
    source: EffectCommandParseError,
    _option: &'static str,
    _argument: &'static str,
) -> EffectCommandParseError {
    source
}

fn starts_frequency(value: &str) -> bool {
    value
        .chars()
        .next()
        .is_some_and(|character| character.is_ascii_digit() || character == '.')
}

#[cfg(test)]
mod tests {
    use super::parse_sinc;
    use crate::{EffectCommand, EffectCommandParseError, EffectError, Sinc, SincBand, SincOptions};

    #[test]
    fn parses_low_pass_and_high_pass_forms() {
        assert_eq!(
            parse_sinc("sinc", &["-n", "11", "-4000"])
                .unwrap()
                .render_tokens(),
            ["sinc", "-n", "11", "-4000"]
        );
        assert_eq!(
            parse_sinc("sinc", &["-n11", "1000"]).unwrap(),
            EffectCommand::Sinc(
                Sinc::with_options(
                    SincBand::HighPass {
                        frequency_hz: 1000.0
                    },
                    SincOptions::with_taps(11).unwrap()
                )
                .unwrap()
            )
        );
    }

    #[test]
    fn rejects_band_ranges_and_unsupported_phase_options() {
        assert_eq!(
            parse_sinc("sinc", &["1000-4000"]).unwrap_err(),
            EffectCommandParseError::InvalidEffectConfig {
                effect: "sinc",
                argument: "frequency-range",
                source: EffectError::InvalidSinc,
            }
        );
        assert_eq!(
            parse_sinc("sinc", &["-M", "1000"]).unwrap_err(),
            EffectCommandParseError::UnsupportedOption {
                effect: "sinc",
                option: "-M".to_owned(),
            }
        );
    }
}

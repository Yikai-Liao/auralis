use crate::{
    Echos, EchosTap,
    command::{
        CommandResult, EffectCommand, EffectCommandParseError, parse_f64, render_f64, required_arg,
    },
};

pub(super) fn parse_echos(effect: &'static str, args: &[&str]) -> CommandResult<EffectCommand> {
    let gain_in = parse_f64(effect, "gain-in", required_arg(effect, args, "gain-in")?)?;
    let gain_out = parse_f64(
        effect,
        "gain-out",
        args.get(1)
            .copied()
            .ok_or(EffectCommandParseError::MissingArgument {
                effect,
                argument: "gain-out",
            })?,
    )?;
    let tap_args = &args[2..];

    if tap_args.is_empty() {
        return Err(EffectCommandParseError::MissingArgument {
            effect,
            argument: "delay-decay-pair",
        });
    }
    if !tap_args.len().is_multiple_of(2) {
        return Err(EffectCommandParseError::MissingArgument {
            effect,
            argument: "decay",
        });
    }

    let mut taps = Vec::with_capacity(tap_args.len() / 2);
    for pair in tap_args.chunks_exact(2) {
        let delay_ms = parse_f64(effect, "delay", pair[0])?;
        let decay = parse_f64(effect, "decay", pair[1])?;
        taps.push(EchosTap::new(delay_ms, decay).map_err(|source| {
            EffectCommandParseError::InvalidEffectConfig {
                effect,
                argument: "delay-decay-pair",
                source,
            }
        })?);
    }

    Ok(EffectCommand::Echos(
        Echos::new(gain_in, gain_out, taps).map_err(|source| {
            EffectCommandParseError::InvalidEffectConfig {
                effect,
                argument: "echos",
                source,
            }
        })?,
    ))
}

pub(super) fn render_echos(echos: &Echos) -> Vec<String> {
    let mut tokens = vec![
        "echos".to_owned(),
        render_f64(echos.gain_in()),
        render_f64(echos.gain_out()),
    ];
    for tap in echos.taps() {
        tokens.push(render_f64(tap.delay_ms()));
        tokens.push(render_f64(tap.decay()));
    }
    tokens
}

#[cfg(test)]
mod tests {
    use super::parse_echos;
    use crate::{Echos, EchosTap, EffectCommand, EffectCommandParseError};

    #[test]
    fn parses_one_or_more_delay_decay_pairs() {
        assert_eq!(
            parse_echos("echos", &["0.8", "0.9", "1", "0.5", "2.5", "0.25"]).unwrap(),
            EffectCommand::Echos(
                Echos::new(
                    0.8,
                    0.9,
                    [
                        EchosTap::new(1.0, 0.5).unwrap(),
                        EchosTap::new(2.5, 0.25).unwrap(),
                    ],
                )
                .unwrap()
            )
        );
    }

    #[test]
    fn render_echos_uses_canonical_numeric_tokens() {
        assert_eq!(
            parse_echos("echos", &["1.0", "0.5", "1.0", "0.25"])
                .unwrap()
                .render_tokens(),
            ["echos", "1", "0.5", "1", "0.25"]
        );
    }

    #[test]
    fn requires_gain_and_complete_delay_decay_pairs() {
        assert!(matches!(
            parse_echos("echos", &["1", "1", "10"]).unwrap_err(),
            EffectCommandParseError::MissingArgument {
                argument: "decay",
                ..
            }
        ));
        assert!(matches!(
            parse_echos("echos", &["1", "1"]).unwrap_err(),
            EffectCommandParseError::MissingArgument {
                argument: "delay-decay-pair",
                ..
            }
        ));
    }
}

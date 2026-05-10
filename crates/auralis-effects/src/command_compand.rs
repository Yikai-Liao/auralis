use crate::command::{CommandResult, EffectCommand, EffectCommandParseError, render_f64};
use crate::{Compand, CompandTransferPoint};

pub(super) fn parse_compand(effect: &'static str, args: &[&str]) -> CommandResult<EffectCommand> {
    if args.len() < 2 {
        return Err(EffectCommandParseError::MissingArgument {
            effect,
            argument: "attack,decay and transfer",
        });
    }
    if args.len() > 5 {
        return Err(EffectCommandParseError::UnexpectedArgument {
            effect,
            argument: args[5].to_owned(),
        });
    }

    Compand::parse_sox_args(args)
        .map(EffectCommand::Compand)
        .map_err(|source| EffectCommandParseError::InvalidEffectConfig {
            effect,
            argument: "compand",
            source,
        })
}

pub(super) fn render_compand(compand: &Compand) -> Vec<String> {
    let mut tokens = vec![
        "compand".to_owned(),
        render_attack_decay(compand),
        render_transfer(compand),
    ];

    if compand.gain_db != 0.0 || compand.initial_volume_db != 0.0 || compand.delay_seconds != 0.0 {
        tokens.push(render_f64(compand.gain_db));
    }
    if compand.initial_volume_db != 0.0 || compand.delay_seconds != 0.0 {
        tokens.push(render_f64(compand.initial_volume_db));
    }
    if compand.delay_seconds != 0.0 {
        tokens.push(render_f64(compand.delay_seconds));
    }

    tokens
}

fn render_attack_decay(compand: &Compand) -> String {
    compand
        .attack_decay()
        .iter()
        .flat_map(|pair| {
            [
                render_f64(pair.attack_seconds),
                render_f64(pair.decay_seconds),
            ]
        })
        .collect::<Vec<_>>()
        .join(",")
}

fn render_transfer(compand: &Compand) -> String {
    let mut rendered = String::new();
    if let Some(soft_knee_db) = compand.transfer().soft_knee_db() {
        rendered.push_str(&render_f64(soft_knee_db));
        rendered.push(':');
    }

    rendered.push_str(
        &compand
            .transfer()
            .points()
            .iter()
            .copied()
            .map(render_transfer_point)
            .collect::<Vec<_>>()
            .join(","),
    );
    rendered
}

fn render_transfer_point(point: CompandTransferPoint) -> String {
    if let Some(output_db) = point.output_db {
        format!(
            "{},{}",
            render_transfer_db(point.input_db),
            render_transfer_db(output_db)
        )
    } else {
        render_transfer_db(point.input_db)
    }
}

fn render_transfer_db(value: f64) -> String {
    if value <= crate::compand::SOX_SAMPLE_MIN_DB {
        "-inf".to_owned()
    } else {
        render_f64(value)
    }
}

#[cfg(test)]
mod tests {
    use super::parse_compand;
    use crate::{Compand, EffectCommand, EffectCommandParseError, EffectError};

    #[test]
    fn parses_and_renders_compand_arguments() {
        let command = parse_compand(
            "compand",
            &[
                "0.3,1,0.05,0.2",
                "6:-70,-60,-20,-30,0,-5",
                "-3",
                "-90",
                "0.2",
            ],
        )
        .unwrap();

        assert_eq!(
            command,
            EffectCommand::Compand(
                Compand::parse_sox_args(&[
                    "0.3,1,0.05,0.2",
                    "6:-70,-60,-20,-30,0,-5",
                    "-3",
                    "-90",
                    "0.2"
                ])
                .unwrap()
            )
        );
        assert_eq!(
            command.render_tokens(),
            [
                "compand",
                "0.3,1,0.05,0.2",
                "6:-70,-60,-20,-30,0,-5",
                "-3",
                "-90",
                "0.2"
            ]
        );
    }

    #[test]
    fn rejects_missing_invalid_and_extra_arguments() {
        assert_eq!(
            parse_compand("compand", &[]).unwrap_err(),
            EffectCommandParseError::MissingArgument {
                effect: "compand",
                argument: "attack,decay and transfer",
            }
        );
        assert_eq!(
            parse_compand("compand", &["0,1", "-20,-20,-30,-30"]).unwrap_err(),
            EffectCommandParseError::InvalidEffectConfig {
                effect: "compand",
                argument: "compand",
                source: EffectError::InvalidCompand,
            }
        );
        assert_eq!(
            parse_compand("compand", &["0,1", "-60,-60", "0", "0", "0", "extra"]).unwrap_err(),
            EffectCommandParseError::UnexpectedArgument {
                effect: "compand",
                argument: "extra".to_owned(),
            }
        );
    }
}

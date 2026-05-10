use crate::command::{
    CommandResult, EffectCommand, EffectCommandParseError, parse_f64, reject_extra_arguments,
    render_f64, required_arg,
};
use crate::{Biquad, BiquadCoefficients};

pub(super) fn parse_biquad(effect: &'static str, args: &[&str]) -> CommandResult<EffectCommand> {
    let b0 = parse_coefficient(effect, args, 0, "b0")?;
    let b1 = parse_coefficient(effect, args, 1, "b1")?;
    let b2 = parse_coefficient(effect, args, 2, "b2")?;
    let a0 = parse_coefficient(effect, args, 3, "a0")?;
    let a1 = parse_coefficient(effect, args, 4, "a1")?;
    let a2 = parse_coefficient(effect, args, 5, "a2")?;

    reject_extra_arguments(effect, args.get(6..).unwrap_or_default())?;

    BiquadCoefficients::from_raw(b0, b1, b2, a0, a1, a2)
        .map(Biquad::new)
        .map(EffectCommand::Biquad)
        .map_err(|source| EffectCommandParseError::InvalidEffectConfig {
            effect,
            argument: "coefficients",
            source: source.into(),
        })
}

pub(super) fn render_biquad(biquad: Biquad) -> Vec<String> {
    let coefficients = biquad.coefficients();
    vec![
        "biquad".to_owned(),
        render_f64(coefficients.b0),
        render_f64(coefficients.b1),
        render_f64(coefficients.b2),
        "1".to_owned(),
        render_f64(coefficients.a1),
        render_f64(coefficients.a2),
    ]
}

fn parse_coefficient(
    effect: &'static str,
    args: &[&str],
    index: usize,
    argument: &'static str,
) -> CommandResult<f64> {
    let value = required_arg(effect, &args[index..], argument)?;
    parse_f64(effect, argument, value)
}

#[cfg(test)]
mod tests {
    use super::parse_biquad;
    use crate::{Biquad, BiquadCoefficients, EffectCommand, EffectCommandParseError, EffectError};

    #[test]
    fn parses_raw_coefficients_and_renders_normalized_form() {
        let command = parse_biquad("biquad", &["2", "1", "0.5", "4", "-1", "0.25"]).unwrap();

        assert_eq!(
            command,
            EffectCommand::Biquad(Biquad::new(
                BiquadCoefficients::normalized(0.5, 0.25, 0.125, -0.25, 0.0625).unwrap()
            ))
        );
        assert_eq!(
            command.render_tokens(),
            ["biquad", "0.5", "0.25", "0.125", "1", "-0.25", "0.0625"]
        );
    }

    #[test]
    fn rejects_missing_extra_and_invalid_coefficients() {
        assert_eq!(
            parse_biquad("biquad", &["1", "0", "0", "1", "0"]).unwrap_err(),
            EffectCommandParseError::MissingArgument {
                effect: "biquad",
                argument: "a2",
            }
        );
        assert_eq!(
            parse_biquad("biquad", &["1", "0", "0", "1", "0", "0", "extra"]).unwrap_err(),
            EffectCommandParseError::UnexpectedArgument {
                effect: "biquad",
                argument: "extra".to_owned(),
            }
        );
        assert_eq!(
            parse_biquad("biquad", &["1", "0", "0", "0", "0", "0"]).unwrap_err(),
            EffectCommandParseError::InvalidEffectConfig {
                effect: "biquad",
                argument: "coefficients",
                source: EffectError::InvalidBiquadCoefficients,
            }
        );
    }
}

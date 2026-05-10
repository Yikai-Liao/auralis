use crate::{
    Vad, VadOptions,
    command::{CommandResult, EffectCommand, EffectCommandParseError, parse_f64, render_f64},
};

pub(super) fn parse_vad(effect: &'static str, args: &[&str]) -> CommandResult<EffectCommand> {
    let mut options = VadOptions::default();
    let mut offset = 0;

    while let Some(option) = args.get(offset).copied() {
        if !crate::command::is_option_like(option) {
            return Err(EffectCommandParseError::UnexpectedArgument {
                effect,
                argument: option.to_owned(),
            });
        }
        let value =
            args.get(offset + 1)
                .copied()
                .ok_or(EffectCommandParseError::MissingArgument {
                    effect,
                    argument: "option value",
                })?;
        match option {
            "-b" => options.boot_time = parse_f64(effect, "boot-time", value)?,
            "-N" => options.noise_tc_up = parse_f64(effect, "noise-tc-up", value)?,
            "-n" => options.noise_tc_down = parse_f64(effect, "noise-tc-down", value)?,
            "-r" => {
                options.noise_reduction_amount =
                    parse_f64(effect, "noise-reduction-amount", value)?;
            }
            "-f" => options.measure_frequency = parse_f64(effect, "measure-frequency", value)?,
            "-m" => options.measure_duration = parse_f64(effect, "measure-duration", value)?,
            "-M" => options.measure_tc = parse_f64(effect, "measure-tc", value)?,
            "-h" => {
                options.high_pass_frequency =
                    parse_frequency(effect, "high-pass-frequency", value)?;
            }
            "-l" => {
                options.low_pass_frequency = parse_frequency(effect, "low-pass-frequency", value)?;
            }
            "-H" => {
                options.high_pass_lifter_frequency =
                    parse_frequency(effect, "high-pass-lifter-frequency", value)?;
            }
            "-L" => {
                options.low_pass_lifter_frequency =
                    parse_frequency(effect, "low-pass-lifter-frequency", value)?;
            }
            "-T" => options.trigger_time = parse_f64(effect, "trigger-time", value)?,
            "-t" => options.trigger_level = parse_f64(effect, "trigger-level", value)?,
            "-s" => options.search_time = parse_f64(effect, "search-time", value)?,
            "-g" => options.gap_time = parse_f64(effect, "gap-time", value)?,
            "-p" => options.pre_trigger_time = parse_f64(effect, "pre-trigger-time", value)?,
            _ => {
                return Err(EffectCommandParseError::UnsupportedOption {
                    effect,
                    option: option.to_owned(),
                });
            }
        }
        offset += 2;
    }

    Vad::from_sox_options(options)
        .map(EffectCommand::Vad)
        .map_err(|source| EffectCommandParseError::InvalidEffectConfig {
            effect,
            argument: "vad",
            source,
        })
}

pub(super) fn render_vad(vad: Vad) -> Vec<String> {
    let mut tokens = vec!["vad".to_owned()];
    let Some(options) = vad.sox_options() else {
        return tokens;
    };
    let defaults = VadOptions::default();
    push_if_changed(&mut tokens, "-b", options.boot_time, defaults.boot_time);
    push_if_changed(&mut tokens, "-N", options.noise_tc_up, defaults.noise_tc_up);
    push_if_changed(
        &mut tokens,
        "-n",
        options.noise_tc_down,
        defaults.noise_tc_down,
    );
    push_if_changed(
        &mut tokens,
        "-r",
        options.noise_reduction_amount,
        defaults.noise_reduction_amount,
    );
    push_if_changed(
        &mut tokens,
        "-f",
        options.measure_frequency,
        defaults.measure_frequency,
    );
    push_if_changed(
        &mut tokens,
        "-m",
        options.measure_duration,
        defaults.measure_duration,
    );
    push_if_changed(&mut tokens, "-M", options.measure_tc, defaults.measure_tc);
    push_if_changed(
        &mut tokens,
        "-h",
        options.high_pass_frequency,
        defaults.high_pass_frequency,
    );
    push_if_changed(
        &mut tokens,
        "-l",
        options.low_pass_frequency,
        defaults.low_pass_frequency,
    );
    push_if_changed(
        &mut tokens,
        "-H",
        options.high_pass_lifter_frequency,
        defaults.high_pass_lifter_frequency,
    );
    push_if_changed(
        &mut tokens,
        "-L",
        options.low_pass_lifter_frequency,
        defaults.low_pass_lifter_frequency,
    );
    push_if_changed(
        &mut tokens,
        "-T",
        options.trigger_time,
        defaults.trigger_time,
    );
    push_if_changed(
        &mut tokens,
        "-t",
        options.trigger_level,
        defaults.trigger_level,
    );
    push_if_changed(&mut tokens, "-s", options.search_time, defaults.search_time);
    push_if_changed(&mut tokens, "-g", options.gap_time, defaults.gap_time);
    push_if_changed(
        &mut tokens,
        "-p",
        options.pre_trigger_time,
        defaults.pre_trigger_time,
    );
    tokens
}

fn parse_frequency(
    effect: &'static str,
    argument: &'static str,
    value: &str,
) -> CommandResult<f64> {
    let (number, multiplier) = value
        .strip_suffix('k')
        .or_else(|| value.strip_suffix('K'))
        .map_or((value, 1.0), |number| (number, 1000.0));
    parse_f64(effect, argument, number).map(|frequency| frequency * multiplier)
}

fn push_if_changed(tokens: &mut Vec<String>, option: &str, value: f64, default: f64) {
    if value.to_bits() != default.to_bits() {
        tokens.push(option.to_owned());
        tokens.push(render_f64(value));
    }
}

#[cfg(test)]
mod tests {
    use super::{parse_vad, render_vad};
    use crate::{EffectCommand, VadOptions};

    #[test]
    fn parses_default_vad_command() {
        let command = parse_vad("vad", &[]).unwrap();

        assert_eq!(
            command,
            EffectCommand::Vad(crate::Vad::from_sox_options(VadOptions::default()).unwrap())
        );
        assert_eq!(command.render_tokens(), ["vad"]);
    }

    #[test]
    fn parses_and_renders_advanced_options() {
        let command = parse_vad(
            "vad",
            &[
                "-T", "0.01", "-t", "1", "-g", "0.1", "-p", "0.001", "-h", "1k",
            ],
        )
        .unwrap();

        assert_eq!(
            command.render_tokens(),
            [
                "vad", "-h", "1000", "-T", "0.01", "-t", "1", "-g", "0.1", "-p", "0.001"
            ]
        );
    }

    #[test]
    fn rejects_unknown_options_and_out_of_range_values() {
        assert!(
            parse_vad("vad", &["-x", "1"])
                .unwrap_err()
                .to_string()
                .contains("-x")
        );
        assert!(
            parse_vad("vad", &["-t", "21"])
                .unwrap_err()
                .to_string()
                .contains("invalid `vad`")
        );
    }

    #[test]
    fn renders_frame_domain_typed_core_as_default_command() {
        assert_eq!(render_vad(crate::Vad::default()), ["vad"]);
    }
}

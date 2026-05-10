use auralis_core::SampleRate;

use crate::command::{
    CommandResult, EffectCommand, EffectCommandParseError, is_option_like, reject_extra_arguments,
    render_f32, required_arg,
};
use crate::{EffectError, Rate, RateBandwidth, RateOptions, RatePhase, RatePrecision, RateQuality};

pub(super) fn parse_rate(effect: &'static str, args: &[&str]) -> CommandResult<EffectCommand> {
    let (quality, options, args) = parse_options(effect, args)?;
    let value = required_arg(effect, args, "frequency")?;
    if is_option_like(value) {
        return Err(EffectCommandParseError::UnsupportedOption {
            effect,
            option: value.to_owned(),
        });
    }
    reject_extra_arguments(effect, args.get(1..).unwrap_or_default())?;

    let sample_rate = parse_sample_rate(effect, value)?;
    Rate::with_options(sample_rate, quality, options)
        .map(EffectCommand::Rate)
        .map_err(|source| EffectCommandParseError::InvalidEffectConfig {
            effect,
            argument: "options",
            source,
        })
}

pub(super) fn render_rate(rate: Rate) -> Vec<String> {
    let mut tokens = vec!["rate".to_owned()];
    if let Some(option) = rate.quality.command_option() {
        tokens.push(option.to_owned());
    }
    render_options(rate.options, &mut tokens);
    tokens.push(rate.target_sample_rate.as_u32().to_string());
    tokens
}

fn parse_options<'a>(
    effect: &'static str,
    args: &'a [&'a str],
) -> CommandResult<(RateQuality, RateOptions, &'a [&'a str])> {
    let mut quality = RateQuality::Default;
    let mut options = RateOptions::DEFAULT;
    let mut index = 0;
    let mut rejection_seen = false;

    while let Some(option) = args.get(index).copied() {
        if let Some(parsed_quality) = RateQuality::from_command_option(option) {
            quality = parsed_quality;
            index += 1;
            continue;
        }

        if option == "-Q" {
            let value = required_option_arg(effect, args, index, "quality")?;
            quality = RateQuality::from_quality_number(value).ok_or_else(|| {
                EffectCommandParseError::UnsupportedOption {
                    effect,
                    option: format!("-Q {value}"),
                }
            })?;
            index += 2;
            continue;
        }

        let Some(consumed) =
            parse_rate_option(effect, args, index, &mut options, &mut rejection_seen)?
        else {
            break;
        };
        index += consumed;
    }

    Ok((quality, options, &args[index..]))
}

fn parse_rate_option(
    effect: &'static str,
    args: &[&str],
    index: usize,
    options: &mut RateOptions,
    rejection_seen: &mut bool,
) -> CommandResult<Option<usize>> {
    if parse_no_value_rate_option(effect, args[index], options)? {
        return Ok(Some(1));
    }

    parse_value_rate_option(effect, args, index, options, rejection_seen)
}

fn parse_no_value_rate_option(
    effect: &'static str,
    option: &str,
    options: &mut RateOptions,
) -> CommandResult<bool> {
    match option {
        "-f" => options.flags = options.flags.with_no_rolloff(),
        "-n" => options.flags = options.flags.with_no_integer_optimization(),
        "-t" => options.flags = options.flags.with_high_precision_clock(),
        "-M" => options.phase = Some(RatePhase::Minimum),
        "-I" => options.phase = Some(RatePhase::Intermediate),
        "-L" => options.phase = Some(RatePhase::Linear),
        "-s" => set_bandwidth(effect, options, RateBandwidth::Steep)?,
        "-a" => options.flags = options.flags.with_allow_aliasing(),
        _ => return Ok(false),
    }

    Ok(true)
}

fn parse_value_rate_option(
    effect: &'static str,
    args: &[&str],
    index: usize,
    options: &mut RateOptions,
    rejection_seen: &mut bool,
) -> CommandResult<Option<usize>> {
    match args[index] {
        "-i" => parse_interpolator_option(effect, args, index, options),
        "-c" => parse_coefficient_budget_option(effect, args, index, options),
        "-p" => parse_phase_percent_option(effect, args, index, options),
        "-b" | "-B" => parse_bandwidth_option(effect, args, index, options),
        "-A" => parse_anti_aliasing_option(effect, args, index, options),
        "-d" | "-R" => parse_precision_option(effect, args, index, options, rejection_seen),
        _ => Ok(None),
    }
}

fn parse_interpolator_option(
    effect: &'static str,
    args: &[&str],
    index: usize,
    options: &mut RateOptions,
) -> CommandResult<Option<usize>> {
    let value = required_option_arg(effect, args, index, "interpolator")?;
    options.interpolator = Some(parse_integer_range(effect, "interpolator", value, -1, 2)?);
    Ok(Some(2))
}

fn parse_coefficient_budget_option(
    effect: &'static str,
    args: &[&str],
    index: usize,
    options: &mut RateOptions,
) -> CommandResult<Option<usize>> {
    let value = required_option_arg(effect, args, index, "coefficient-budget")?;
    options.coefficient_budget_kib = Some(parse_u32_range(
        effect,
        "coefficient-budget",
        value,
        100,
        u32::MAX,
    )?);
    Ok(Some(2))
}

fn parse_phase_percent_option(
    effect: &'static str,
    args: &[&str],
    index: usize,
    options: &mut RateOptions,
) -> CommandResult<Option<usize>> {
    let value = required_option_arg(effect, args, index, "phase")?;
    options.phase = Some(RatePhase::Percent(parse_float_range(
        effect, "phase", value, 0.0, 100.0,
    )?));
    Ok(Some(2))
}

fn parse_bandwidth_option(
    effect: &'static str,
    args: &[&str],
    index: usize,
    options: &mut RateOptions,
) -> CommandResult<Option<usize>> {
    let option = args[index];
    let value = required_option_arg(effect, args, index, "bandwidth")?;
    let bandwidth = if option == "-b" {
        RateBandwidth::ThreeDbPercent(parse_float_range(effect, "bandwidth", value, 74.0, 99.7)?)
    } else {
        RateBandwidth::ZeroDbPercent(parse_float_range(effect, "bandwidth", value, 53.0, 99.5)?)
    };
    set_bandwidth(effect, options, bandwidth)?;
    Ok(Some(2))
}

fn parse_anti_aliasing_option(
    effect: &'static str,
    args: &[&str],
    index: usize,
    options: &mut RateOptions,
) -> CommandResult<Option<usize>> {
    let value = required_option_arg(effect, args, index, "anti-aliasing-bandwidth")?;
    options.anti_aliasing_percent = Some(parse_float_range(
        effect,
        "anti-aliasing-bandwidth",
        value,
        85.0,
        100.0,
    )?);
    Ok(Some(2))
}

fn parse_precision_option(
    effect: &'static str,
    args: &[&str],
    index: usize,
    options: &mut RateOptions,
    rejection_seen: &mut bool,
) -> CommandResult<Option<usize>> {
    if args[index] == "-R" {
        let value = required_option_arg(effect, args, index, "rejection")?;
        options.precision =
            RatePrecision::RejectionDb(parse_float_range(effect, "rejection", value, 90.0, 200.0)?);
        *rejection_seen = true;
    } else {
        let value = required_option_arg(effect, args, index, "bit-depth")?;
        if *rejection_seen {
            parse_float_range(effect, "bit-depth", value, 15.0, 33.0)?;
        } else {
            options.precision =
                RatePrecision::BitDepth(parse_float_range(effect, "bit-depth", value, 15.0, 33.0)?);
        }
    }

    Ok(Some(2))
}

fn required_option_arg<'a>(
    effect: &'static str,
    args: &'a [&'a str],
    option_index: usize,
    argument: &'static str,
) -> CommandResult<&'a str> {
    args.get(option_index + 1)
        .copied()
        .ok_or(EffectCommandParseError::MissingArgument { effect, argument })
}

fn parse_float_range(
    effect: &'static str,
    argument: &'static str,
    value: &str,
    min: f64,
    max: f64,
) -> CommandResult<f32> {
    let parsed = value
        .parse::<f64>()
        .map_err(|source| EffectCommandParseError::InvalidNumber {
            effect,
            argument,
            value: value.to_owned(),
            source,
        })?;
    if !parsed.is_finite() || parsed < min || parsed > max {
        return Err(EffectCommandParseError::InvalidEffectConfig {
            effect,
            argument,
            source: EffectError::InvalidRateOptions,
        });
    }

    #[allow(
        clippy::cast_possible_truncation,
        reason = "rate option values are bounded to small SoX-ng percentages and bit-depth ranges before storing as f32 metadata"
    )]
    Ok(parsed as f32)
}

fn parse_integer_range(
    effect: &'static str,
    argument: &'static str,
    value: &str,
    min: i8,
    max: i8,
) -> CommandResult<i8> {
    let parsed = parse_float_range(effect, argument, value, f64::from(min), f64::from(max))?;
    if parsed.fract() != 0.0 {
        return Err(EffectCommandParseError::InvalidEffectConfig {
            effect,
            argument,
            source: EffectError::InvalidRateOptions,
        });
    }

    #[allow(
        clippy::cast_possible_truncation,
        reason = "integer range is validated against the i8 target range before casting"
    )]
    Ok(parsed as i8)
}

fn parse_u32_range(
    effect: &'static str,
    argument: &'static str,
    value: &str,
    min: u32,
    max: u32,
) -> CommandResult<u32> {
    let parsed = parse_float_range(effect, argument, value, f64::from(min), f64::from(max))?;
    if parsed.fract() != 0.0 {
        return Err(EffectCommandParseError::InvalidEffectConfig {
            effect,
            argument,
            source: EffectError::InvalidRateOptions,
        });
    }

    #[allow(
        clippy::cast_possible_truncation,
        clippy::cast_sign_loss,
        reason = "integer range is validated against the u32 target range before casting"
    )]
    Ok(parsed as u32)
}

fn set_bandwidth(
    effect: &'static str,
    options: &mut RateOptions,
    bandwidth: RateBandwidth,
) -> CommandResult<()> {
    let conflicts = matches!(
        (options.bandwidth, bandwidth),
        (
            RateBandwidth::Steep | RateBandwidth::ThreeDbPercent(_),
            RateBandwidth::ZeroDbPercent(_)
        ) | (
            RateBandwidth::ZeroDbPercent(_),
            RateBandwidth::Steep | RateBandwidth::ThreeDbPercent(_)
        )
    );
    if conflicts {
        return Err(EffectCommandParseError::InvalidOptionCombination {
            effect,
            options: "-b/-s with -B",
        });
    }

    options.bandwidth = bandwidth;
    Ok(())
}

fn render_options(options: RateOptions, tokens: &mut Vec<String>) {
    if let Some(interpolator) = options.interpolator {
        tokens.extend(["-i".to_owned(), interpolator.to_string()]);
    }
    if let Some(coefficient_budget_kib) = options.coefficient_budget_kib {
        tokens.extend(["-c".to_owned(), coefficient_budget_kib.to_string()]);
    }
    if options.flags.no_rolloff() {
        tokens.push("-f".to_owned());
    }
    if options.flags.no_integer_optimization() {
        tokens.push("-n".to_owned());
    }
    if options.flags.high_precision_clock() {
        tokens.push("-t".to_owned());
    }
    if let Some(phase) = options.phase {
        match phase {
            RatePhase::Minimum => tokens.push("-M".to_owned()),
            RatePhase::Intermediate => tokens.push("-I".to_owned()),
            RatePhase::Linear => tokens.push("-L".to_owned()),
            RatePhase::Maximum => tokens.extend(["-p".to_owned(), "100".to_owned()]),
            RatePhase::Percent(percent) => {
                tokens.extend(["-p".to_owned(), render_f32(percent)]);
            }
        }
    }
    match options.bandwidth {
        RateBandwidth::Default => {}
        RateBandwidth::Steep => tokens.push("-s".to_owned()),
        RateBandwidth::ThreeDbPercent(percent) => {
            tokens.extend(["-b".to_owned(), render_f32(percent)]);
        }
        RateBandwidth::ZeroDbPercent(percent) => {
            tokens.extend(["-B".to_owned(), render_f32(percent)]);
        }
    }
    if let Some(anti_aliasing_percent) = options.anti_aliasing_percent {
        tokens.extend(["-A".to_owned(), render_f32(anti_aliasing_percent)]);
    }
    if options.flags.allow_aliasing() {
        tokens.push("-a".to_owned());
    }
    match options.precision {
        RatePrecision::Default => {}
        RatePrecision::BitDepth(bit_depth) => {
            tokens.extend(["-d".to_owned(), render_f32(bit_depth)]);
        }
        RatePrecision::RejectionDb(rejection) => {
            tokens.extend(["-R".to_owned(), render_f32(rejection)]);
        }
    }
}

fn parse_sample_rate(effect: &'static str, value: &str) -> CommandResult<SampleRate> {
    let (number_text, multiplier) = if let Some(number) = value.strip_suffix('k') {
        (number, 1_000.0)
    } else if let Some(number) = value.strip_suffix('K') {
        (number, 1_000.0)
    } else {
        (value, 1.0)
    };
    let number =
        number_text
            .parse::<f64>()
            .map_err(|source| EffectCommandParseError::InvalidNumber {
                effect,
                argument: "frequency",
                value: value.to_owned(),
                source,
            })?;
    let sample_rate = number * multiplier;
    if !sample_rate.is_finite() || sample_rate < 0.5 || sample_rate > f64::from(u32::MAX) {
        return Err(EffectCommandParseError::InvalidCoreValue {
            effect,
            argument: "frequency",
            source: auralis_core::AuralisError::InvalidSampleRate,
        });
    }

    #[allow(
        clippy::cast_possible_truncation,
        clippy::cast_sign_loss,
        reason = "range and finiteness are validated before rounding into Auralis' integer sample-rate type"
    )]
    let rounded = sample_rate.round() as u32;
    SampleRate::new(rounded).map_err(|source| EffectCommandParseError::InvalidCoreValue {
        effect,
        argument: "frequency",
        source,
    })
}

impl RateQuality {
    fn from_command_option(option: &str) -> Option<Self> {
        match option {
            "-q" => Some(Self::Quick),
            "-l" => Some(Self::Low),
            "-m" => Some(Self::Medium),
            "-g" => Some(Self::Generic),
            "-h" => Some(Self::High),
            "-e" => Some(Self::Extreme),
            "-v" => Some(Self::VeryHigh),
            "-u" => Some(Self::Ultra),
            _ => None,
        }
    }

    fn from_quality_number(value: &str) -> Option<Self> {
        match value {
            "0" => Some(Self::Quick),
            "1" => Some(Self::Low),
            "2" => Some(Self::Medium),
            "3" => Some(Self::Generic),
            "4" => Some(Self::High),
            "5" => Some(Self::Extreme),
            "6" => Some(Self::VeryHigh),
            "7" => Some(Self::Ultra),
            _ => None,
        }
    }

    fn command_option(self) -> Option<&'static str> {
        match self {
            Self::Default => None,
            Self::Quick => Some("-q"),
            Self::Low => Some("-l"),
            Self::Medium => Some("-m"),
            Self::Generic => Some("-g"),
            Self::High => Some("-h"),
            Self::Extreme => Some("-e"),
            Self::VeryHigh => Some("-v"),
            Self::Ultra => Some("-u"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::parse_rate;
    use crate::{
        EffectCommand, EffectCommandParseError, Rate, RateBandwidth, RateOptionFlags, RateOptions,
        RatePhase, RatePrecision, RateQuality,
    };
    use auralis_core::SampleRate;

    #[test]
    fn parses_integer_and_kilohertz_frequency() {
        assert_eq!(
            parse_rate("rate", &["24000"]).unwrap(),
            EffectCommand::Rate(Rate::new(SampleRate::new(24_000).unwrap()))
        );
        assert_eq!(
            parse_rate("rate", &["44.1k"]).unwrap(),
            EffectCommand::Rate(Rate::new(SampleRate::new(44_100).unwrap()))
        );
        assert_eq!(
            parse_rate("rate", &["44.1k"]).unwrap().render_tokens(),
            ["rate", "44100"]
        );
    }

    #[test]
    fn parses_quick_and_low_quality_modes() {
        assert_eq!(
            parse_rate("rate", &["-q", "24000"]).unwrap(),
            EffectCommand::Rate(Rate::quick(SampleRate::new(24_000).unwrap()))
        );
        assert_eq!(
            parse_rate("rate", &["-Q", "0", "24k"])
                .unwrap()
                .render_tokens(),
            ["rate", "-q", "24000"]
        );
        assert_eq!(
            parse_rate("rate", &["-l", "48000"]).unwrap(),
            EffectCommand::Rate(Rate::low(SampleRate::new(48_000).unwrap()))
        );
        assert_eq!(
            parse_rate("rate", &["-Q", "1", "48k"])
                .unwrap()
                .render_tokens(),
            ["rate", "-l", "48000"]
        );
    }

    #[test]
    fn parses_high_quality_modes() {
        let sample_rate = SampleRate::new(48_000).unwrap();
        let cases = [
            (
                ["-m", "48k"],
                Rate::medium(sample_rate),
                ["rate", "-m", "48000"],
            ),
            (
                ["-g", "48k"],
                Rate::generic(sample_rate),
                ["rate", "-g", "48000"],
            ),
            (
                ["-h", "48k"],
                Rate::high(sample_rate),
                ["rate", "-h", "48000"],
            ),
            (
                ["-e", "48k"],
                Rate::extreme(sample_rate),
                ["rate", "-e", "48000"],
            ),
            (
                ["-v", "48k"],
                Rate::very_high(sample_rate),
                ["rate", "-v", "48000"],
            ),
            (
                ["-u", "48k"],
                Rate::ultra(sample_rate),
                ["rate", "-u", "48000"],
            ),
        ];

        for (args, expected, rendered) in cases {
            let command = parse_rate("rate", &args).unwrap();
            assert_eq!(command, EffectCommand::Rate(expected));
            assert_eq!(command.render_tokens(), rendered);
        }

        assert_eq!(
            parse_rate("rate", &["-Q", "2", "48k"])
                .unwrap()
                .render_tokens(),
            ["rate", "-m", "48000"]
        );
        assert_eq!(
            parse_rate("rate", &["-Q", "7", "48k"])
                .unwrap()
                .render_tokens(),
            ["rate", "-u", "48000"]
        );
    }

    #[test]
    fn parses_rate_control_and_override_options() {
        let command = parse_rate(
            "rate",
            &[
                "-h", "-i", "2", "-c", "800", "-f", "-n", "-t", "-M", "-s", "-A", "95", "-a", "-R",
                "120", "44.1k",
            ],
        )
        .unwrap();
        let expected = Rate::with_options(
            SampleRate::new(44_100).unwrap(),
            RateQuality::High,
            RateOptions {
                interpolator: Some(2),
                coefficient_budget_kib: Some(800),
                flags: RateOptionFlags::DEFAULT
                    .with_no_rolloff()
                    .with_no_integer_optimization()
                    .with_high_precision_clock()
                    .with_allow_aliasing(),
                phase: Some(RatePhase::Minimum),
                bandwidth: RateBandwidth::Steep,
                anti_aliasing_percent: Some(95.0),
                precision: RatePrecision::RejectionDb(120.0),
            },
        )
        .unwrap();

        assert_eq!(command, EffectCommand::Rate(expected));
        assert_eq!(
            command.render_tokens(),
            [
                "rate", "-h", "-i", "2", "-c", "800", "-f", "-n", "-t", "-M", "-s", "-A", "95",
                "-a", "-R", "120", "44100"
            ]
        );
    }

    #[test]
    fn parses_custom_override_values() {
        let command = parse_rate(
            "rate",
            &["-Q", "3", "-p", "12.5", "-B", "91.3", "-d", "20", "48000"],
        )
        .unwrap();

        assert_eq!(
            command.render_tokens(),
            [
                "rate", "-g", "-p", "12.5", "-B", "91.3", "-d", "20", "48000"
            ]
        );
        assert_eq!(
            command,
            EffectCommand::Rate(
                Rate::with_options(
                    SampleRate::new(48_000).unwrap(),
                    RateQuality::Generic,
                    RateOptions {
                        phase: Some(RatePhase::Percent(12.5)),
                        bandwidth: RateBandwidth::ZeroDbPercent(91.3),
                        precision: RatePrecision::BitDepth(20.0),
                        ..RateOptions::DEFAULT
                    },
                )
                .unwrap()
            )
        );
    }

    #[test]
    fn rejects_missing_invalid_option_and_extra_arguments() {
        assert_eq!(
            parse_rate("rate", &[]).unwrap_err(),
            EffectCommandParseError::MissingArgument {
                effect: "rate",
                argument: "frequency",
            }
        );
        assert!(matches!(
            parse_rate("rate", &["bad"]).unwrap_err(),
            EffectCommandParseError::InvalidNumber {
                effect: "rate",
                argument: "frequency",
                ..
            }
        ));
        assert_eq!(
            parse_rate("rate", &["0"]).unwrap_err(),
            EffectCommandParseError::InvalidCoreValue {
                effect: "rate",
                argument: "frequency",
                source: auralis_core::AuralisError::InvalidSampleRate,
            }
        );
        assert_eq!(
            parse_rate("rate", &["-q"]).unwrap_err(),
            EffectCommandParseError::MissingArgument {
                effect: "rate",
                argument: "frequency",
            }
        );
        assert_eq!(
            parse_rate("rate", &["-Q"]).unwrap_err(),
            EffectCommandParseError::MissingArgument {
                effect: "rate",
                argument: "quality",
            }
        );
        assert_eq!(
            parse_rate("rate", &["-Q", "8", "24000"]).unwrap_err(),
            EffectCommandParseError::UnsupportedOption {
                effect: "rate",
                option: "-Q 8".to_owned(),
            }
        );
        assert_eq!(
            parse_rate("rate", &["-p"]).unwrap_err(),
            EffectCommandParseError::MissingArgument {
                effect: "rate",
                argument: "phase",
            }
        );
        assert!(matches!(
            parse_rate("rate", &["-p", "101", "24000"]).unwrap_err(),
            EffectCommandParseError::InvalidEffectConfig {
                effect: "rate",
                argument: "phase",
                source: crate::EffectError::InvalidRateOptions,
            }
        ));
        assert!(matches!(
            parse_rate("rate", &["-q", "-M", "24000"]).unwrap_err(),
            EffectCommandParseError::InvalidEffectConfig {
                effect: "rate",
                argument: "options",
                source: crate::EffectError::InvalidRateOptions,
            }
        ));
        assert!(matches!(
            parse_rate("rate", &["-b", "80", "-a", "24000"]).unwrap_err(),
            EffectCommandParseError::InvalidEffectConfig {
                effect: "rate",
                argument: "options",
                source: crate::EffectError::InvalidRateOptions,
            }
        ));
        assert_eq!(
            parse_rate("rate", &["-s", "-B", "90", "24000"]).unwrap_err(),
            EffectCommandParseError::InvalidOptionCombination {
                effect: "rate",
                options: "-b/-s with -B",
            }
        );
        assert_eq!(
            parse_rate("rate", &["24000", "extra"]).unwrap_err(),
            EffectCommandParseError::UnexpectedArgument {
                effect: "rate",
                argument: "extra".to_owned(),
            }
        );
    }
}

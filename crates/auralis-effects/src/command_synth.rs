use auralis_core::FrameCount;

use crate::{
    Synth, SynthChannel, SynthLength, SynthWaveform,
    command::{CommandResult, EffectCommand, EffectCommandParseError, parse_f64, render_f64},
};

pub(super) fn parse_synth(effect: &'static str, args: &[&str]) -> CommandResult<EffectCommand> {
    let mut index = 0;
    let mut no_headroom = false;
    while matches!(args.get(index).copied(), Some("-n")) {
        no_headroom = true;
        index += 1;
    }
    if let Some(option) = args
        .get(index)
        .copied()
        .filter(|token| token.starts_with('-'))
    {
        return Err(EffectCommandParseError::UnsupportedOption {
            effect,
            option: option.to_owned(),
        });
    }

    let length = if args
        .get(index)
        .is_some_and(|token| starts_like_synth_length(token))
    {
        let parsed = parse_synth_length(effect, args[index])?;
        index += 1;
        Some(parsed)
    } else {
        None
    };

    let mut channels = Vec::new();
    while index < args.len() {
        let waveform = parse_waveform(effect, args[index])?;
        index += 1;
        if let Some(combine) = args
            .get(index)
            .copied()
            .filter(|token| is_combine_mode(token))
        {
            return Err(EffectCommandParseError::UnsupportedOption {
                effect,
                option: combine.to_owned(),
            });
        }

        let frequency_hz = args
            .get(index)
            .filter(|token| !is_waveform(token))
            .map(|value| parse_f64(effect, "frequency", value))
            .transpose()?
            .unwrap_or(440.0);
        if index < args.len() && !is_waveform(args[index]) {
            index += 1;
        }

        let mut params = Vec::new();
        while index < args.len() && !is_waveform(args[index]) {
            params.push(parse_f64(effect, "parameter", args[index])? / 100.0);
            index += 1;
        }
        if params.len() > 5 {
            return Err(EffectCommandParseError::UnexpectedArgument {
                effect,
                argument: args[index - 1].to_owned(),
            });
        }

        channels.push(channel_from_parts(effect, waveform, frequency_hz, &params)?);
    }

    let synth = if channels.is_empty() {
        Synth::new()
    } else {
        Synth::with_channels(length, channels)
    }
    .map_err(|source| EffectCommandParseError::InvalidEffectConfig {
        effect,
        argument: "synth",
        source,
    })?;

    Ok(EffectCommand::Synth(if no_headroom {
        synth.without_headroom()
    } else {
        synth
    }))
}

pub(super) fn render_synth(synth: &Synth) -> Vec<String> {
    let mut tokens = vec!["synth".to_owned()];
    if synth.no_headroom() {
        tokens.push("-n".to_owned());
    }
    if let Some(length) = synth.length() {
        tokens.push(render_length(length));
    }
    for channel in synth.channel_specs() {
        tokens.push(render_waveform(channel.waveform).to_owned());
        tokens.push(render_f64(channel.frequency_hz));
        if channel.offset.to_bits() != 0.0_f64.to_bits()
            || channel.phase.to_bits() != 0.0_f64.to_bits()
            || has_explicit_shape(channel)
        {
            tokens.push(render_percent(channel.offset));
            tokens.push(render_percent(channel.phase));
        }
        if has_explicit_shape(channel) {
            tokens.push(render_percent(channel.p1));
            match channel.waveform {
                SynthWaveform::Trapezium | SynthWaveform::Exp => {
                    tokens.push(render_percent(channel.p2));
                }
                _ => {}
            }
            if channel.waveform == SynthWaveform::Trapezium {
                tokens.push(render_percent(channel.p3));
            }
        }
    }
    tokens
}

fn channel_from_parts(
    effect: &'static str,
    waveform: SynthWaveform,
    frequency_hz: f64,
    params: &[f64],
) -> CommandResult<SynthChannel> {
    let offset = params.first().copied().unwrap_or(0.0);
    let phase = params.get(1).copied().unwrap_or(0.0);
    SynthChannel::with_parameters(
        waveform,
        frequency_hz,
        offset,
        phase,
        params.get(2).copied(),
        params.get(3).copied(),
        params.get(4).copied(),
    )
    .map_err(|source| EffectCommandParseError::InvalidEffectConfig {
        effect,
        argument: "synth",
        source,
    })
}

fn parse_synth_length(effect: &'static str, value: &str) -> CommandResult<SynthLength> {
    if let Some(frames) = value
        .strip_suffix('s')
        .filter(|prefix| !prefix.contains('.'))
    {
        return frames
            .parse::<u64>()
            .map(|frames| SynthLength::frames(FrameCount::new(frames)))
            .map_err(|source| EffectCommandParseError::InvalidFrameCount {
                effect,
                argument: "length",
                value: value.to_owned(),
                source,
            });
    }

    SynthLength::seconds(parse_f64(effect, "length", value)?).map_err(|source| {
        EffectCommandParseError::InvalidEffectConfig {
            effect,
            argument: "length",
            source,
        }
    })
}

fn parse_waveform(effect: &'static str, value: &str) -> CommandResult<SynthWaveform> {
    match value {
        "sine" | "sin" => Ok(SynthWaveform::Sine),
        "square" => Ok(SynthWaveform::Square),
        "sawtooth" | "saw" => Ok(SynthWaveform::Sawtooth),
        "triangle" | "tri" => Ok(SynthWaveform::Triangle),
        "trapezium" | "trapetz" => Ok(SynthWaveform::Trapezium),
        "exp" => Ok(SynthWaveform::Exp),
        "whitenoise" | "noise" | "tpdfnoise" | "pinknoise" | "brownnoise" | "pluck" => {
            Err(EffectCommandParseError::UnsupportedOption {
                effect,
                option: value.to_owned(),
            })
        }
        other => Err(EffectCommandParseError::UnexpectedArgument {
            effect,
            argument: other.to_owned(),
        }),
    }
}

pub(crate) fn is_synth_waveform(value: &str) -> bool {
    is_waveform(value)
}

pub(crate) fn is_synth_combine_mode(value: &str) -> bool {
    is_combine_mode(value)
}

fn is_waveform(value: &str) -> bool {
    matches!(
        value,
        "sine"
            | "sin"
            | "square"
            | "sawtooth"
            | "saw"
            | "triangle"
            | "tri"
            | "trapezium"
            | "trapetz"
            | "exp"
            | "whitenoise"
            | "noise"
            | "tpdfnoise"
            | "pinknoise"
            | "brownnoise"
            | "pluck"
    )
}

fn is_combine_mode(value: &str) -> bool {
    matches!(value, "create" | "mix" | "amod" | "fmod" | "vdelay")
}

pub(crate) fn starts_like_synth_length(value: &str) -> bool {
    value
        .as_bytes()
        .first()
        .is_some_and(|byte| byte.is_ascii_digit() || *byte == b'.')
}

fn render_waveform(waveform: SynthWaveform) -> &'static str {
    match waveform {
        SynthWaveform::Sine => "sine",
        SynthWaveform::Square => "square",
        SynthWaveform::Sawtooth => "sawtooth",
        SynthWaveform::Triangle => "triangle",
        SynthWaveform::Trapezium => "trapezium",
        SynthWaveform::Exp => "exp",
    }
}

fn render_length(length: SynthLength) -> String {
    match length {
        SynthLength::Seconds(seconds) => render_f64(seconds),
        SynthLength::Frames(frames) => format!("{}s", frames.as_u64()),
    }
}

fn render_percent(value: f64) -> String {
    render_f64(value * 100.0)
}

fn has_explicit_shape(channel: &SynthChannel) -> bool {
    match channel.waveform {
        SynthWaveform::Sine | SynthWaveform::Sawtooth => false,
        SynthWaveform::Square | SynthWaveform::Triangle => {
            channel.p1.to_bits() != 0.5_f64.to_bits()
        }
        SynthWaveform::Trapezium => {
            channel.p1.to_bits() != 0.1_f64.to_bits()
                || channel.p2.to_bits() != 0.5_f64.to_bits()
                || channel.p3.to_bits() != 0.6_f64.to_bits()
        }
        SynthWaveform::Exp => {
            channel.p1.to_bits() != 0.5_f64.to_bits() || channel.p2.to_bits() != 0.5_f64.to_bits()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::parse_synth;
    use crate::{
        EffectCommand, EffectCommandParseError, EffectError, Synth, SynthChannel, SynthLength,
        SynthWaveform,
    };
    use auralis_core::FrameCount;

    #[test]
    fn parses_defaults_and_basic_waveforms() {
        assert_eq!(
            parse_synth("synth", &[]).unwrap(),
            EffectCommand::Synth(Synth::new().unwrap())
        );
        assert_eq!(
            parse_synth("synth", &["-n", "16s", "sine", "1000"])
                .unwrap()
                .render_tokens(),
            ["synth", "-n", "16s", "sine", "1000"]
        );
        assert_eq!(
            parse_synth("synth", &["triangle", "4", "0", "25", "25"])
                .unwrap()
                .render_tokens(),
            ["synth", "triangle", "4", "0", "25", "25"]
        );
    }

    #[test]
    fn parses_multiple_channel_specs() {
        let expected = Synth::with_channels(
            Some(SynthLength::frames(FrameCount::new(4))),
            [
                SynthChannel::new(SynthWaveform::Sine, 1.0).unwrap(),
                SynthChannel::new(SynthWaveform::Square, 2.0).unwrap(),
            ],
        )
        .unwrap();
        assert_eq!(
            parse_synth("synth", &["4s", "sine", "1", "square", "2"]).unwrap(),
            EffectCommand::Synth(expected)
        );
    }

    #[test]
    fn rejects_future_noise_sweep_and_combine_modes() {
        assert_eq!(
            parse_synth("synth", &["noise"]).unwrap_err(),
            EffectCommandParseError::UnsupportedOption {
                effect: "synth",
                option: "noise".to_owned(),
            }
        );
        assert_eq!(
            parse_synth("synth", &["sine", "440:880"]).unwrap_err(),
            EffectCommandParseError::InvalidNumber {
                effect: "synth",
                argument: "frequency",
                value: "440:880".to_owned(),
                source: "440:880".parse::<f64>().unwrap_err(),
            }
        );
        assert_eq!(
            parse_synth("synth", &["sine", "mix"]).unwrap_err(),
            EffectCommandParseError::UnsupportedOption {
                effect: "synth",
                option: "mix".to_owned(),
            }
        );
    }

    #[test]
    fn rejects_invalid_parameters() {
        assert!(matches!(
            parse_synth("synth", &["sine", "-1"]).unwrap_err(),
            EffectCommandParseError::InvalidEffectConfig {
                source: EffectError::InvalidSynth,
                ..
            }
        ));
        assert_eq!(
            parse_synth("synth", &["sine", "440", "0", "0", "0", "0", "0", "1"]).unwrap_err(),
            EffectCommandParseError::UnexpectedArgument {
                effect: "synth",
                argument: "1".to_owned(),
            }
        );
    }
}

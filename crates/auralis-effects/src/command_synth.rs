use auralis_core::FrameCount;

use crate::{
    Synth, SynthChannel, SynthCombineMode, SynthLength, SynthSweep, SynthVariableDelay,
    SynthWaveform,
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
        let combine = if let Some(combine) = args
            .get(index)
            .copied()
            .filter(|token| is_combine_mode(token))
        {
            index += 1;
            let (combine, consumed_vdelay) = parse_combine(effect, combine, &args[index..])?;
            index += consumed_vdelay;
            combine
        } else {
            SynthCombineMode::Create
        };

        let frequency = args
            .get(index)
            .filter(|token| !is_waveform(token))
            .map(|value| parse_frequency(effect, value, length.is_some()))
            .transpose()?
            .unwrap_or(SynthFrequency {
                frequency_hz: 440.0,
                sweep: None,
                frequency2_hz: None,
            });
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

        channels.push(channel_from_parts(
            effect, waveform, combine, frequency, &params,
        )?);
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
        if channel.combine != SynthCombineMode::Create {
            render_combine(channel.combine, &mut tokens);
        }
        tokens.push(render_frequency(channel));
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
    combine: SynthCombineMode,
    frequency: SynthFrequency,
    params: &[f64],
) -> CommandResult<SynthChannel> {
    let offset = params.first().copied().unwrap_or(0.0);
    let phase = params.get(1).copied().unwrap_or(0.0);
    let channel = SynthChannel::with_parameters(
        waveform,
        frequency.frequency_hz,
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
    })?
    .with_combine(combine);

    match (frequency.sweep, frequency.frequency2_hz) {
        (Some(sweep), Some(frequency2_hz)) => {
            channel.with_sweep(sweep, frequency2_hz).map_err(|source| {
                EffectCommandParseError::InvalidEffectConfig {
                    effect,
                    argument: "frequency",
                    source,
                }
            })
        }
        _ => Ok(channel),
    }
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
        "whitenoise" | "white" | "noise" => Ok(SynthWaveform::WhiteNoise),
        "tpdfnoise" | "tpdf" => Ok(SynthWaveform::TpdfNoise),
        "pinknoise" | "pink" => Ok(SynthWaveform::PinkNoise),
        "brownnoise" | "brown" => Ok(SynthWaveform::BrownNoise),
        "pluck" => Err(EffectCommandParseError::UnsupportedOption {
            effect,
            option: value.to_owned(),
        }),
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
            | "white"
            | "noise"
            | "tpdfnoise"
            | "tpdf"
            | "pinknoise"
            | "pink"
            | "brownnoise"
            | "brown"
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
        SynthWaveform::WhiteNoise => "whitenoise",
        SynthWaveform::TpdfNoise => "tpdfnoise",
        SynthWaveform::PinkNoise => "pinknoise",
        SynthWaveform::BrownNoise => "brownnoise",
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
        SynthWaveform::Sine
        | SynthWaveform::Sawtooth
        | SynthWaveform::WhiteNoise
        | SynthWaveform::TpdfNoise
        | SynthWaveform::PinkNoise
        | SynthWaveform::BrownNoise => false,
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

#[derive(Debug, Clone, Copy, PartialEq)]
struct SynthFrequency {
    frequency_hz: f64,
    sweep: Option<SynthSweep>,
    frequency2_hz: Option<f64>,
}

fn parse_combine(
    effect: &'static str,
    combine: &str,
    rest: &[&str],
) -> CommandResult<(SynthCombineMode, usize)> {
    match combine {
        "create" => Ok((SynthCombineMode::Create, 0)),
        "mix" => Ok((SynthCombineMode::Mix, 0)),
        "amod" => Ok((SynthCombineMode::Amod, 0)),
        "fmod" => Ok((SynthCombineMode::Fmod, 0)),
        "vdelay" => {
            let value = rest
                .first()
                .copied()
                .ok_or(EffectCommandParseError::MissingArgument {
                    effect,
                    argument: "vdelay",
                })?;
            parse_vdelay(effect, value).map(|delay| (SynthCombineMode::Vdelay(delay), 1))
        }
        _ => unreachable!("caller filters synth combine modes"),
    }
}

fn parse_vdelay(effect: &'static str, value: &str) -> CommandResult<SynthVariableDelay> {
    let parts = value.split(',').collect::<Vec<_>>();
    if parts.is_empty() || parts.len() > 3 || parts.iter().any(|part| part.is_empty()) {
        return Err(EffectCommandParseError::UnexpectedArgument {
            effect,
            argument: value.to_owned(),
        });
    }
    let fixed_ms = parse_f64(effect, "vdelay", parts[0])?;
    let extra_ms = parts
        .get(1)
        .map(|part| parse_f64(effect, "vdelay", part))
        .transpose()?
        .unwrap_or(0.0);
    let mix = parts
        .get(2)
        .map(|part| parse_f64(effect, "vdelay", part).map(|value| value / 100.0))
        .transpose()?
        .unwrap_or(0.5);
    SynthVariableDelay::new(fixed_ms, extra_ms, mix).map_err(|source| {
        EffectCommandParseError::InvalidEffectConfig {
            effect,
            argument: "vdelay",
            source,
        }
    })
}

fn parse_frequency(
    effect: &'static str,
    value: &str,
    has_length: bool,
) -> CommandResult<SynthFrequency> {
    if let Some((left, separator, right)) = split_sweep(value) {
        if !has_length {
            return Err(EffectCommandParseError::InvalidEffectConfig {
                effect,
                argument: "frequency",
                source: crate::EffectError::InvalidSynth,
            });
        }
        let frequency_hz = parse_f64(effect, "frequency", left)?;
        let end_hz = parse_f64(effect, "frequency", right)?;
        return Ok(SynthFrequency {
            frequency_hz,
            sweep: Some(match separator {
                ':' => SynthSweep::Linear,
                '+' => SynthSweep::Square,
                '/' => SynthSweep::Exponential,
                '-' => SynthSweep::ExponentialCycle,
                _ => unreachable!("split_sweep returns only synth sweep separators"),
            }),
            frequency2_hz: Some(end_hz),
        });
    }

    Ok(SynthFrequency {
        frequency_hz: parse_f64(effect, "frequency", value)?,
        sweep: None,
        frequency2_hz: None,
    })
}

fn split_sweep(value: &str) -> Option<(&str, char, &str)> {
    for (index, separator) in value.char_indices().skip(1) {
        if matches!(separator, ':' | '+' | '/' | '-') {
            let right_start = index + separator.len_utf8();
            return Some((&value[..index], separator, &value[right_start..]));
        }
    }
    None
}

fn render_combine(combine: SynthCombineMode, tokens: &mut Vec<String>) {
    match combine {
        SynthCombineMode::Create => {}
        SynthCombineMode::Mix => tokens.push("mix".to_owned()),
        SynthCombineMode::Amod => tokens.push("amod".to_owned()),
        SynthCombineMode::Fmod => tokens.push("fmod".to_owned()),
        SynthCombineMode::Vdelay(delay) => {
            tokens.push("vdelay".to_owned());
            tokens.push(render_vdelay(delay));
        }
    }
}

fn render_vdelay(delay: SynthVariableDelay) -> String {
    if delay.extra_ms.to_bits() == 0.0_f64.to_bits() && delay.mix.to_bits() == 0.5_f64.to_bits() {
        render_f64(delay.fixed_ms)
    } else if delay.mix.to_bits() == 0.5_f64.to_bits() {
        format!(
            "{},{}",
            render_f64(delay.fixed_ms),
            render_f64(delay.extra_ms)
        )
    } else {
        format!(
            "{},{},{}",
            render_f64(delay.fixed_ms),
            render_f64(delay.extra_ms),
            render_f64(delay.mix * 100.0)
        )
    }
}

fn render_frequency(channel: &SynthChannel) -> String {
    let Some(sweep) = channel.sweep else {
        return render_f64(channel.frequency_hz);
    };
    let separator = match sweep {
        SynthSweep::Linear => ":",
        SynthSweep::Square => "+",
        SynthSweep::Exponential => "/",
        SynthSweep::ExponentialCycle => "-",
    };
    format!(
        "{}{}{}",
        render_f64(channel.frequency_hz),
        separator,
        render_f64(channel.frequency2_hz.unwrap_or(channel.frequency_hz))
    )
}

#[cfg(test)]
mod tests {
    use super::parse_synth;
    use crate::{
        EffectCommand, EffectCommandParseError, EffectError, Synth, SynthChannel, SynthCombineMode,
        SynthLength, SynthSweep, SynthVariableDelay, SynthWaveform,
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
    fn parses_noise_sweep_and_combine_modes() {
        assert_eq!(
            parse_synth("synth", &["noise"]).unwrap().render_tokens(),
            ["synth", "whitenoise", "440"]
        );
        assert_eq!(
            parse_synth("synth", &["1", "sine", "440:880"])
                .unwrap()
                .render_tokens(),
            ["synth", "1", "sine", "440:880"]
        );
        let expected = Synth::with_channels(
            Some(SynthLength::seconds(1.0).unwrap()),
            [SynthChannel::new(SynthWaveform::Sine, 440.0)
                .unwrap()
                .with_sweep(SynthSweep::Linear, 880.0)
                .unwrap()
                .with_combine(SynthCombineMode::Fmod)],
        )
        .unwrap();
        assert_eq!(
            parse_synth("synth", &["1", "sine", "fmod", "440:880"]).unwrap(),
            EffectCommand::Synth(expected)
        );
        assert_eq!(
            parse_synth("synth", &["sine", "vdelay", "10,2,25"])
                .unwrap()
                .render_tokens(),
            ["synth", "sine", "vdelay", "10,2,25", "440"]
        );
        assert_eq!(
            SynthVariableDelay::new(10.0, 2.0, 0.25).unwrap(),
            SynthVariableDelay {
                fixed_ms: 10.0,
                extra_ms: 2.0,
                mix: 0.25
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

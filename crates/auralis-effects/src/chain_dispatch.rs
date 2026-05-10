use auralis_core::AudioBuffer;
use auralis_simd::BackendKind;

use crate::{
    EffectCommand, EffectError, EffectKind,
    chain::{is_chain_boundary_token, is_effect_boundary, is_unsupported_boundary_control},
    chain_gain::{GainHeadroomState, apply_gain_command},
    command::is_option_like,
};

pub(crate) fn apply_command(
    command: &EffectCommand,
    audio: &mut AudioBuffer,
    requested_backend: BackendKind,
    gain_headroom: &mut GainHeadroomState,
) -> std::result::Result<(), (&'static str, EffectError)> {
    if apply_in_place_command(command, audio, requested_backend) {
        return Ok(());
    }
    if apply_filter_command(command, audio)? {
        return Ok(());
    }
    if apply_buffer_command(command, audio, requested_backend)? {
        return Ok(());
    }

    match command {
        EffectCommand::Gain(gain) => {
            apply_gain_command(*gain, audio, requested_backend, gain_headroom)?;
            Ok(())
        }
        EffectCommand::Centercut(_)
        | EffectCommand::Channels(_)
        | EffectCommand::Chorus(_)
        | EffectCommand::Delay(_)
        | EffectCommand::Echo(_)
        | EffectCommand::Echos(_)
        | EffectCommand::Fade(_)
        | EffectCommand::Norm(_)
        | EffectCommand::Oops(_)
        | EffectCommand::Pad(_)
        | EffectCommand::Repeat(_)
        | EffectCommand::Remix(_)
        | EffectCommand::Trim(_) => unreachable!("buffer commands returned early"),
        EffectCommand::Biquad(_)
        | EffectCommand::Contrast(_)
        | EffectCommand::DcShift(_)
        | EffectCommand::Overdrive(_)
        | EffectCommand::Reverse(_)
        | EffectCommand::Saturation(_)
        | EffectCommand::SoftVol(_)
        | EffectCommand::Swap(_)
        | EffectCommand::Tremolo(_)
        | EffectCommand::Vol(_) => unreachable!("in-place commands returned early"),
        EffectCommand::AllPass(_)
        | EffectCommand::Band(_)
        | EffectCommand::BandPass(_)
        | EffectCommand::BandReject(_)
        | EffectCommand::Bass(_)
        | EffectCommand::Deemph(_)
        | EffectCommand::Equalizer(_)
        | EffectCommand::HighPass(_)
        | EffectCommand::LowPass(_)
        | EffectCommand::Riaa(_)
        | EffectCommand::Treble(_) => unreachable!("filter commands returned early"),
    }
}

fn apply_buffer_command(
    command: &EffectCommand,
    audio: &mut AudioBuffer,
    requested_backend: BackendKind,
) -> std::result::Result<bool, (&'static str, EffectError)> {
    match command {
        EffectCommand::Centercut(centercut) => apply_centercut_command(*centercut, audio)?,
        EffectCommand::Channels(channels) => {
            *audio = channels
                .process_buffer_with_backend(audio, requested_backend)
                .map_err(|source| ("channels", source))?;
        }
        EffectCommand::Chorus(chorus) => {
            *audio = chorus
                .process_buffer(audio)
                .map_err(|source| ("stage", source))?;
        }
        EffectCommand::Delay(delay) => {
            *audio = delay
                .process_buffer(audio)
                .map_err(|source| ("position", source))?;
        }
        EffectCommand::Echo(echo) => {
            *audio = echo
                .process_buffer(audio)
                .map_err(|source| ("delay-decay-pair", source))?;
        }
        EffectCommand::Echos(echos) => {
            *audio = echos
                .process_buffer(audio)
                .map_err(|source| ("delay-decay-pair", source))?;
        }
        EffectCommand::Fade(fade) => {
            if fade.stop_position.is_some() {
                *audio = fade
                    .process_buffer_to_output(audio, requested_backend)
                    .map_err(|source| ("frame-count", source))?;
            } else {
                fade.process_buffer_with_backend(audio, requested_backend);
            }
        }
        EffectCommand::Norm(norm) => norm
            .process_buffer_with_backend(audio, requested_backend)
            .map_err(|source| ("level", source))?,
        EffectCommand::Oops(oops) => {
            *audio = oops
                .process_buffer(audio)
                .map_err(|source| ("channels", source))?;
        }
        EffectCommand::Pad(pad) => {
            *audio = pad
                .process_buffer(audio)
                .map_err(|source| ("frame-count", source))?;
        }
        EffectCommand::Repeat(repeat) => {
            *audio = repeat
                .process_buffer(audio)
                .map_err(|source| ("count", source))?;
        }
        EffectCommand::Remix(remix) => {
            *audio = remix
                .process_buffer(audio)
                .map_err(|source| ("out-spec", source))?;
        }
        EffectCommand::Trim(trim) => {
            *audio = trim
                .process_buffer(audio)
                .map_err(|source| ("frame-range", source))?;
        }
        _ => return Ok(false),
    }

    Ok(true)
}

fn apply_filter_command(
    command: &EffectCommand,
    audio: &mut AudioBuffer,
) -> std::result::Result<bool, (&'static str, EffectError)> {
    match command {
        EffectCommand::AllPass(all_pass) => all_pass.process_buffer(audio),
        EffectCommand::Band(band) => band.process_buffer(audio),
        EffectCommand::BandPass(band_pass) => band_pass.process_buffer(audio),
        EffectCommand::BandReject(band_reject) => band_reject.process_buffer(audio),
        EffectCommand::Bass(bass) => bass.process_buffer(audio),
        EffectCommand::Deemph(deemph) => deemph.process_buffer(audio),
        EffectCommand::Equalizer(equalizer) => equalizer.process_buffer(audio),
        EffectCommand::HighPass(high_pass) => high_pass.process_buffer(audio),
        EffectCommand::LowPass(low_pass) => low_pass.process_buffer(audio),
        EffectCommand::Riaa(riaa) => riaa.process_buffer(audio),
        EffectCommand::Treble(treble) => treble.process_buffer(audio),
        _ => return Ok(false),
    }
    .map_err(|source| ("filter-design", source))?;

    Ok(true)
}

fn apply_in_place_command(
    command: &EffectCommand,
    audio: &mut AudioBuffer,
    requested_backend: BackendKind,
) -> bool {
    match command {
        EffectCommand::Biquad(biquad) => biquad.process_buffer(audio),
        EffectCommand::Contrast(contrast) => contrast.process_buffer(audio),
        EffectCommand::DcShift(dc_shift) => {
            dc_shift.process_buffer_with_backend(audio, requested_backend);
        }
        EffectCommand::Overdrive(overdrive) => overdrive.process_buffer(audio),
        EffectCommand::Reverse(reverse) => reverse.process_buffer(audio),
        EffectCommand::Saturation(saturation) => saturation.process_buffer(audio),
        EffectCommand::SoftVol(softvol) => softvol.process_buffer(audio),
        EffectCommand::Swap(swap) => swap.process_buffer(audio),
        EffectCommand::Tremolo(tremolo) => tremolo.process_buffer(audio),
        EffectCommand::Vol(vol) => {
            vol.process_buffer_with_backend(audio, requested_backend);
        }
        _ => return false,
    }

    true
}

fn apply_centercut_command(
    centercut: crate::Centercut,
    audio: &mut AudioBuffer,
) -> std::result::Result<(), (&'static str, EffectError)> {
    let separated = centercut
        .process_buffer(audio)
        .map_err(|source| ("channels", source))?;
    *audio = separated;

    Ok(())
}

pub(crate) fn command_end(kind: EffectKind, tokens: &[&str], command_start: usize) -> usize {
    let args_start = command_start + 1;

    match kind {
        EffectKind::Biquad => optional_arg_end(tokens, args_start, 6),
        EffectKind::Centercut => centercut_arg_end(tokens, args_start),
        EffectKind::Channels => optional_arg_end(tokens, args_start, 1),
        EffectKind::Chorus => chorus_arg_end(tokens, args_start),
        EffectKind::Fade => fade_arg_end(tokens, args_start),
        EffectKind::Gain => gain_arg_end(tokens, args_start),
        EffectKind::Contrast | EffectKind::Norm | EffectKind::Repeat => {
            optional_arg_end(tokens, args_start, 1)
        }
        EffectKind::DcShift | EffectKind::Overdrive | EffectKind::Tremolo => {
            optional_arg_end(tokens, args_start, 2)
        }
        EffectKind::Delay => delay_arg_end(tokens, args_start),
        EffectKind::Echo | EffectKind::Echos => echo_arg_end(tokens, args_start),
        EffectKind::Pad => pad_arg_end(tokens, args_start),
        EffectKind::Remix => remix_arg_end(tokens, args_start),
        EffectKind::Deemph
        | EffectKind::Oops
        | EffectKind::Reverse
        | EffectKind::Riaa
        | EffectKind::Swap => no_arg_end(tokens, args_start),
        EffectKind::Saturation => optional_arg_end(tokens, args_start, 4),
        EffectKind::Trim => trim_arg_end(tokens, args_start),
        EffectKind::AllPass
        | EffectKind::Band
        | EffectKind::BandPass
        | EffectKind::BandReject
        | EffectKind::Bass
        | EffectKind::Equalizer
        | EffectKind::HighPass
        | EffectKind::LowPass
        | EffectKind::Treble
        | EffectKind::SoftVol
        | EffectKind::Vol => optional_arg_end(tokens, args_start, 3),
    }
}

fn centercut_arg_end(tokens: &[&str], args_start: usize) -> usize {
    let mut end = args_start;

    while end < tokens.len() && !is_command_boundary(tokens[end]) {
        let token = tokens[end];
        end += 1;
        if matches!(token, "-a" | "-w") && end < tokens.len() && !is_command_boundary(tokens[end]) {
            end += 1;
        }
    }

    end
}

fn trim_arg_end(tokens: &[&str], args_start: usize) -> usize {
    let mut end = args_start;
    while end < tokens.len() && !is_command_boundary(tokens[end]) {
        end += 1;
    }
    end
}

fn delay_arg_end(tokens: &[&str], args_start: usize) -> usize {
    let mut end = args_start;
    while end < tokens.len() && !is_command_boundary(tokens[end]) {
        end += 1;
    }
    end
}

fn echo_arg_end(tokens: &[&str], args_start: usize) -> usize {
    let mut end = args_start;
    while end < tokens.len() && !is_command_boundary(tokens[end]) {
        end += 1;
    }
    end
}

fn chorus_arg_end(tokens: &[&str], args_start: usize) -> usize {
    let mut end = args_start;
    while end < tokens.len() && !is_command_boundary(tokens[end]) {
        end += 1;
    }
    end
}

fn gain_arg_end(tokens: &[&str], args_start: usize) -> usize {
    let mut end = args_start;
    while end < tokens.len() && !is_command_boundary(tokens[end]) && is_option_like(tokens[end]) {
        end += 1;
    }
    if end < tokens.len() && !is_command_boundary(tokens[end]) {
        end += 1;
    }
    include_unexpected_argument(tokens, end)
}

fn optional_arg_end(tokens: &[&str], args_start: usize, max: usize) -> usize {
    let mut end = args_start;
    let mut consumed = 0;

    while consumed < max && end < tokens.len() && !is_command_boundary(tokens[end]) {
        consumed += 1;
        end += 1;
    }

    include_unexpected_argument(tokens, end)
}

fn pad_arg_end(tokens: &[&str], args_start: usize) -> usize {
    let mut end = args_start;

    while end < tokens.len() && !is_command_boundary(tokens[end]) {
        end += 1;
    }

    end
}

fn remix_arg_end(tokens: &[&str], args_start: usize) -> usize {
    pad_arg_end(tokens, args_start)
}

fn no_arg_end(tokens: &[&str], args_start: usize) -> usize {
    include_unexpected_argument(tokens, args_start)
}

fn fade_arg_end(tokens: &[&str], args_start: usize) -> usize {
    let mut end = args_start;

    if matches!(tokens.get(end).copied(), Some("l" | "q" | "h" | "t" | "p")) {
        end += 1;
    }

    if end >= tokens.len() || is_command_boundary(tokens[end]) {
        return end;
    }
    end += 1;

    if end < tokens.len() && !is_command_boundary(tokens[end]) {
        end += 1;
    }

    if end < tokens.len() && !is_command_boundary(tokens[end]) {
        end += 1;
    }

    include_unexpected_argument(tokens, end)
}

fn include_unexpected_argument(tokens: &[&str], end: usize) -> usize {
    if end < tokens.len() && !is_command_boundary(tokens[end]) {
        end + 1
    } else {
        end
    }
}

fn is_command_boundary(token: &str) -> bool {
    is_chain_boundary_token(token)
        || is_unsupported_boundary_control(token)
        || is_effect_boundary(token)
}

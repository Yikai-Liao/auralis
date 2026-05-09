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

    match command {
        EffectCommand::AllPass(all_pass) => all_pass
            .process_buffer(audio)
            .map_err(|source| ("filter-design", source)),
        EffectCommand::Band(band) => band
            .process_buffer(audio)
            .map_err(|source| ("filter-design", source)),
        EffectCommand::BandPass(band_pass) => band_pass
            .process_buffer(audio)
            .map_err(|source| ("filter-design", source)),
        EffectCommand::BandReject(band_reject) => band_reject
            .process_buffer(audio)
            .map_err(|source| ("filter-design", source)),
        EffectCommand::Bass(bass) => bass
            .process_buffer(audio)
            .map_err(|source| ("filter-design", source)),
        EffectCommand::Equalizer(equalizer) => equalizer
            .process_buffer(audio)
            .map_err(|source| ("filter-design", source)),
        EffectCommand::Treble(treble) => treble
            .process_buffer(audio)
            .map_err(|source| ("filter-design", source)),
        EffectCommand::Centercut(centercut) => apply_centercut_command(*centercut, audio),
        EffectCommand::Channels(channels) => {
            let converted = channels
                .process_buffer_with_backend(audio, requested_backend)
                .map_err(|source| ("channels", source))?;
            *audio = converted;
            Ok(())
        }
        EffectCommand::Fade(fade) => {
            if fade.stop_position.is_some() {
                let faded = fade
                    .process_buffer_to_output(audio, requested_backend)
                    .map_err(|source| ("frame-count", source))?;
                *audio = faded;
            } else {
                fade.process_buffer_with_backend(audio, requested_backend);
            }
            Ok(())
        }
        EffectCommand::Gain(gain) => {
            apply_gain_command(*gain, audio, requested_backend, gain_headroom)?;
            Ok(())
        }
        EffectCommand::Norm(norm) => norm
            .process_buffer_with_backend(audio, requested_backend)
            .map_err(|source| ("level", source)),
        EffectCommand::Oops(oops) => {
            let extracted = oops
                .process_buffer(audio)
                .map_err(|source| ("channels", source))?;
            *audio = extracted;
            Ok(())
        }
        EffectCommand::Pad(pad) => {
            let padded = pad
                .process_buffer(audio)
                .map_err(|source| ("frame-count", source))?;
            *audio = padded;
            Ok(())
        }
        EffectCommand::Repeat(repeat) => {
            let repeated = repeat
                .process_buffer(audio)
                .map_err(|source| ("count", source))?;
            *audio = repeated;
            Ok(())
        }
        EffectCommand::Remix(remix) => {
            let remixed = remix
                .process_buffer(audio)
                .map_err(|source| ("out-spec", source))?;
            *audio = remixed;
            Ok(())
        }
        EffectCommand::Trim(trim) => {
            let trimmed = trim
                .process_buffer(audio)
                .map_err(|source| ("frame-range", source))?;
            *audio = trimmed;
            Ok(())
        }
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
    }
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
        EffectKind::Fade => fade_arg_end(tokens, args_start),
        EffectKind::Gain => gain_arg_end(tokens, args_start),
        EffectKind::Contrast | EffectKind::Norm | EffectKind::Repeat => {
            optional_arg_end(tokens, args_start, 1)
        }
        EffectKind::DcShift | EffectKind::Overdrive | EffectKind::Tremolo => {
            optional_arg_end(tokens, args_start, 2)
        }
        EffectKind::Pad => pad_arg_end(tokens, args_start),
        EffectKind::Remix => remix_arg_end(tokens, args_start),
        EffectKind::Oops | EffectKind::Reverse | EffectKind::Swap => no_arg_end(tokens, args_start),
        EffectKind::Saturation => optional_arg_end(tokens, args_start, 4),
        EffectKind::Trim => trim_arg_end(tokens, args_start),
        EffectKind::AllPass
        | EffectKind::Band
        | EffectKind::BandPass
        | EffectKind::BandReject
        | EffectKind::Bass
        | EffectKind::Equalizer
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

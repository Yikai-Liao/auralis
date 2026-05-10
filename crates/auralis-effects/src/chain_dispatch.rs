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
        | EffectCommand::Bend(_)
        | EffectCommand::Channels(_)
        | EffectCommand::Chorus(_)
        | EffectCommand::Compand(_)
        | EffectCommand::Delay(_)
        | EffectCommand::Downsample(_)
        | EffectCommand::Echo(_)
        | EffectCommand::Echos(_)
        | EffectCommand::Fade(_)
        | EffectCommand::Fir(_)
        | EffectCommand::FirFit(_)
        | EffectCommand::Flanger(_)
        | EffectCommand::Loudness(_)
        | EffectCommand::MCompand(_)
        | EffectCommand::NoiseProf(_)
        | EffectCommand::NoiseRed(_)
        | EffectCommand::Norm(_)
        | EffectCommand::Oops(_)
        | EffectCommand::Pad(_)
        | EffectCommand::Phaser(_)
        | EffectCommand::Pitch(_)
        | EffectCommand::Rate(_)
        | EffectCommand::Reverb(_)
        | EffectCommand::Repeat(_)
        | EffectCommand::Remix(_)
        | EffectCommand::Silence(_)
        | EffectCommand::Speed(_)
        | EffectCommand::Splice(_)
        | EffectCommand::Stretch(_)
        | EffectCommand::Tempo(_)
        | EffectCommand::Trim(_)
        | EffectCommand::Upsample(_)
        | EffectCommand::Vad(_) => unreachable!("buffer commands returned early"),
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

#[allow(
    clippy::too_many_lines,
    reason = "central effect-chain dispatch keeps one explicit arm per buffer-level command"
)]
fn apply_buffer_command(
    command: &EffectCommand,
    audio: &mut AudioBuffer,
    requested_backend: BackendKind,
) -> std::result::Result<bool, (&'static str, EffectError)> {
    if apply_sample_rate_command(command, audio)? {
        return Ok(true);
    }

    match command {
        EffectCommand::Bend(bend) => {
            *audio = bend
                .process_buffer(audio)
                .map_err(|source| ("bend", source))?;
        }
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
        EffectCommand::Compand(processor) => {
            processor
                .process_buffer(audio)
                .map_err(|source| ("compand", source))?;
        }
        EffectCommand::MCompand(processor) => {
            processor
                .process_buffer(audio)
                .map_err(|source| ("mcompand", source))?;
        }
        EffectCommand::NoiseProf(noiseprof) => noiseprof.process_buffer(audio),
        EffectCommand::NoiseRed(noisered) => {
            *audio = noisered
                .process_buffer(audio)
                .map_err(|source| ("noisered", source))?;
        }
        EffectCommand::Loudness(loudness) => {
            loudness
                .process_buffer(audio)
                .map_err(|source| ("loudness", source))?;
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
        EffectCommand::Fir(fir) => {
            *audio = fir
                .process_buffer(audio)
                .map_err(|source| ("coefficients", source))?;
        }
        EffectCommand::FirFit(firfit) => {
            *audio = firfit
                .process_buffer(audio)
                .map_err(|source| ("knots", source))?;
        }
        EffectCommand::Flanger(flanger) => {
            *audio = flanger
                .process_buffer(audio)
                .map_err(|source| ("flanger", source))?;
        }
        EffectCommand::Phaser(phaser) => {
            *audio = phaser
                .process_buffer(audio)
                .map_err(|source| ("phaser", source))?;
        }
        EffectCommand::Pitch(pitch) => {
            *audio = pitch
                .process_buffer(audio)
                .map_err(|source| ("pitch", source))?;
        }
        EffectCommand::Reverb(reverb) => {
            *audio = reverb
                .process_buffer(audio)
                .map_err(|source| ("reverb", source))?;
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
        EffectCommand::Silence(silence) => {
            *audio = silence
                .process_buffer(audio)
                .map_err(|source| ("silence", source))?;
        }
        EffectCommand::Splice(splice) => {
            *audio = splice
                .process_buffer(audio)
                .map_err(|source| ("splice", source))?;
        }
        EffectCommand::Stretch(stretch) => {
            *audio = stretch
                .process_buffer(audio)
                .map_err(|source| ("stretch", source))?;
        }
        EffectCommand::Tempo(tempo) => {
            *audio = tempo
                .process_buffer(audio)
                .map_err(|source| ("factor", source))?;
        }
        EffectCommand::Trim(trim) => {
            *audio = trim
                .process_buffer(audio)
                .map_err(|source| ("frame-range", source))?;
        }
        EffectCommand::Vad(vad) => {
            *audio = vad
                .process_buffer(audio)
                .map_err(|source| ("vad", source))?;
        }
        _ => return Ok(false),
    }

    Ok(true)
}

fn apply_sample_rate_command(
    command: &EffectCommand,
    audio: &mut AudioBuffer,
) -> std::result::Result<bool, (&'static str, EffectError)> {
    match command {
        EffectCommand::Downsample(downsample) => {
            *audio = downsample
                .process_buffer(audio)
                .map_err(|source| ("factor", source))?;
        }
        EffectCommand::Rate(rate) => {
            *audio = rate
                .process_buffer(audio)
                .map_err(|source| ("frequency", source))?;
        }
        EffectCommand::Speed(speed) => {
            *audio = speed
                .process_buffer(audio)
                .map_err(|source| ("factor", source))?;
        }
        EffectCommand::Upsample(upsample) => {
            *audio = upsample
                .process_buffer(audio)
                .map_err(|source| ("factor", source))?;
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
        EffectKind::Bend => bend_arg_end(tokens, args_start),
        EffectKind::Centercut => centercut_arg_end(tokens, args_start),
        EffectKind::Chorus => chorus_arg_end(tokens, args_start),
        EffectKind::Fade => fade_arg_end(tokens, args_start),
        EffectKind::Fir => fir_arg_end(tokens, args_start),
        EffectKind::FirFit => firfit_arg_end(tokens, args_start),
        EffectKind::Flanger => flanger_arg_end(tokens, args_start),
        EffectKind::Phaser => phaser_arg_end(tokens, args_start),
        EffectKind::Pitch => pitch_arg_end(tokens, args_start),
        EffectKind::Reverb => reverb_arg_end(tokens, args_start),
        EffectKind::Gain => gain_arg_end(tokens, args_start),
        EffectKind::Contrast
        | EffectKind::Channels
        | EffectKind::Downsample
        | EffectKind::NoiseProf
        | EffectKind::Norm
        | EffectKind::Repeat
        | EffectKind::Speed
        | EffectKind::Upsample => optional_arg_end(tokens, args_start, 1),
        EffectKind::NoiseRed => optional_arg_end(tokens, args_start, 2),
        EffectKind::Splice => splice_arg_end(tokens, args_start),
        EffectKind::Tempo => tempo_arg_end(tokens, args_start),
        EffectKind::Compand | EffectKind::Stretch => optional_arg_end(tokens, args_start, 5),
        EffectKind::MCompand => mcompand_arg_end(tokens, args_start),
        EffectKind::Rate => rate_arg_end(tokens, args_start),
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
        EffectKind::Silence => silence_arg_end(tokens, args_start),
        EffectKind::Trim => trim_arg_end(tokens, args_start),
        EffectKind::Vad => vad_arg_end(tokens, args_start),
        EffectKind::AllPass
        | EffectKind::Band
        | EffectKind::BandPass
        | EffectKind::BandReject
        | EffectKind::Bass
        | EffectKind::Equalizer
        | EffectKind::HighPass
        | EffectKind::Loudness
        | EffectKind::LowPass
        | EffectKind::Treble
        | EffectKind::SoftVol
        | EffectKind::Vol => optional_arg_end(tokens, args_start, 3),
    }
}

fn vad_arg_end(tokens: &[&str], args_start: usize) -> usize {
    let mut end = args_start;
    while end < tokens.len() && !is_command_boundary(tokens[end]) {
        if is_option_like(tokens[end]) && end + 1 < tokens.len() {
            end += 2;
        } else {
            end += 1;
        }
    }
    end
}

fn mcompand_arg_end(tokens: &[&str], args_start: usize) -> usize {
    let mut end = args_start;
    while end < tokens.len() && !is_command_boundary(tokens[end]) {
        end += 1;
    }

    end
}

fn tempo_arg_end(tokens: &[&str], args_start: usize) -> usize {
    let mut end = args_start;

    while end < tokens.len() && !is_command_boundary(tokens[end]) {
        match tokens[end] {
            "-q" | "-m" | "-s" | "-l" => end += 1,
            token if is_option_like(token) => {
                end += 1;
                if end < tokens.len() && !is_command_boundary(tokens[end]) {
                    end += 1;
                }
                return include_unexpected_argument(tokens, end);
            }
            _ => break,
        }
    }

    optional_arg_end(tokens, end, 4)
}

fn pitch_arg_end(tokens: &[&str], args_start: usize) -> usize {
    let mut end = args_start;

    while end < tokens.len() && !is_command_boundary(tokens[end]) {
        match tokens[end] {
            "-q" => end += 1,
            token if is_option_like(token) => {
                end += 1;
                if end < tokens.len() && !is_command_boundary(tokens[end]) {
                    end += 1;
                }
                return include_unexpected_argument(tokens, end);
            }
            _ => break,
        }
    }

    optional_arg_end(tokens, end, 4)
}

fn splice_arg_end(tokens: &[&str], args_start: usize) -> usize {
    let mut end = args_start;

    if end < tokens.len() && matches!(tokens[end], "-h" | "-t" | "-q") {
        end += 1;
    } else if end < tokens.len() && is_option_like(tokens[end]) {
        end += 1;
        return include_unexpected_argument(tokens, end);
    }

    while end < tokens.len() && !is_command_boundary(tokens[end]) {
        end += 1;
    }

    end
}

fn bend_arg_end(tokens: &[&str], args_start: usize) -> usize {
    let mut end = args_start;

    while end < tokens.len() && !is_command_boundary(tokens[end]) {
        match tokens[end] {
            "-f" | "-o" => {
                end += 1;
                if end < tokens.len() && !is_command_boundary(tokens[end]) {
                    end += 1;
                }
            }
            token if is_option_like(token) => {
                end += 1;
                if end < tokens.len() && !is_command_boundary(tokens[end]) {
                    end += 1;
                }
                return include_unexpected_argument(tokens, end);
            }
            _ => break,
        }
    }

    while end < tokens.len() && !is_command_boundary(tokens[end]) {
        end += 1;
    }

    end
}

fn rate_arg_end(tokens: &[&str], args_start: usize) -> usize {
    let mut end = args_start;

    while end < tokens.len() && !is_command_boundary(tokens[end]) {
        match tokens[end] {
            "-q" | "-l" | "-m" | "-g" | "-h" | "-e" | "-v" | "-u" | "-f" | "-n" | "-t" | "-M"
            | "-I" | "-L" | "-s" | "-a" => {
                end += 1;
            }
            "-Q" | "-i" | "-c" | "-p" | "-b" | "-B" | "-A" | "-d" | "-R" => {
                end += 1;
                if end < tokens.len() && !is_command_boundary(tokens[end]) {
                    end += 1;
                }
            }
            token if is_option_like(token) => {
                end += 1;
                if end < tokens.len() && !is_command_boundary(tokens[end]) {
                    end += 1;
                }
                return include_unexpected_argument(tokens, end);
            }
            _ => break,
        }
    }

    if end < tokens.len() && !is_command_boundary(tokens[end]) {
        end += 1;
    }

    include_unexpected_argument(tokens, end)
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

fn flanger_arg_end(tokens: &[&str], args_start: usize) -> usize {
    chorus_arg_end(tokens, args_start)
}

fn phaser_arg_end(tokens: &[&str], args_start: usize) -> usize {
    chorus_arg_end(tokens, args_start)
}

fn reverb_arg_end(tokens: &[&str], args_start: usize) -> usize {
    let mut end = args_start;
    if matches!(tokens.get(end).copied(), Some("-w" | "--wet-only")) {
        end += 1;
    }

    optional_arg_end(tokens, end, 6)
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

fn silence_arg_end(tokens: &[&str], args_start: usize) -> usize {
    let mut end = args_start;
    if matches!(tokens.get(end).copied(), Some("-l")) {
        end += 1;
    } else if tokens.get(end).is_some_and(|token| is_option_like(token)) {
        end += 1;
        return include_unexpected_argument(tokens, end);
    }
    if end >= tokens.len() || is_command_boundary(tokens[end]) {
        return end;
    }
    let above_periods = tokens[end].parse::<u32>().ok();
    end += 1;
    if above_periods.is_some_and(|periods| periods > 0) {
        end = end.saturating_add(2).min(tokens.len());
    }
    if end < tokens.len() && !is_command_boundary(tokens[end]) {
        end = end.saturating_add(3).min(tokens.len());
    }
    include_unexpected_argument(tokens, end)
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

fn fir_arg_end(tokens: &[&str], args_start: usize) -> usize {
    let mut end = args_start;
    while end < tokens.len() && !is_command_boundary(tokens[end]) {
        end += 1;
    }
    end
}

fn firfit_arg_end(tokens: &[&str], args_start: usize) -> usize {
    fir_arg_end(tokens, args_start)
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

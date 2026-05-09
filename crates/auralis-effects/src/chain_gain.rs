use auralis_core::AudioBuffer;
use auralis_dsp::linear_gain;
use auralis_simd::{BackendKind, gain_f32_in_place_with_backend, select_backend};

use crate::{EffectError, Gain, GainChannelMode};

#[derive(Debug, Default, Clone, Copy, PartialEq)]
pub(crate) struct GainHeadroomState {
    multiplier: Option<f32>,
}

pub(crate) fn apply_gain_command(
    gain: Gain,
    audio: &mut AudioBuffer,
    requested_backend: BackendKind,
    gain_headroom: &mut GainHeadroomState,
) -> std::result::Result<(), (&'static str, EffectError)> {
    let fixed_multiplier = linear_gain(gain.db);
    let mut multiplier = fixed_multiplier;
    if gain.normalize {
        multiplier *= peak_reclaim_multiplier(audio).map_err(|source| ("normalize", source))?;
    }
    if gain.reclaims_headroom() {
        let Some(reclaim) = gain_headroom
            .multiplier
            .filter(|&multiplier| multiplier < 1.0)
        else {
            return Err(("headroom", EffectError::MissingGainHeadroom));
        };
        multiplier *= peak_reclaim_multiplier(audio)
            .map_err(|source| ("headroom", source))?
            .min(1.0 / reclaim);
    }
    if gain.channel_mode != GainChannelMode::None {
        apply_channel_mode(gain, audio, requested_backend, multiplier)
            .map_err(|source| ("channel", source))?;
        gain_headroom.multiplier = gain.reserves_headroom().then_some(linear_gain(gain.db));
        return Ok(());
    }
    if gain.limiter {
        apply_simple_limiter(audio.as_planar_f32_mut(), multiplier, fixed_multiplier);
    } else {
        gain_f32_in_place_with_backend(
            select_backend(requested_backend),
            audio.as_planar_f32_mut(),
            multiplier,
        );
    }
    gain_headroom.multiplier = gain.reserves_headroom().then_some(linear_gain(gain.db));
    Ok(())
}

fn peak_reclaim_multiplier(audio: &AudioBuffer) -> std::result::Result<f32, EffectError> {
    let mut peak = 0.0_f32;
    for (sample_index, &sample) in audio.as_planar_f32().iter().enumerate() {
        if !sample.is_finite() {
            return Err(EffectError::NonFiniteGainSample { sample_index });
        }
        peak = peak.max(sample.abs());
    }
    Ok(if peak == 0.0 { 1.0 } else { 1.0 / peak })
}

fn apply_channel_mode(
    gain: Gain,
    audio: &mut AudioBuffer,
    requested_backend: BackendKind,
    fixed_multiplier: f32,
) -> std::result::Result<(), EffectError> {
    if gain.normalize && gain.channel_mode == GainChannelMode::Equalize {
        let multiplier = fixed_multiplier * peak_reclaim_multiplier(audio)?;
        gain_f32_in_place_with_backend(
            select_backend(requested_backend),
            audio.as_planar_f32_mut(),
            multiplier,
        );
        return Ok(());
    }

    let stats = channel_stats(audio)?;
    let mut multipliers = match gain.channel_mode {
        GainChannelMode::None => return Ok(()),
        GainChannelMode::Equalize => equalize_multipliers(&stats, fixed_multiplier),
        GainChannelMode::Balance | GainChannelMode::BalanceNoClip => balance_multipliers(&stats),
    };

    if matches!(
        gain.channel_mode,
        GainChannelMode::Balance | GainChannelMode::BalanceNoClip
    ) {
        let max_peak = stats
            .iter()
            .zip(&multipliers)
            .map(|(stats, &multiplier)| stats.peak * multiplier.abs())
            .fold(0.0_f32, f32::max);
        for multiplier in &mut multipliers {
            *multiplier *= fixed_multiplier;
        }
        let should_attenuate = (gain.normalize && max_peak > 0.0)
            || (gain.channel_mode == GainChannelMode::BalanceNoClip && max_peak > 1.0);
        if should_attenuate {
            for multiplier in &mut multipliers {
                *multiplier /= max_peak;
            }
        }
    }

    let backend = select_backend(requested_backend);
    for (channel_index, multiplier) in multipliers.into_iter().enumerate() {
        let Some(channel) = audio.channel_mut(channel_index) else {
            continue;
        };
        gain_f32_in_place_with_backend(backend, channel, multiplier);
    }
    Ok(())
}

#[derive(Debug, Clone, Copy)]
struct ChannelStats {
    peak: f32,
    rms: f32,
}

fn channel_stats(audio: &AudioBuffer) -> std::result::Result<Vec<ChannelStats>, EffectError> {
    let mut stats = Vec::with_capacity(audio.channels().as_usize());
    for channel_index in 0..audio.channels().as_usize() {
        let channel = audio.channel(channel_index).unwrap_or_default();
        let mut peak = 0.0_f32;
        let mut sum_squares = 0.0_f32;
        let mut sample_count = 0.0_f32;
        for (frame_index, &sample) in channel.iter().enumerate() {
            if !sample.is_finite() {
                return Err(EffectError::NonFiniteGainSample {
                    sample_index: channel_index * channel.len() + frame_index,
                });
            }
            peak = peak.max(sample.abs());
            sum_squares += sample * sample;
            sample_count += 1.0;
        }
        let rms = if sample_count == 0.0 {
            0.0
        } else {
            (sum_squares / sample_count).sqrt()
        };
        stats.push(ChannelStats { peak, rms });
    }
    Ok(stats)
}

fn equalize_multipliers(stats: &[ChannelStats], fixed_multiplier: f32) -> Vec<f32> {
    let max_peak = stats.iter().map(|stats| stats.peak).fold(0.0_f32, f32::max);
    stats
        .iter()
        .map(|stats| {
            if stats.peak == 0.0 {
                fixed_multiplier
            } else {
                fixed_multiplier * max_peak / stats.peak
            }
        })
        .collect()
}

fn balance_multipliers(stats: &[ChannelStats]) -> Vec<f32> {
    let max_rms = stats.iter().map(|stats| stats.rms).fold(0.0_f32, f32::max);
    stats
        .iter()
        .map(|stats| {
            if stats.rms == 0.0 {
                1.0
            } else {
                max_rms / stats.rms
            }
        })
        .collect()
}

fn apply_simple_limiter(samples: &mut [f32], multiplier: f32, fixed_multiplier: f32) {
    let limiter = 1.0 - fixed_multiplier.recip();
    for sample in samples {
        let scaled = *sample * multiplier;
        *sample = if scaled == 0.0 {
            0.0
        } else {
            scaled / (1.0 + limiter * scaled.abs())
        };
    }
}

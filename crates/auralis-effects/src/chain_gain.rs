use auralis_core::AudioBuffer;
use auralis_dsp::linear_gain;
use auralis_simd::{BackendKind, gain_f32_in_place_with_backend, select_backend};

use crate::{EffectError, Gain};

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

use auralis_core::AudioBuffer;

use crate::{EffectError, Result};

const SAFETY_FACTOR: f32 = 0.9999;

/// Saturation transfer family.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum SaturationType {
    /// Hyperbolic tangent saturation using a drive multiplier.
    Tanh,

    /// Square-root saturation using a color mix.
    Sqrt,

    /// Diode-style hard threshold saturation.
    Diode,
}

/// SoX-ng-style saturation distortion.
///
/// `Saturation` applies one of SoX-ng's `tanh`, `sqrt`, or `diode` saturation
/// transfer functions, recenters the wet signal so zero input produces zero
/// wet output, applies SoX-ng's safety gain compensation, and mixes the wet
/// path with the dry input according to `blend`. The `offset` parameter adds a
/// positive DC offset before shaping for asymmetric distortion.
///
/// The operation is deterministic and streaming-safe because each sample is
/// transformed independently. It currently uses scalar `tanh`/`sqrt`
/// evaluation because there is no SIMD kernel for this nonlinear processor.
///
/// # Examples
///
/// ```
/// use auralis_effects::{Saturation, SaturationType};
///
/// let mut samples = [0.0, 0.5, -0.5];
/// Saturation::new(SaturationType::Tanh, 1.0, 0.0, 1.0)?.process_samples(&mut samples);
///
/// assert_eq!(samples[0], 0.0);
/// # Ok::<(), auralis_effects::EffectError>(())
/// ```
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Saturation {
    /// Saturation transfer family.
    pub saturation_type: SaturationType,

    /// Wet/dry mix in SoX-ng's `0..=1` range.
    pub blend: f32,

    /// Positive pre-shaper DC offset in SoX-ng's `0..=1` range.
    pub offset: f32,

    /// Type-specific parameter: `drive`, `color`, or `threshold`.
    pub parameter: f32,

    offset_out: f32,
    gain_out: f32,
}

impl Saturation {
    /// Creates a saturation processor.
    ///
    /// # Errors
    ///
    /// Returns [`EffectError::InvalidSaturation`] when `blend` or `offset` is
    /// not finite or outside `0..=1`, when `drive` for `tanh` is not finite or
    /// less than `1`, or when `color`/`threshold` for `sqrt`/`diode` is not
    /// finite or outside `0..=1`.
    pub fn new(
        saturation_type: SaturationType,
        blend: f32,
        offset: f32,
        parameter: f32,
    ) -> Result<Self> {
        if !is_unit_interval(blend) || !is_unit_interval(offset) {
            return Err(EffectError::InvalidSaturation);
        }
        match saturation_type {
            SaturationType::Tanh if !parameter.is_finite() || parameter < 1.0 => {
                return Err(EffectError::InvalidSaturation);
            }
            SaturationType::Sqrt | SaturationType::Diode if !is_unit_interval(parameter) => {
                return Err(EffectError::InvalidSaturation);
            }
            _ => {}
        }

        let mut saturation = Self {
            saturation_type,
            blend,
            offset,
            parameter,
            offset_out: 0.0,
            gain_out: 1.0,
        };
        saturation.offset_out = saturation.wet_sample_without_gain(0.0);

        let peak = saturation
            .wet_sample_without_gain(1.0)
            .max(saturation.wet_sample_without_gain(-1.0).abs());
        saturation.gain_out = if peak > 0.0 {
            SAFETY_FACTOR / peak
        } else {
            0.0
        };

        Ok(saturation)
    }

    /// Creates SoX-ng's default `saturation` processor.
    #[must_use]
    pub fn default_settings() -> Self {
        Self {
            saturation_type: SaturationType::Tanh,
            blend: 1.0,
            offset: 0.0,
            parameter: 1.0,
            offset_out: 0.0,
            gain_out: SAFETY_FACTOR / 1.0_f32.tanh(),
        }
    }

    /// Creates a `tanh` saturation processor.
    ///
    /// # Errors
    ///
    /// Returns [`EffectError::InvalidSaturation`] when `blend` or `offset` is
    /// outside `0..=1`, or when `drive` is not finite or is less than `1`.
    pub fn tanh(blend: f32, offset: f32, drive: f32) -> Result<Self> {
        Self::new(SaturationType::Tanh, blend, offset, drive)
    }

    /// Creates a `sqrt` saturation processor.
    ///
    /// # Errors
    ///
    /// Returns [`EffectError::InvalidSaturation`] when `blend`, `offset`, or
    /// `color` is not finite or outside `0..=1`.
    pub fn sqrt(blend: f32, offset: f32, color: f32) -> Result<Self> {
        Self::new(SaturationType::Sqrt, blend, offset, color)
    }

    /// Creates a `diode` saturation processor.
    ///
    /// # Errors
    ///
    /// Returns [`EffectError::InvalidSaturation`] when `blend`, `offset`, or
    /// `threshold` is not finite or outside `0..=1`.
    pub fn diode(blend: f32, offset: f32, threshold: f32) -> Result<Self> {
        Self::new(SaturationType::Diode, blend, offset, threshold)
    }

    /// Applies saturation to all samples in an audio buffer.
    pub fn process_buffer(self, audio: &mut AudioBuffer) {
        self.process_samples(audio.as_planar_f32_mut());
    }

    /// Applies saturation to a planar sample slice.
    pub fn process_samples(self, samples: &mut [f32]) {
        match self.saturation_type {
            SaturationType::Tanh => self.process_tanh_samples(samples),
            SaturationType::Sqrt => self.process_sqrt_samples(samples),
            SaturationType::Diode => self.process_diode_samples(samples),
        }
    }

    fn process_tanh_samples(self, samples: &mut [f32]) {
        let dry_blend = 1.0 - self.blend;
        for sample in samples {
            let dry = *sample;
            let shifted = dry + self.offset;
            let wet = ((self.parameter * shifted).tanh() - self.offset_out) * self.gain_out;
            *sample = wet.mul_add(self.blend, dry * dry_blend).clamp(-1.0, 1.0);
        }
    }

    fn process_sqrt_samples(self, samples: &mut [f32]) {
        let dry_blend = 1.0 - self.blend;
        let root_blend = 1.0 - self.parameter;
        for sample in samples {
            let dry = *sample;
            let shifted = dry + self.offset;
            let root = shifted.abs().sqrt();
            let signed_root = if shifted < 0.0 { -root } else { root };
            let sample_root = shifted * root;
            let shaped = signed_root.mul_add(self.parameter, sample_root * root_blend);
            let wet = (shaped - self.offset_out) * self.gain_out;
            *sample = wet.mul_add(self.blend, dry * dry_blend).clamp(-1.0, 1.0);
        }
    }

    fn process_diode_samples(self, samples: &mut [f32]) {
        let dry_blend = 1.0 - self.blend;
        for sample in samples {
            let dry = *sample;
            let shifted = dry + self.offset;
            let shaped = shifted.clamp(-self.parameter, self.parameter);
            let wet = (shaped - self.offset_out) * self.gain_out;
            *sample = wet.mul_add(self.blend, dry * dry_blend).clamp(-1.0, 1.0);
        }
    }

    #[must_use]
    fn wet_sample_without_gain(self, sample: f32) -> f32 {
        let shifted = sample + self.offset;
        let shaped = match self.saturation_type {
            SaturationType::Tanh => (self.parameter * shifted).tanh(),
            SaturationType::Sqrt => {
                let root = shifted.abs().sqrt();
                let signed_root = if shifted < 0.0 { -root } else { root };
                let sample_root = shifted * root;
                (signed_root * self.parameter) + (sample_root * (1.0 - self.parameter))
            }
            SaturationType::Diode => shifted.clamp(-self.parameter, self.parameter),
        };

        shaped - self.offset_out
    }
}

impl Default for Saturation {
    fn default() -> Self {
        Self::default_settings()
    }
}

fn is_unit_interval(value: f32) -> bool {
    value.is_finite() && (0.0..=1.0).contains(&value)
}

#[cfg(test)]
mod tests {
    use super::{Saturation, SaturationType};
    use crate::EffectError;

    #[test]
    fn default_tanh_matches_sox_ng_formula() {
        let mut samples = [-1.0, -0.5, 0.0, 0.5, 1.0];

        Saturation::default().process_samples(&mut samples);

        assert_samples_close(
            &samples,
            &[-0.9999, -0.606_715_44, 0.0, 0.606_715_44, 0.9999],
        );
    }

    #[test]
    fn sqrt_color_and_offset_match_sox_ng_formula() {
        let mut samples = [-1.0, -0.5, 0.0, 0.5, 1.0];

        Saturation::sqrt(0.5, 0.1, 0.25)
            .unwrap()
            .process_samples(&mut samples);

        assert_samples_close(
            &samples,
            &[-0.978_292_35, -0.469_860_2, 0.0, 0.464_405, 0.999_95],
        );
    }

    #[test]
    fn diode_threshold_clips_wet_path_before_output_gain() {
        let mut samples = [-1.0, -0.5, 0.0, 0.5, 1.0];

        Saturation::diode(1.0, 0.0, 0.5)
            .unwrap()
            .process_samples(&mut samples);

        assert_samples_close(&samples, &[-0.9999, -0.9999, 0.0, 0.9999, 0.9999]);
    }

    #[test]
    fn invalid_values_are_rejected() {
        assert_eq!(
            Saturation::new(SaturationType::Tanh, -0.1, 0.0, 1.0).unwrap_err(),
            EffectError::InvalidSaturation
        );
        assert_eq!(
            Saturation::new(SaturationType::Tanh, 1.0, 1.1, 1.0).unwrap_err(),
            EffectError::InvalidSaturation
        );
        assert_eq!(
            Saturation::new(SaturationType::Tanh, 1.0, 0.0, 0.9).unwrap_err(),
            EffectError::InvalidSaturation
        );
        assert_eq!(
            Saturation::new(SaturationType::Sqrt, 1.0, 0.0, f32::NAN).unwrap_err(),
            EffectError::InvalidSaturation
        );
        assert_eq!(
            Saturation::new(SaturationType::Diode, 1.0, 0.0, 1.1).unwrap_err(),
            EffectError::InvalidSaturation
        );
    }

    fn assert_samples_close(actual: &[f32], expected: &[f32]) {
        assert_eq!(actual.len(), expected.len());
        for (index, (&actual, &expected)) in actual.iter().zip(expected).enumerate() {
            assert!(
                (actual - expected).abs() <= 0.000_001,
                "sample {index}: actual={actual}, expected={expected}"
            );
        }
    }
}

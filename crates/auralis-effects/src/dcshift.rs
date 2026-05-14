use auralis_core::AudioBuffer;
use auralis_dsp::{dc_shift_in_place, dc_shift_in_place_with_backend};
use auralis_simd::{BackendKind, select_backend};

use crate::{EffectError, Result};

/// Constant DC offset effect processor.
///
/// `DcShift` adds a normalized full-scale offset to every sample. For example,
/// `0.25` adds one quarter of full scale and `0.0` is identity. The accepted
/// shift range is `-2.0..=2.0`, matching SoX-ng's `dcshift` command. The plain
/// single-argument processor does not clip, normalize, allocate, or inspect
/// channel boundaries; samples outside `[-1.0, 1.0]` are clipped only by later
/// boundary encoders such as PCM16 WAV output. [`Self::with_limiter_gain`]
/// enables SoX-ng's optional peak limiter, which reduces samples that would
/// clip in the direction of the DC shift and clips the effect output
/// immediately, matching SoX-ng chain behavior. Finite samples never become
/// `NaN`. Explicit backend methods can request SIMD through Auralis' backend
/// selection layer while preserving the same numerical behavior and scalar
/// fallback rules; limiter-gain processing uses the scalar SoX-ng formula.
///
/// # Errors
///
/// [`Self::new`] returns [`EffectError::InvalidDcShift`] when the shift is
/// `NaN`, infinite, or outside `-2.0..=2.0`. [`Self::with_limiter_gain`] also
/// returns [`EffectError::InvalidDcShift`] when the limiter gain is not finite.
///
/// # Examples
///
/// ```
/// use auralis_core::{
///     AudioBuffer, AudioSpec, ChannelCount, FrameCount, SampleFormat, SampleRate,
/// };
/// use auralis_effects::DcShift;
///
/// let spec = AudioSpec::new(
///     SampleRate::new(48_000)?,
///     ChannelCount::new(1)?,
///     SampleFormat::Float32,
/// );
/// let mut audio = AudioBuffer::from_planar_f32(
///     spec,
///     FrameCount::new(3),
///     vec![-0.5, 0.0, 0.5],
/// )?;
///
/// DcShift::new(0.25)?.process_buffer(&mut audio);
///
/// assert_eq!(audio.as_planar_f32(), &[-0.25, 0.25, 0.75]);
/// # Ok::<(), Box<dyn std::error::Error>>(())
/// ```
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct DcShift {
    /// Normalized full-scale offset to add to every sample.
    pub shift: f32,

    /// Optional SoX-ng limiter gain used only near clipping peaks.
    pub limiter_gain: Option<f32>,
}

impl DcShift {
    /// Creates a DC shift processor from a normalized full-scale offset.
    ///
    /// # Errors
    ///
    /// Returns [`EffectError::InvalidDcShift`] when `shift` is not finite or
    /// outside the supported `-2.0..=2.0` range.
    pub fn new(shift: f32) -> Result<Self> {
        if shift.is_finite() && (-2.0..=2.0).contains(&shift) {
            Ok(Self {
                shift,
                limiter_gain: None,
            })
        } else {
            Err(EffectError::InvalidDcShift)
        }
    }

    /// Creates a DC shift processor with SoX-ng's optional limiter gain.
    ///
    /// The limiter gain is a normalized full-scale amount used by SoX-ng's
    /// limiter formula on peaks that would clip in the direction of the shift.
    /// A value much smaller than `1.0`, such as `0.05`, is typical.
    ///
    /// # Errors
    ///
    /// Returns [`EffectError::InvalidDcShift`] when `shift` is not finite or
    /// outside the supported `-2.0..=2.0` range, or when `limiter_gain` is not
    /// finite.
    pub fn with_limiter_gain(shift: f32, limiter_gain: f32) -> Result<Self> {
        let mut dc_shift = Self::new(shift)?;
        if limiter_gain.is_finite() {
            dc_shift.limiter_gain = Some(limiter_gain);
            Ok(dc_shift)
        } else {
            Err(EffectError::InvalidDcShift)
        }
    }

    /// Applies the DC shift to all samples in an audio buffer.
    pub fn process_buffer(self, audio: &mut AudioBuffer) {
        self.process_samples(audio.as_planar_f32_mut());
    }

    /// Applies the DC shift to all samples in an audio buffer using the
    /// requested backend.
    ///
    /// Requesting [`BackendKind::Scalar`] forces the scalar reference path.
    /// Requesting [`BackendKind::Simd`] uses SIMD when the build and target
    /// support it, otherwise it follows the documented scalar fallback.
    pub fn process_buffer_with_backend(
        self,
        audio: &mut AudioBuffer,
        requested_backend: BackendKind,
    ) {
        self.process_samples_with_backend(audio.as_planar_f32_mut(), requested_backend);
    }

    /// Applies the DC shift to a planar sample slice.
    ///
    /// This method is suitable for streaming or chunked processing because each
    /// sample is transformed independently.
    pub fn process_samples(self, samples: &mut [f32]) {
        if let Some(limiter_gain) = self.limiter_gain {
            dc_shift_limited_in_place(samples, self.shift, limiter_gain);
        } else {
            dc_shift_in_place(samples, self.shift);
        }
    }

    /// Applies the DC shift to a planar sample slice using the requested backend.
    ///
    /// This method is suitable for streaming or chunked processing because each
    /// sample is transformed independently. Unsupported SIMD requests follow
    /// Auralis backend fallback metadata before processing continues.
    pub fn process_samples_with_backend(self, samples: &mut [f32], requested_backend: BackendKind) {
        if let Some(limiter_gain) = self.limiter_gain {
            dc_shift_limited_in_place(samples, self.shift, limiter_gain);
        } else {
            dc_shift_in_place_with_backend(select_backend(requested_backend), samples, self.shift);
        }
    }
}

const SOX_SAMPLE_MAX: f64 = i32::MAX as f64;
const SOX_SAMPLE_SCALE: f64 = SOX_SAMPLE_MAX + 1.0;
const SOX_SAMPLE_CLIP_MAX: f64 = SOX_SAMPLE_MAX / SOX_SAMPLE_SCALE;

#[expect(
    clippy::cast_possible_truncation,
    reason = "clamped SoX sample-domain values are intentionally stored in f32 buffers"
)]
fn dc_shift_limited_in_place(samples: &mut [f32], shift: f32, limiter_gain: f32) {
    if shift == 0.0 {
        return;
    }

    let shift = f64::from(shift);
    let limiter_gain = f64::from(limiter_gain);
    let limiter_threshold = SOX_SAMPLE_MAX * (1.0 - (shift.abs() - limiter_gain));
    let denominator = SOX_SAMPLE_MAX - limiter_threshold;
    let limiter_scale = if denominator == 0.0 {
        0.0
    } else {
        limiter_gain / denominator
    };
    let shift_sox = shift * SOX_SAMPLE_MAX;
    let inv_sox_sample_scale = 1.0 / SOX_SAMPLE_SCALE;

    for sample in samples {
        let sample_sox = f64::from(*sample) * SOX_SAMPLE_SCALE;
        let shifted = if sample_sox > limiter_threshold && shift > 0.0 {
            let limited = if denominator == 0.0 {
                limiter_threshold
            } else {
                (sample_sox - limiter_threshold) * limiter_scale + limiter_threshold + shift
            };
            limited * inv_sox_sample_scale
        } else if sample_sox < -limiter_threshold && shift < 0.0 {
            let limited = if denominator == 0.0 {
                -limiter_threshold
            } else {
                (sample_sox + limiter_threshold) * limiter_scale - limiter_threshold + shift
            };
            limited * inv_sox_sample_scale
        } else {
            (sample_sox + shift_sox) * inv_sox_sample_scale
        };
        *sample = clip_sox_sample(shifted) as f32;
    }
}

#[expect(
    clippy::manual_clamp,
    reason = "this helper is used in hot scalar loops where explicit comparisons benchmark faster"
)]
fn clip_sox_sample(sample: f64) -> f64 {
    if sample > SOX_SAMPLE_CLIP_MAX {
        SOX_SAMPLE_CLIP_MAX
    } else if sample < -1.0 {
        -1.0
    } else {
        sample
    }
}

#[cfg(test)]
mod tests {
    use super::DcShift;
    use crate::{
        EffectError,
        test_support::{assert_sample_bits_eq, audio_buffer, stereo_audio_buffer},
    };
    use auralis_core::{ChannelCount, FrameCount};
    use auralis_simd::BackendKind;

    #[test]
    fn zero_dc_shift_is_identity() {
        let mut audio = audio_buffer(vec![-0.5, 0.0, 0.5]);

        DcShift::new(0.0).unwrap().process_buffer(&mut audio);

        assert_eq!(audio.as_planar_f32(), &[-0.5, 0.0, 0.5]);
    }

    #[test]
    fn dc_shift_adds_positive_and_negative_offsets() {
        let mut positive = audio_buffer(vec![-0.5, 0.0, 0.5]);
        let mut negative = audio_buffer(vec![-0.5, 0.0, 0.5]);

        DcShift::new(0.25).unwrap().process_buffer(&mut positive);
        DcShift::new(-0.25).unwrap().process_buffer(&mut negative);

        assert_eq!(positive.as_planar_f32(), &[-0.25, 0.25, 0.75]);
        assert_eq!(negative.as_planar_f32(), &[-0.75, -0.25, 0.25]);
    }

    #[test]
    fn dc_shift_does_not_clip_and_preserves_stereo_shape() {
        let mut audio = stereo_audio_buffer(vec![0.75, 1.0, -0.75, -1.0]);

        DcShift::new(0.5).unwrap().process_buffer(&mut audio);

        assert_eq!(audio.frames(), FrameCount::new(2));
        assert_eq!(audio.channels(), ChannelCount::new(2).unwrap());
        assert_eq!(audio.as_planar_f32(), &[1.25, 1.5, -0.25, -0.5]);
    }

    #[test]
    fn dc_shift_limiter_clips_and_limits_peaks_in_shift_direction() {
        let mut audio = audio_buffer(vec![-1.0, -0.75, 0.0, 0.75, 1.0]);

        DcShift::with_limiter_gain(0.5, 0.05)
            .unwrap()
            .process_buffer(&mut audio);

        assert_sample_bits_eq(audio.as_planar_f32(), &[-0.5, -0.25, 0.5, 0.55, 0.55]);
    }

    #[test]
    fn dc_shift_limiter_applies_negative_shift_symmetrically() {
        let mut audio = audio_buffer(vec![-1.0, -0.75, 0.0, 0.75, 1.0]);

        DcShift::with_limiter_gain(-0.5, 0.05)
            .unwrap()
            .process_buffer(&mut audio);

        assert_sample_bits_eq(audio.as_planar_f32(), &[-0.55, -0.55, -0.5, 0.25, 0.5]);
    }

    #[test]
    fn dc_shift_rejects_non_finite_and_out_of_range_offsets() {
        assert_eq!(DcShift::new(f32::NAN), Err(EffectError::InvalidDcShift));
        assert_eq!(
            DcShift::new(f32::INFINITY),
            Err(EffectError::InvalidDcShift)
        );
        assert_eq!(DcShift::new(2.000_001), Err(EffectError::InvalidDcShift));
        assert_eq!(DcShift::new(-2.000_001), Err(EffectError::InvalidDcShift));
        assert_eq!(
            DcShift::with_limiter_gain(0.25, f32::NAN),
            Err(EffectError::InvalidDcShift)
        );
        assert!(DcShift::new(2.0).is_ok());
        assert!(DcShift::new(-2.0).is_ok());
    }

    #[test]
    fn dc_shift_chunked_processing_matches_whole_slice() {
        let source = vec![-1.0, -0.5, 0.0, 0.5, 1.0];
        let mut whole = source.clone();
        let mut chunked = source;

        DcShift::new(0.125).unwrap().process_samples(&mut whole);
        for chunk in chunked.chunks_mut(2) {
            DcShift::new(0.125).unwrap().process_samples(chunk);
        }

        assert_eq!(whole, chunked);
    }

    #[test]
    fn dc_shift_effect_matches_under_forced_scalar_and_requested_simd() {
        let source = vec![
            -1.0,
            -0.999_984_74,
            -f32::MIN_POSITIVE,
            -f32::from_bits(1),
            -0.0,
            0.0,
            f32::from_bits(1),
            f32::MIN_POSITIVE,
            0.999_984_74,
            1.0,
        ];
        let mut scalar = audio_buffer(source.clone());
        let mut simd = audio_buffer(source);
        let dc_shift = DcShift::new(0.125).unwrap();

        dc_shift.process_buffer_with_backend(&mut scalar, BackendKind::Scalar);
        dc_shift.process_buffer_with_backend(&mut simd, BackendKind::Simd);

        assert_sample_bits_eq(simd.as_planar_f32(), scalar.as_planar_f32());
    }

    #[test]
    fn dc_shift_limiter_matches_under_forced_scalar_and_requested_simd() {
        let source = vec![-1.0, -0.75, -0.0, 0.0, 0.75, 1.0];
        let mut scalar = audio_buffer(source.clone());
        let mut simd = audio_buffer(source);
        let dc_shift = DcShift::with_limiter_gain(0.5, 0.05).unwrap();

        dc_shift.process_buffer_with_backend(&mut scalar, BackendKind::Scalar);
        dc_shift.process_buffer_with_backend(&mut simd, BackendKind::Simd);

        assert_sample_bits_eq(simd.as_planar_f32(), scalar.as_planar_f32());
    }

    #[test]
    fn finite_dc_shift_inputs_do_not_produce_nan() {
        let mut audio = audio_buffer(vec![-1.0, -0.0, 0.0, 1.0, f32::MAX]);

        DcShift::new(2.0).unwrap().process_buffer(&mut audio);

        assert!(audio.as_planar_f32().iter().all(|sample| !sample.is_nan()));
    }
}

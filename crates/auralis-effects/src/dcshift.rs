use auralis_core::AudioBuffer;
use auralis_dsp::{dc_shift_in_place, dc_shift_in_place_with_backend};
use auralis_simd::{BackendKind, select_backend};

use crate::{EffectError, Result};

/// Constant DC offset effect processor.
///
/// `DcShift` adds a normalized full-scale offset to every sample. For example,
/// `0.25` adds one quarter of full scale and `0.0` is identity. The accepted
/// shift range is `-2.0..=2.0`, matching SoX-ng's single-argument `dcshift`
/// command. The processor does not clip, normalize, allocate, or inspect
/// channel boundaries; samples outside `[-1.0, 1.0]` are clipped only by later
/// boundary encoders such as PCM16 WAV output. Finite samples never become
/// `NaN`. Explicit backend methods can request SIMD through Auralis' backend
/// selection layer while preserving the same numerical behavior and scalar
/// fallback rules.
///
/// # Errors
///
/// [`Self::new`] returns [`EffectError::InvalidDcShift`] when the shift is
/// `NaN`, infinite, or outside `-2.0..=2.0`.
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
            Ok(Self { shift })
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
        dc_shift_in_place(samples, self.shift);
    }

    /// Applies the DC shift to a planar sample slice using the requested backend.
    ///
    /// This method is suitable for streaming or chunked processing because each
    /// sample is transformed independently. Unsupported SIMD requests follow
    /// Auralis backend fallback metadata before processing continues.
    pub fn process_samples_with_backend(self, samples: &mut [f32], requested_backend: BackendKind) {
        dc_shift_in_place_with_backend(select_backend(requested_backend), samples, self.shift);
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
    fn dc_shift_rejects_non_finite_and_out_of_range_offsets() {
        assert_eq!(DcShift::new(f32::NAN), Err(EffectError::InvalidDcShift));
        assert_eq!(
            DcShift::new(f32::INFINITY),
            Err(EffectError::InvalidDcShift)
        );
        assert_eq!(DcShift::new(2.000_001), Err(EffectError::InvalidDcShift));
        assert_eq!(DcShift::new(-2.000_001), Err(EffectError::InvalidDcShift));
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
    fn finite_dc_shift_inputs_do_not_produce_nan() {
        let mut audio = audio_buffer(vec![-1.0, -0.0, 0.0, 1.0, f32::MAX]);

        DcShift::new(2.0).unwrap().process_buffer(&mut audio);

        assert!(audio.as_planar_f32().iter().all(|sample| !sample.is_nan()));
    }
}

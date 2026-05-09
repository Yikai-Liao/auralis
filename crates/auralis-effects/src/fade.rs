use auralis_core::{AudioBuffer, FrameCount};
use auralis_dsp::{fade_in_place, fade_in_place_with_backend};
use auralis_simd::{BackendKind, select_backend};

/// Linear fade-in and fade-out effect processor.
///
/// `Fade` applies independent linear envelopes to the start and end of every
/// channel in a planar `f32` [`AudioBuffer`]. Fade lengths are measured in
/// frames. A fade-in length of `4` uses coefficients
/// `[0.0, 0.25, 0.5, 0.75]`; the first frame after the fade reaches `1.0`.
/// A fade-out length of `4` applies `[0.75, 0.5, 0.25, 0.0]` to the final four
/// frames. If fade-in and fade-out overlap, their coefficients are multiplied.
/// Zero-length fades are identity transforms.
///
/// # Examples
///
/// ```
/// use auralis_core::{
///     AudioBuffer, AudioSpec, ChannelCount, FrameCount, SampleFormat, SampleRate,
/// };
/// use auralis_effects::Fade;
///
/// let spec = AudioSpec::new(
///     SampleRate::new(48_000)?,
///     ChannelCount::new(1)?,
///     SampleFormat::Float32,
/// );
/// let mut audio = AudioBuffer::from_planar_f32(
///     spec,
///     FrameCount::new(4),
///     vec![1.0, 1.0, 1.0, 1.0],
/// )?;
///
/// Fade::new(FrameCount::new(2), FrameCount::new(2)).process_buffer(&mut audio);
///
/// assert_eq!(audio.as_planar_f32(), &[0.0, 0.5, 0.5, 0.0]);
/// # Ok::<(), auralis_core::AuralisError>(())
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Fade {
    /// Frames over which to ramp from silence to unity at the start.
    pub fade_in: FrameCount,

    /// Frames over which to ramp from unity to silence at the end.
    pub fade_out: FrameCount,
}

impl Fade {
    /// Creates a linear fade processor.
    #[must_use]
    pub const fn new(fade_in: FrameCount, fade_out: FrameCount) -> Self {
        Self { fade_in, fade_out }
    }

    /// Applies the fade envelope in place to every channel of an audio buffer.
    ///
    /// Processing is deterministic, non-allocating, and preserves channel
    /// grouping. Zero fade lengths are identity transforms.
    pub fn process_buffer(self, audio: &mut AudioBuffer) {
        self.process_buffer_with_backend(audio, BackendKind::Scalar);
    }

    /// Applies the fade envelope in place using the requested backend.
    ///
    /// Processing has the same channel-preserving behavior as
    /// [`Self::process_buffer`]. Requesting [`BackendKind::Simd`] uses the
    /// backend-dispatched fade kernel when available and deterministic scalar
    /// fallback otherwise.
    pub fn process_buffer_with_backend(
        self,
        audio: &mut AudioBuffer,
        requested_backend: BackendKind,
    ) {
        let total_frames = audio.frames().as_u64();
        for channel_index in 0..audio.channels().as_usize() {
            if let Some(channel) = audio.channel_mut(channel_index) {
                self.process_channel_segment_with_backend(
                    channel,
                    total_frames,
                    FrameCount::new(0),
                    requested_backend,
                );
            }
        }
    }

    /// Applies the fade to a contiguous channel segment with a known frame
    /// offset in the full signal.
    ///
    /// This method exists so streaming callers and tests can process chunks
    /// while still using full-signal frame positions. Passing every segment in
    /// order produces the same result as [`Self::process_buffer`].
    pub fn process_channel_segment(
        self,
        samples: &mut [f32],
        total_frames: u64,
        start_frame: FrameCount,
    ) {
        fade_in_place(
            samples,
            total_frames,
            start_frame.as_u64(),
            self.fade_in.as_u64(),
            self.fade_out.as_u64(),
        );
    }

    /// Applies the fade to a contiguous channel segment using the requested backend.
    ///
    /// Parameters and chunk-invariance behavior match
    /// [`Self::process_channel_segment`].
    pub fn process_channel_segment_with_backend(
        self,
        samples: &mut [f32],
        total_frames: u64,
        start_frame: FrameCount,
        requested_backend: BackendKind,
    ) {
        fade_in_place_with_backend(
            select_backend(requested_backend),
            samples,
            total_frames,
            start_frame.as_u64(),
            self.fade_in.as_u64(),
            self.fade_out.as_u64(),
        );
    }
}

#[cfg(test)]
mod tests {
    use super::Fade;
    use crate::test_support::{
        assert_sample_bits_eq, assert_samples_close, audio_buffer, stereo_audio_buffer,
    };
    use auralis_core::{ChannelCount, FrameCount};
    use auralis_simd::BackendKind;

    #[test]
    fn fade_envelope_coefficients_are_linear() {
        let mut audio = audio_buffer(vec![1.0; 6]);

        Fade::new(FrameCount::new(4), FrameCount::new(3)).process_buffer(&mut audio);

        assert_samples_close(
            audio.as_planar_f32(),
            &[0.0, 0.25, 0.5, 0.5, 1.0 / 3.0, 0.0],
        );
    }

    #[test]
    fn zero_length_fade_is_identity() {
        let source = stereo_audio_buffer(vec![-0.5, 0.0, 0.5, 0.25, -0.25, 1.0]);
        let mut actual = source.clone();

        Fade::new(FrameCount::new(0), FrameCount::new(0)).process_buffer(&mut actual);

        assert_eq!(actual, source);
    }

    #[test]
    fn fade_in_only_and_fade_out_only_apply_one_side() {
        let mut fade_in = audio_buffer(vec![1.0; 5]);
        let mut fade_out = audio_buffer(vec![1.0; 5]);

        Fade::new(FrameCount::new(4), FrameCount::new(0)).process_buffer(&mut fade_in);
        Fade::new(FrameCount::new(0), FrameCount::new(4)).process_buffer(&mut fade_out);

        assert_samples_close(fade_in.as_planar_f32(), &[0.0, 0.25, 0.5, 0.75, 1.0]);
        assert_samples_close(fade_out.as_planar_f32(), &[1.0, 0.75, 0.5, 0.25, 0.0]);
    }

    #[test]
    fn stereo_fade_preserves_channel_grouping() {
        let mut audio = stereo_audio_buffer(vec![1.0, 1.0, 1.0, 1.0, -1.0, -1.0, -1.0, -1.0]);

        Fade::new(FrameCount::new(2), FrameCount::new(2)).process_buffer(&mut audio);

        assert_eq!(audio.frames(), FrameCount::new(4));
        assert_eq!(audio.channels(), ChannelCount::new(2).unwrap());
        assert_samples_close(
            audio.as_planar_f32(),
            &[0.0, 0.5, 0.5, 0.0, -0.0, -0.5, -0.5, -0.0],
        );
    }

    #[test]
    fn fade_segment_processing_matches_whole_channel() {
        let source = vec![1.0; 9];
        let fade = Fade::new(FrameCount::new(4), FrameCount::new(4));
        let mut whole = audio_buffer(source.clone());
        let mut chunked = source;
        let mut start = 0_u64;

        fade.process_buffer(&mut whole);
        for chunk in chunked.chunks_mut(3) {
            fade.process_channel_segment(chunk, 9, FrameCount::new(start));
            start += u64::try_from(chunk.len()).unwrap();
        }

        assert_samples_close(whole.as_planar_f32(), &chunked);
    }

    #[test]
    fn fade_effect_matches_under_forced_scalar_and_requested_simd() {
        let source = stereo_audio_buffer(vec![
            -1.0,
            -0.999_984_74,
            -0.5,
            -0.0,
            0.0,
            0.5,
            0.999_984_74,
            1.0,
            1.0,
            0.999_984_74,
            0.5,
            0.0,
            -0.0,
            -0.5,
            -0.999_984_74,
            -1.0,
        ]);
        let fade = Fade::new(FrameCount::new(5), FrameCount::new(7));
        let mut scalar = source.clone();
        let mut simd = source;

        fade.process_buffer_with_backend(&mut scalar, BackendKind::Scalar);
        fade.process_buffer_with_backend(&mut simd, BackendKind::Simd);

        assert_sample_bits_eq(simd.as_planar_f32(), scalar.as_planar_f32());
    }
}

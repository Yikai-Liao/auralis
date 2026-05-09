use auralis_core::AudioBuffer;

/// Frame-level reverse effect processor.
///
/// `Reverse` reverses the frame order independently within every channel of a
/// planar `f32` [`AudioBuffer`]. It never swaps channels: a stereo buffer with
/// left channel `[L0, L1]` and right channel `[R0, R1]` becomes `[L1, L0]` and
/// `[R1, R0]`. Processing is deterministic, in-place, non-allocating, and an
/// identity transform for zero-length and one-frame buffers.
///
/// # Examples
///
/// ```
/// use auralis_core::{
///     AudioBuffer, AudioSpec, ChannelCount, FrameCount, SampleFormat, SampleRate,
/// };
/// use auralis_effects::Reverse;
///
/// let spec = AudioSpec::new(
///     SampleRate::new(48_000)?,
///     ChannelCount::new(2)?,
///     SampleFormat::Float32,
/// );
/// let mut audio = AudioBuffer::from_planar_f32(
///     spec,
///     FrameCount::new(3),
///     vec![0.0, 0.25, 0.5, -0.5, -0.25, 0.0],
/// )?;
///
/// Reverse::new().process_buffer(&mut audio);
///
/// assert_eq!(audio.as_planar_f32(), &[0.5, 0.25, 0.0, 0.0, -0.25, -0.5]);
/// # Ok::<(), auralis_core::AuralisError>(())
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Reverse;

impl Reverse {
    /// Creates a frame-level reverse processor.
    #[must_use]
    pub const fn new() -> Self {
        Self
    }

    /// Reverses frame order in every channel of an audio buffer.
    pub fn process_buffer(self, audio: &mut AudioBuffer) {
        for channel_index in 0..audio.channels().as_usize() {
            if let Some(channel) = audio.channel_mut(channel_index) {
                self.process_channel(channel);
            }
        }
    }

    /// Reverses a single planar channel in place.
    ///
    /// This method is provided for tests and low-level callers that already
    /// hold channel views. It is not a streaming transform: reversing separate
    /// chunks is not equivalent to reversing the full signal.
    pub fn process_channel(self, samples: &mut [f32]) {
        samples.reverse();
    }
}

#[cfg(test)]
mod tests {
    use super::Reverse;
    use crate::test_support::{audio_buffer, stereo_audio_buffer};
    use auralis_core::{ChannelCount, FrameCount};

    #[test]
    fn reverse_mono_exact_frame_order() {
        let mut audio = audio_buffer(vec![-0.75, -0.25, 0.25, 0.75]);

        Reverse::new().process_buffer(&mut audio);

        assert_eq!(audio.as_planar_f32(), &[0.75, 0.25, -0.25, -0.75]);
    }

    #[test]
    fn reverse_stereo_preserves_channel_grouping() {
        let mut audio = stereo_audio_buffer(vec![0.0, 0.25, 0.5, 1.0, -0.25, -0.5]);

        Reverse::new().process_buffer(&mut audio);

        assert_eq!(audio.frames(), FrameCount::new(3));
        assert_eq!(audio.channels(), ChannelCount::new(2).unwrap());
        assert_eq!(audio.as_planar_f32(), &[0.5, 0.25, 0.0, -0.5, -0.25, 1.0]);
    }

    #[test]
    fn reverse_twice_is_identity() {
        let source = stereo_audio_buffer(vec![0.0, 0.25, 0.5, -0.5, -0.25, 1.0]);
        let mut actual = source.clone();

        Reverse::new().process_buffer(&mut actual);
        Reverse::new().process_buffer(&mut actual);

        assert_eq!(actual, source);
    }

    #[test]
    fn reverse_zero_length_and_one_frame_are_identity() {
        let mut empty = audio_buffer(Vec::new());
        let mut one_frame = stereo_audio_buffer(vec![0.25, -0.25]);

        Reverse::new().process_buffer(&mut empty);
        Reverse::new().process_buffer(&mut one_frame);

        assert!(empty.as_planar_f32().is_empty());
        assert_eq!(one_frame.as_planar_f32(), &[0.25, -0.25]);
    }
}

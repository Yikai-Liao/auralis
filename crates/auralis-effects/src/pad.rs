use auralis_core::{AudioBuffer, FrameCount};

use crate::{EffectError, Result};

/// Start/end silence padding effect processor.
///
/// `Pad` adds zero-valued frames before and after every channel and returns a
/// new planar `f32` [`AudioBuffer`]. Padding lengths are measured in frames, so
/// multi-channel audio keeps channel grouping intact. `0` start and end frames
/// is an identity transform aside from allocating a new buffer with identical
/// contents.
///
/// # Errors
///
/// [`Self::process_buffer`] returns [`EffectError::PadLengthOverflow`] when the
/// requested output frame count or planar sample count cannot be represented on
/// the current platform.
///
/// # Examples
///
/// ```
/// use auralis_core::{
///     AudioBuffer, AudioSpec, ChannelCount, FrameCount, SampleFormat, SampleRate,
/// };
/// use auralis_effects::Pad;
///
/// let spec = AudioSpec::new(
///     SampleRate::new(48_000)?,
///     ChannelCount::new(1)?,
///     SampleFormat::Float32,
/// );
/// let audio = AudioBuffer::from_planar_f32(
///     spec,
///     FrameCount::new(2),
///     vec![0.25, -0.5],
/// )?;
///
/// let padded = Pad::new(FrameCount::new(1), FrameCount::new(1))
///     .process_buffer(&audio)?;
///
/// assert_eq!(padded.as_planar_f32(), &[0.0, 0.25, -0.5, 0.0]);
/// # Ok::<(), Box<dyn std::error::Error>>(())
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Pad {
    /// Silent frames to add before the input audio.
    pub start: FrameCount,

    /// Silent frames to add after the input audio.
    pub end: FrameCount,
}

impl Pad {
    /// Creates a start/end zero-padding processor.
    #[must_use]
    pub const fn new(start: FrameCount, end: FrameCount) -> Self {
        Self { start, end }
    }

    /// Applies zero padding to an audio buffer and returns the padded output.
    ///
    /// The output keeps the input specification and stores samples in the same
    /// planar channel-major layout. Processing is deterministic and allocates
    /// exactly `channels * (start + input_frames + end)` samples.
    ///
    /// # Errors
    ///
    /// Returns [`EffectError::PadLengthOverflow`] when the output frame count
    /// or allocation length cannot be represented.
    pub fn process_buffer(self, audio: &AudioBuffer) -> Result<AudioBuffer> {
        let output_frames = self
            .start
            .as_u64()
            .checked_add(audio.frames().as_u64())
            .and_then(|frames| frames.checked_add(self.end.as_u64()))
            .ok_or(EffectError::PadLengthOverflow)?;
        let output_frames = FrameCount::new(output_frames);
        let start =
            usize::try_from(self.start.as_u64()).map_err(|_| EffectError::PadLengthOverflow)?;
        let end = usize::try_from(self.end.as_u64()).map_err(|_| EffectError::PadLengthOverflow)?;
        let output_frames_usize =
            usize::try_from(output_frames.as_u64()).map_err(|_| EffectError::PadLengthOverflow)?;
        let capacity = audio
            .channels()
            .as_usize()
            .checked_mul(output_frames_usize)
            .ok_or(EffectError::PadLengthOverflow)?;
        let mut output = Vec::with_capacity(capacity);

        for channel_index in 0..audio.channels().as_usize() {
            output.resize(output.len() + start, 0.0);
            let channel = audio
                .channel(channel_index)
                .ok_or(EffectError::PadLengthOverflow)?;
            output.extend_from_slice(channel);
            output.resize(output.len() + end, 0.0);
        }

        debug_assert_eq!(output.len(), capacity);

        Ok(AudioBuffer::from_planar_f32(
            audio.spec(),
            output_frames,
            output,
        )?)
    }
}

#[cfg(test)]
mod tests {
    use super::Pad;
    use crate::{
        EffectError,
        test_support::{audio_buffer, stereo_audio_buffer},
    };
    use auralis_core::FrameCount;

    #[test]
    fn zero_pad_is_identity() {
        let audio = audio_buffer(vec![-0.5, 0.0, 0.5]);

        let padded = Pad::new(FrameCount::new(0), FrameCount::new(0))
            .process_buffer(&audio)
            .unwrap();

        assert_eq!(padded, audio);
    }

    #[test]
    fn start_and_end_pad_insert_exact_zeros() {
        let audio = audio_buffer(vec![0.25, -0.5]);

        let padded = Pad::new(FrameCount::new(2), FrameCount::new(1))
            .process_buffer(&audio)
            .unwrap();

        assert_eq!(padded.frames(), FrameCount::new(5));
        assert_eq!(padded.as_planar_f32(), &[0.0, 0.0, 0.25, -0.5, 0.0]);
    }

    #[test]
    fn stereo_pad_preserves_channel_shape() {
        let audio = stereo_audio_buffer(vec![0.25, 0.5, -0.25, -0.5]);

        let padded = Pad::new(FrameCount::new(1), FrameCount::new(2))
            .process_buffer(&audio)
            .unwrap();

        assert_eq!(padded.frames(), FrameCount::new(5));
        assert_eq!(padded.channels(), audio.channels());
        assert_eq!(
            padded.as_planar_f32(),
            &[0.0, 0.25, 0.5, 0.0, 0.0, 0.0, -0.25, -0.5, 0.0, 0.0]
        );
    }

    #[test]
    fn pad_rejects_unrepresentable_frame_count() {
        let error = Pad::new(FrameCount::new(u64::MAX), FrameCount::new(1))
            .process_buffer(&audio_buffer(vec![0.0]))
            .unwrap_err();

        assert_eq!(error, EffectError::PadLengthOverflow);
    }
}

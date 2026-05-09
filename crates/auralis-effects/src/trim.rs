use auralis_core::{AudioBuffer, FrameCount};

use crate::{EffectError, Result};

/// Frame-range trim effect processor.
///
/// `Trim` keeps the half-open frame range `start..end` from every channel and
/// returns a new planar `f32` [`AudioBuffer`]. The range is measured in frames,
/// not individual samples, so stereo and larger channel layouts preserve frame
/// grouping. `start == end` is valid and produces an empty buffer with the same
/// audio specification.
///
/// # Errors
///
/// [`Self::new`] returns [`EffectError::InvalidTrimOrder`] when `start > end`.
/// [`Self::process_buffer`] returns [`EffectError::TrimRangeOutOfBounds`] when
/// the validated range is outside the input buffer.
///
/// # Examples
///
/// ```
/// use auralis_core::{
///     AudioBuffer, AudioSpec, ChannelCount, FrameCount, SampleFormat, SampleRate,
/// };
/// use auralis_effects::Trim;
///
/// let spec = AudioSpec::new(
///     SampleRate::new(48_000)?,
///     ChannelCount::new(1)?,
///     SampleFormat::Float32,
/// );
/// let audio = AudioBuffer::from_planar_f32(
///     spec,
///     FrameCount::new(4),
///     vec![0.0, 0.25, 0.5, 0.75],
/// )?;
///
/// let trimmed = Trim::new(FrameCount::new(1), FrameCount::new(3))?
///     .process_buffer(&audio)?;
///
/// assert_eq!(trimmed.as_planar_f32(), &[0.25, 0.5]);
/// # Ok::<(), Box<dyn std::error::Error>>(())
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Trim {
    /// First frame to keep.
    pub start: FrameCount,

    /// End-exclusive frame index.
    pub end: FrameCount,
}

impl Trim {
    /// Creates a frame-range trim processor.
    ///
    /// # Errors
    ///
    /// Returns [`EffectError::InvalidTrimOrder`] when `start > end`.
    pub fn new(start: FrameCount, end: FrameCount) -> Result<Self> {
        if start <= end {
            Ok(Self { start, end })
        } else {
            Err(EffectError::InvalidTrimOrder)
        }
    }

    /// Applies the trim to an audio buffer and returns the retained range.
    ///
    /// The output keeps the input specification and stores samples in the same
    /// planar channel-major layout. Processing is deterministic and allocates
    /// exactly `channels * (end - start)` samples.
    ///
    /// # Errors
    ///
    /// Returns [`EffectError::TrimRangeOutOfBounds`] when `end` is greater than
    /// the input frame count or the frame indices cannot be represented on the
    /// current platform.
    pub fn process_buffer(self, audio: &AudioBuffer) -> Result<AudioBuffer> {
        let frames = audio.frames().as_u64();
        if self.end.as_u64() > frames {
            return Err(EffectError::TrimRangeOutOfBounds);
        }

        let start =
            usize::try_from(self.start.as_u64()).map_err(|_| EffectError::TrimRangeOutOfBounds)?;
        let end =
            usize::try_from(self.end.as_u64()).map_err(|_| EffectError::TrimRangeOutOfBounds)?;
        let output_frames = FrameCount::new(self.end.as_u64() - self.start.as_u64());
        let mut output = Vec::with_capacity(
            audio
                .channels()
                .as_usize()
                .checked_mul(end - start)
                .ok_or(EffectError::TrimRangeOutOfBounds)?,
        );

        for channel_index in 0..audio.channels().as_usize() {
            let channel = audio
                .channel(channel_index)
                .ok_or(EffectError::TrimRangeOutOfBounds)?;
            output.extend_from_slice(&channel[start..end]);
        }

        Ok(AudioBuffer::from_planar_f32(
            audio.spec(),
            output_frames,
            output,
        )?)
    }
}

#[cfg(test)]
mod tests {
    use super::Trim;
    use crate::{
        EffectError,
        test_support::{audio_buffer, stereo_audio_buffer},
    };
    use auralis_core::FrameCount;

    #[test]
    fn trim_exact_frame_range() {
        let audio = stereo_audio_buffer(vec![0.0, 0.25, 0.5, 0.75, 1.0, -0.25, -0.5, -0.75]);

        let trimmed = Trim::new(FrameCount::new(1), FrameCount::new(3))
            .unwrap()
            .process_buffer(&audio)
            .unwrap();

        assert_eq!(trimmed.frames(), FrameCount::new(2));
        assert_eq!(trimmed.channels(), audio.channels());
        assert_eq!(trimmed.as_planar_f32(), &[0.25, 0.5, -0.25, -0.5]);
    }

    #[test]
    fn trim_full_range_is_identity() {
        let audio = audio_buffer(vec![-0.5, 0.0, 0.5]);

        let trimmed = Trim::new(FrameCount::new(0), audio.frames())
            .unwrap()
            .process_buffer(&audio)
            .unwrap();

        assert_eq!(trimmed, audio);
    }

    #[test]
    fn trim_empty_range_keeps_shape_metadata() {
        let audio = stereo_audio_buffer(vec![0.0, 0.25, 0.5, 0.75]);

        let trimmed = Trim::new(FrameCount::new(2), FrameCount::new(2))
            .unwrap()
            .process_buffer(&audio)
            .unwrap();

        assert_eq!(trimmed.frames(), FrameCount::new(0));
        assert_eq!(trimmed.channels(), audio.channels());
        assert!(trimmed.as_planar_f32().is_empty());
    }

    #[test]
    fn trim_rejects_reversed_or_out_of_bounds_ranges() {
        assert_eq!(
            Trim::new(FrameCount::new(3), FrameCount::new(2)),
            Err(EffectError::InvalidTrimOrder)
        );

        let error = Trim::new(FrameCount::new(0), FrameCount::new(4))
            .unwrap()
            .process_buffer(&audio_buffer(vec![0.0, 0.5]))
            .unwrap_err();

        assert_eq!(error, EffectError::TrimRangeOutOfBounds);
    }
}

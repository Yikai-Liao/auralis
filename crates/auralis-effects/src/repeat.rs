use auralis_core::{AudioBuffer, FrameCount};

use crate::{EffectError, Result};

/// Maximum finite repeat count accepted by SoX-ng.
///
/// SoX-ng parses `repeat COUNT` as an unsigned integer in `0..=UINT_MAX - 1`.
/// The separate `repeat -` indefinite form is intentionally not modeled by
/// Auralis because in-memory processing requires a bounded output length.
pub const MAX_FINITE_REPEAT_COUNT: u64 = u32::MAX as u64 - 1;

/// SoX-ng-style finite repeat effect processor.
///
/// `Repeat` emits the input audio followed by `count` additional copies, so
/// `Repeat::new(1)` doubles the input and `Repeat::new(0)` is an identity
/// transform aside from allocating a new buffer. Processing preserves channel
/// grouping in Auralis' planar `f32` layout and validates that the output
/// frame count and sample allocation fit on the current platform.
///
/// # Errors
///
/// [`Self::new`] returns [`EffectError::InvalidRepeatCount`] for counts above
/// SoX-ng's finite range. [`Self::process_buffer`] returns
/// [`EffectError::RepeatLengthOverflow`] if the repeated output shape cannot
/// be represented.
///
/// # Examples
///
/// ```
/// use auralis_core::{
///     AudioBuffer, AudioSpec, ChannelCount, FrameCount, SampleFormat, SampleRate,
/// };
/// use auralis_effects::Repeat;
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
/// let repeated = Repeat::new(2)?.process_buffer(&audio)?;
///
/// assert_eq!(repeated.as_planar_f32(), &[0.25, -0.5, 0.25, -0.5, 0.25, -0.5]);
/// # Ok::<(), Box<dyn std::error::Error>>(())
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Repeat {
    /// Number of additional copies to append after the original input.
    pub count: u64,
}

impl Repeat {
    /// Creates a finite repeat processor.
    ///
    /// # Errors
    ///
    /// Returns [`EffectError::InvalidRepeatCount`] when `count` is larger than
    /// SoX-ng's finite `repeat COUNT` range.
    pub fn new(count: u64) -> Result<Self> {
        if count > MAX_FINITE_REPEAT_COUNT {
            return Err(EffectError::InvalidRepeatCount);
        }

        Ok(Self { count })
    }

    /// Creates the SoX-ng default `repeat` processor, equivalent to
    /// `repeat 1`.
    #[must_use]
    pub const fn default_count() -> Self {
        Self { count: 1 }
    }

    /// Applies finite repetition to an audio buffer and returns the output.
    ///
    /// The output keeps the input specification and stores samples in the same
    /// planar channel-major layout.
    ///
    /// # Errors
    ///
    /// Returns [`EffectError::RepeatLengthOverflow`] when the output frame
    /// count or planar sample count cannot be represented.
    pub fn process_buffer(self, audio: &AudioBuffer) -> Result<AudioBuffer> {
        let copies = self
            .count
            .checked_add(1)
            .ok_or(EffectError::RepeatLengthOverflow)?;
        let output_frames = audio
            .frames()
            .as_u64()
            .checked_mul(copies)
            .ok_or(EffectError::RepeatLengthOverflow)?;
        let output_frames = FrameCount::new(output_frames);
        let output_frames_usize = usize::try_from(output_frames.as_u64())
            .map_err(|_| EffectError::RepeatLengthOverflow)?;
        let capacity = audio
            .channels()
            .as_usize()
            .checked_mul(output_frames_usize)
            .ok_or(EffectError::RepeatLengthOverflow)?;
        let copies_usize =
            usize::try_from(copies).map_err(|_| EffectError::RepeatLengthOverflow)?;
        let mut output = Vec::with_capacity(capacity);

        for channel_index in 0..audio.channels().as_usize() {
            let channel = audio
                .channel(channel_index)
                .ok_or(EffectError::RepeatLengthOverflow)?;
            for _ in 0..copies_usize {
                output.extend_from_slice(channel);
            }
        }

        debug_assert_eq!(output.len(), capacity);

        Ok(AudioBuffer::from_planar_f32(
            audio.spec(),
            output_frames,
            output,
        )?)
    }
}

impl Default for Repeat {
    fn default() -> Self {
        Self::default_count()
    }
}

#[cfg(test)]
mod tests {
    use super::{MAX_FINITE_REPEAT_COUNT, Repeat};
    use crate::{
        EffectError,
        test_support::{audio_buffer, stereo_audio_buffer},
    };
    use auralis_core::FrameCount;

    #[test]
    fn finite_repeat_appends_full_copies_per_channel() {
        let audio = stereo_audio_buffer(vec![0.0, 0.25, -0.5, 0.5]);

        let repeated = Repeat::new(2).unwrap().process_buffer(&audio).unwrap();

        assert_eq!(repeated.frames(), FrameCount::new(6));
        assert_eq!(
            repeated.as_planar_f32(),
            &[
                0.0, 0.25, 0.0, 0.25, 0.0, 0.25, -0.5, 0.5, -0.5, 0.5, -0.5, 0.5
            ]
        );
    }

    #[test]
    fn zero_repeat_is_identity_copy() {
        let audio = audio_buffer(vec![-0.5, 0.0, 0.5]);

        let repeated = Repeat::new(0).unwrap().process_buffer(&audio).unwrap();

        assert_eq!(repeated, audio);
    }

    #[test]
    fn rejects_count_above_sox_ng_finite_range() {
        assert_eq!(
            Repeat::new(MAX_FINITE_REPEAT_COUNT + 1).unwrap_err(),
            EffectError::InvalidRepeatCount
        );
    }
}

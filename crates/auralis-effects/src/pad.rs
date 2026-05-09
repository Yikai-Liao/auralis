use auralis_core::{AudioBuffer, FrameCount};

use crate::{EffectError, Result};

/// A single positioned silence insertion for [`Pad`].
///
/// Both fields are measured in frames. The insertion happens before the input
/// frame at `position`; a position equal to the input length appends silence
/// just before any end padding.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PositionedPad {
    /// Silent frames to insert at [`Self::position`].
    pub length: FrameCount,

    /// Input-frame position where silence is inserted.
    pub position: FrameCount,
}

impl PositionedPad {
    /// Creates a positioned silence insertion.
    #[must_use]
    pub const fn new(length: FrameCount, position: FrameCount) -> Self {
        Self { length, position }
    }
}

/// Silence padding effect processor.
///
/// `Pad` adds zero-valued frames before, after, or inside every channel and
/// returns a new planar `f32` [`AudioBuffer`]. Padding lengths and positions are
/// measured in frames, so multi-channel audio keeps channel grouping intact.
/// `0` start and end frames with no positioned insertions is an identity
/// transform aside from allocating a new buffer with identical contents.
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
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Pad {
    /// Silent frames to add before the input audio.
    pub start: FrameCount,

    /// Silent frames to add after the input audio.
    pub end: FrameCount,

    positioned: Vec<PositionedPad>,
}

impl Pad {
    /// Creates a start/end zero-padding processor.
    #[must_use]
    pub const fn new(start: FrameCount, end: FrameCount) -> Self {
        Self {
            start,
            end,
            positioned: Vec::new(),
        }
    }

    /// Creates a zero-padding processor with additional positioned insertions.
    ///
    /// Positioned insertions must be in strictly ascending input-frame order.
    /// A first insertion at frame `0` is valid only when `start` is zero; use
    /// [`Self::new`] or merge the lengths when both forms target the start.
    ///
    /// # Errors
    ///
    /// Returns [`EffectError::PadPositionsOutOfOrder`] if positions are
    /// duplicated or unsorted.
    pub fn with_positioned<I>(start: FrameCount, end: FrameCount, positioned: I) -> Result<Self>
    where
        I: IntoIterator<Item = PositionedPad>,
    {
        let positioned: Vec<PositionedPad> = positioned.into_iter().collect();
        validate_position_order(start, &positioned)?;

        Ok(Self {
            start,
            end,
            positioned,
        })
    }

    /// Returns the configured positioned insertions in input-frame order.
    #[must_use]
    pub fn positioned(&self) -> &[PositionedPad] {
        &self.positioned
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
    /// or allocation length cannot be represented, or
    /// [`EffectError::PadPositionOutOfBounds`] when a positioned insertion is
    /// after the input duration.
    pub fn process_buffer(&self, audio: &AudioBuffer) -> Result<AudioBuffer> {
        let mut output_frames = self
            .start
            .as_u64()
            .checked_add(audio.frames().as_u64())
            .and_then(|frames| frames.checked_add(self.end.as_u64()))
            .ok_or(EffectError::PadLengthOverflow)?;
        for pad in &self.positioned {
            if pad.position.as_u64() > audio.frames().as_u64() {
                return Err(EffectError::PadPositionOutOfBounds);
            }
            output_frames = output_frames
                .checked_add(pad.length.as_u64())
                .ok_or(EffectError::PadLengthOverflow)?;
        }
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
            let mut cursor = 0;
            for pad in &self.positioned {
                let position = usize::try_from(pad.position.as_u64())
                    .map_err(|_| EffectError::PadLengthOverflow)?;
                let length = usize::try_from(pad.length.as_u64())
                    .map_err(|_| EffectError::PadLengthOverflow)?;
                output.extend_from_slice(&channel[cursor..position]);
                output.resize(output.len() + length, 0.0);
                cursor = position;
            }
            output.extend_from_slice(&channel[cursor..]);
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

fn validate_position_order(start: FrameCount, positioned: &[PositionedPad]) -> Result<()> {
    let mut previous = if start.as_u64() > 0 {
        Some(0_u64)
    } else {
        None
    };

    for pad in positioned {
        let position = pad.position.as_u64();
        if previous.is_some_and(|previous| position <= previous) {
            return Err(EffectError::PadPositionsOutOfOrder);
        }
        previous = Some(position);
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::{Pad, PositionedPad};
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
    fn positioned_pad_inserts_silence_at_input_frame_position() {
        let audio = audio_buffer(vec![0.25, 0.5, -0.25, -0.5]);

        let padded = Pad::with_positioned(
            FrameCount::new(1),
            FrameCount::new(2),
            [PositionedPad::new(FrameCount::new(2), FrameCount::new(2))],
        )
        .unwrap()
        .process_buffer(&audio)
        .unwrap();

        assert_eq!(padded.frames(), FrameCount::new(9));
        assert_eq!(
            padded.as_planar_f32(),
            &[0.0, 0.25, 0.5, 0.0, 0.0, -0.25, -0.5, 0.0, 0.0]
        );
    }

    #[test]
    fn multiple_positioned_pads_preserve_stereo_shape() {
        let audio = stereo_audio_buffer(vec![0.25, 0.5, -0.25, -0.5]);

        let padded = Pad::with_positioned(
            FrameCount::new(0),
            FrameCount::new(0),
            [
                PositionedPad::new(FrameCount::new(1), FrameCount::new(1)),
                PositionedPad::new(FrameCount::new(2), FrameCount::new(2)),
            ],
        )
        .unwrap()
        .process_buffer(&audio)
        .unwrap();

        assert_eq!(padded.frames(), FrameCount::new(5));
        assert_eq!(
            padded.as_planar_f32(),
            &[0.25, 0.0, 0.5, 0.0, 0.0, -0.25, 0.0, -0.5, 0.0, 0.0]
        );
    }

    #[test]
    fn pad_rejects_unrepresentable_frame_count() {
        let error = Pad::new(FrameCount::new(u64::MAX), FrameCount::new(1))
            .process_buffer(&audio_buffer(vec![0.0]))
            .unwrap_err();

        assert_eq!(error, EffectError::PadLengthOverflow);
    }

    #[test]
    fn positioned_pad_rejects_unsorted_or_duplicate_positions() {
        let duplicate = Pad::with_positioned(
            FrameCount::new(0),
            FrameCount::new(0),
            [
                PositionedPad::new(FrameCount::new(1), FrameCount::new(2)),
                PositionedPad::new(FrameCount::new(1), FrameCount::new(2)),
            ],
        )
        .unwrap_err();
        assert_eq!(duplicate, EffectError::PadPositionsOutOfOrder);

        let after_start = Pad::with_positioned(
            FrameCount::new(1),
            FrameCount::new(0),
            [PositionedPad::new(FrameCount::new(1), FrameCount::new(0))],
        )
        .unwrap_err();
        assert_eq!(after_start, EffectError::PadPositionsOutOfOrder);
    }

    #[test]
    fn positioned_pad_rejects_positions_after_input_duration() {
        let error = Pad::with_positioned(
            FrameCount::new(0),
            FrameCount::new(0),
            [PositionedPad::new(FrameCount::new(1), FrameCount::new(3))],
        )
        .unwrap()
        .process_buffer(&audio_buffer(vec![0.0, 0.5]))
        .unwrap_err();

        assert_eq!(error, EffectError::PadPositionOutOfBounds);
    }
}

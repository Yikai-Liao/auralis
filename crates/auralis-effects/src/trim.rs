use auralis_core::{AudioBuffer, FrameCount};

use crate::{EffectError, Result};

/// A SoX-ng-style trim position.
///
/// Positions are measured in decoded audio frames. A trim command toggles
/// between discarding and copying audio at each resolved position.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TrimPosition {
    /// Absolute frame position from the start of input.
    Absolute(FrameCount),

    /// Frame offset from the previously resolved position.
    Relative(FrameCount),

    /// Frame offset before the end of input.
    BeforeEnd(FrameCount),

    /// The end of input, equivalent to SoX-ng's `-0` form.
    End,
}

impl TrimPosition {
    /// Creates an absolute trim position.
    #[must_use]
    pub const fn absolute(frame: FrameCount) -> Self {
        Self::Absolute(frame)
    }

    /// Creates a position relative to the previously resolved position.
    #[must_use]
    pub const fn relative(frames: FrameCount) -> Self {
        Self::Relative(frames)
    }

    /// Creates a position relative to the end of input.
    #[must_use]
    pub const fn before_end(frames: FrameCount) -> Self {
        Self::BeforeEnd(frames)
    }
}

/// Frame-range trim effect processor.
///
/// `Trim` keeps one or more half-open frame ranges from every channel and
/// returns a new planar `f32` [`AudioBuffer`]. Ranges are measured in frames,
/// not individual samples, so stereo and larger channel layouts preserve frame
/// grouping. `start == end` is valid and produces an empty buffer with the same
/// audio specification. Multiple positions follow SoX-ng's trim model: the
/// first position starts copying, the second stops, and later positions
/// alternate between copying and discarding.
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
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Trim {
    /// First frame to keep.
    pub start: FrameCount,

    /// End-exclusive frame index.
    pub end: FrameCount,

    positions: Vec<TrimPosition>,
}

impl Trim {
    /// Creates a frame-range trim processor.
    ///
    /// # Errors
    ///
    /// Returns [`EffectError::InvalidTrimOrder`] when `start > end`.
    pub fn new(start: FrameCount, end: FrameCount) -> Result<Self> {
        if start <= end {
            Ok(Self {
                start,
                end,
                positions: vec![
                    TrimPosition::Absolute(start),
                    TrimPosition::Relative(FrameCount::new(end.as_u64() - start.as_u64())),
                ],
            })
        } else {
            Err(EffectError::InvalidTrimOrder)
        }
    }

    /// Creates a trim processor from SoX-ng-style positions.
    ///
    /// The first position starts copying, the second stops, and following
    /// positions continue alternating. Plain SoX-ng positions after the first
    /// should be represented as [`TrimPosition::Relative`].
    ///
    /// # Errors
    ///
    /// Returns [`EffectError::InvalidTrimOrder`] when no positions are given.
    pub fn with_positions<I>(positions: I) -> Result<Self>
    where
        I: IntoIterator<Item = TrimPosition>,
    {
        let positions: Vec<TrimPosition> = positions.into_iter().collect();
        if positions.is_empty() {
            return Err(EffectError::InvalidTrimOrder);
        }

        let (start, end) = preview_first_range(&positions);
        Ok(Self {
            start,
            end,
            positions,
        })
    }

    /// Returns the configured SoX-ng-style trim positions.
    #[must_use]
    pub fn positions(&self) -> &[TrimPosition] {
        &self.positions
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
    pub fn process_buffer(&self, audio: &AudioBuffer) -> Result<AudioBuffer> {
        let ranges = self.resolved_ranges(audio.frames())?;
        let output_frame_count = ranges.iter().try_fold(0_u64, |total, (start, end)| {
            total
                .checked_add(end.as_u64() - start.as_u64())
                .ok_or(EffectError::TrimRangeOutOfBounds)
        })?;
        let output_frames = FrameCount::new(output_frame_count);
        let output_frames_usize =
            usize::try_from(output_frame_count).map_err(|_| EffectError::TrimRangeOutOfBounds)?;
        let capacity = audio
            .channels()
            .as_usize()
            .checked_mul(output_frames_usize)
            .ok_or(EffectError::TrimRangeOutOfBounds)?;
        let mut output = Vec::with_capacity(capacity);

        for channel_index in 0..audio.channels().as_usize() {
            let channel = audio
                .channel(channel_index)
                .ok_or(EffectError::TrimRangeOutOfBounds)?;
            for (start, end) in &ranges {
                let start = usize::try_from(start.as_u64())
                    .map_err(|_| EffectError::TrimRangeOutOfBounds)?;
                let end =
                    usize::try_from(end.as_u64()).map_err(|_| EffectError::TrimRangeOutOfBounds)?;
                output.extend_from_slice(&channel[start..end]);
            }
        }

        Ok(AudioBuffer::from_planar_f32(
            audio.spec(),
            output_frames,
            output,
        )?)
    }

    /// Applies the trim directly to an existing audio buffer.
    ///
    /// # Errors
    ///
    /// Returns [`EffectError::TrimRangeOutOfBounds`] when the resolved ranges
    /// are outside the input buffer or cannot be represented.
    pub fn process_buffer_in_place(&self, audio: &mut AudioBuffer) -> Result<()> {
        let ranges = self.resolved_ranges(audio.frames())?;
        audio.retain_frame_ranges(&ranges)
            .map_err(|_| EffectError::TrimRangeOutOfBounds)
    }

    fn resolved_ranges(&self, input_frames: FrameCount) -> Result<Vec<(FrameCount, FrameCount)>> {
        let positions = self.resolved_positions(input_frames)?;
        let mut ranges = Vec::new();
        let mut index = 0;

        while index < positions.len() {
            let start = positions[index];
            let end = positions.get(index + 1).copied().unwrap_or(input_frames);
            if start > input_frames || end > input_frames {
                return Err(EffectError::TrimRangeOutOfBounds);
            }
            ranges.push((start, end));
            index += 2;
        }

        Ok(ranges)
    }

    fn resolved_positions(&self, input_frames: FrameCount) -> Result<Vec<FrameCount>> {
        let mut latest = FrameCount::new(0);
        let mut resolved = Vec::with_capacity(self.positions.len());

        for position in &self.positions {
            let frame = match *position {
                TrimPosition::Absolute(frame) => frame,
                TrimPosition::Relative(offset) => FrameCount::new(
                    latest
                        .as_u64()
                        .checked_add(offset.as_u64())
                        .ok_or(EffectError::TrimRangeOutOfBounds)?,
                ),
                TrimPosition::BeforeEnd(offset) => {
                    FrameCount::new(input_frames.as_u64().saturating_sub(offset.as_u64()))
                }
                TrimPosition::End => input_frames,
            };
            if frame < latest {
                return Err(EffectError::InvalidTrimOrder);
            }
            latest = frame;
            resolved.push(frame);
        }

        Ok(resolved)
    }
}

fn preview_first_range(positions: &[TrimPosition]) -> (FrameCount, FrameCount) {
    let mut latest = FrameCount::new(0);
    let mut resolved = Vec::new();

    for position in positions.iter().take(2) {
        let frame = match *position {
            TrimPosition::Absolute(frame) => frame,
            TrimPosition::Relative(offset) => {
                FrameCount::new(latest.as_u64().saturating_add(offset.as_u64()))
            }
            TrimPosition::BeforeEnd(_) | TrimPosition::End => break,
        };
        latest = frame;
        resolved.push(frame);
    }

    let start = resolved.first().copied().unwrap_or(FrameCount::new(0));
    let end = resolved.get(1).copied().unwrap_or(start);
    (start, end)
}

#[cfg(test)]
mod tests {
    use super::{Trim, TrimPosition};
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
    fn trim_open_ended_range_keeps_tail() {
        let audio = audio_buffer(vec![0.0, 0.25, 0.5, 0.75]);

        let trimmed = Trim::with_positions([TrimPosition::absolute(FrameCount::new(2))])
            .unwrap()
            .process_buffer(&audio)
            .unwrap();

        assert_eq!(trimmed.as_planar_f32(), &[0.5, 0.75]);
    }

    #[test]
    fn trim_multiple_ranges_are_concatenated_per_channel() {
        let audio = stereo_audio_buffer(vec![0.0, 0.1, 0.2, 0.3, 0.4, 1.0, 1.1, 1.2, 1.3, 1.4]);

        let trimmed = Trim::with_positions([
            TrimPosition::absolute(FrameCount::new(1)),
            TrimPosition::relative(FrameCount::new(2)),
            TrimPosition::absolute(FrameCount::new(4)),
        ])
        .unwrap()
        .process_buffer(&audio)
        .unwrap();

        assert_eq!(trimmed.frames(), FrameCount::new(3));
        assert_eq!(trimmed.as_planar_f32(), &[0.1, 0.2, 0.4, 1.1, 1.2, 1.4]);
    }

    #[test]
    fn trim_end_relative_position_uses_input_length() {
        let audio = audio_buffer(vec![0.0, 0.25, 0.5, 0.75, 1.0]);

        let trimmed = Trim::with_positions([
            TrimPosition::absolute(FrameCount::new(1)),
            TrimPosition::relative(FrameCount::new(1)),
            TrimPosition::before_end(FrameCount::new(2)),
        ])
        .unwrap()
        .process_buffer(&audio)
        .unwrap();

        assert_eq!(trimmed.as_planar_f32(), &[0.25, 0.75, 1.0]);
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

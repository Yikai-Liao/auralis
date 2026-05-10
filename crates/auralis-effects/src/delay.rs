use auralis_core::{AudioBuffer, FrameCount};

use crate::{EffectError, Result};

/// Anchor used when resolving a SoX-ng-style delay position.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DelayAnchor {
    /// Position is measured from the start of the input.
    Start,
    /// Position is measured from the previously resolved delay position.
    Previous,
    /// Position is measured backward from the end of the input.
    End,
}

/// Amount used by a SoX-ng-style delay position.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum DelayAmount {
    /// A direct frame count, matching SoX-ng's `s` suffix.
    Frames(FrameCount),
    /// Seconds resolved using the input sample rate.
    Seconds(f64),
}

/// One channel delay position.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct DelayPosition {
    anchor: DelayAnchor,
    amount: DelayAmount,
}

impl DelayPosition {
    /// Creates an absolute frame-count delay.
    #[must_use]
    pub const fn frames(frames: FrameCount) -> Self {
        Self {
            anchor: DelayAnchor::Start,
            amount: DelayAmount::Frames(frames),
        }
    }

    /// Creates an absolute seconds-based delay.
    ///
    /// # Errors
    ///
    /// Returns [`EffectError::InvalidDelayPosition`] when `seconds` is not
    /// finite or is negative.
    pub fn seconds(seconds: f64) -> Result<Self> {
        validate_seconds(seconds)?;
        Ok(Self {
            anchor: DelayAnchor::Start,
            amount: DelayAmount::Seconds(seconds),
        })
    }

    /// Creates a position with an explicit anchor and amount.
    #[must_use]
    pub const fn new(anchor: DelayAnchor, amount: DelayAmount) -> Self {
        Self { anchor, amount }
    }

    /// Returns the position anchor.
    #[must_use]
    pub const fn anchor(self) -> DelayAnchor {
        self.anchor
    }

    /// Returns the unresolved position amount.
    #[must_use]
    pub const fn amount(self) -> DelayAmount {
        self.amount
    }

    pub(crate) fn resolved(
        self,
        sample_rate_hz: u32,
        previous: FrameCount,
        input_frames: FrameCount,
    ) -> Result<FrameCount> {
        let offset = self.amount.resolved_frames(sample_rate_hz)?;
        let resolved = match self.anchor {
            DelayAnchor::Start => offset,
            DelayAnchor::Previous => previous
                .as_u64()
                .checked_add(offset.as_u64())
                .ok_or(EffectError::DelayLengthOverflow)
                .map(FrameCount::new)?,
            DelayAnchor::End => {
                FrameCount::new(input_frames.as_u64().saturating_sub(offset.as_u64()))
            }
        };

        Ok(resolved)
    }
}

impl DelayAmount {
    #[allow(
        clippy::cast_possible_truncation,
        clippy::cast_precision_loss,
        clippy::cast_sign_loss,
        reason = "seconds are validated non-negative and finite before SoX-ng-style rounding"
    )]
    fn resolved_frames(self, sample_rate_hz: u32) -> Result<FrameCount> {
        match self {
            Self::Frames(frames) => Ok(frames),
            Self::Seconds(seconds) => {
                validate_seconds(seconds)?;
                let frames = seconds.mul_add(f64::from(sample_rate_hz), 0.5).floor();
                if frames > u64::MAX as f64 {
                    return Err(EffectError::DelayLengthOverflow);
                }
                Ok(FrameCount::new(frames as u64))
            }
        }
    }
}

/// SoX-ng-style channel delay.
///
/// `Delay` delays the first channel by the first configured position, the
/// second channel by the second position, and so on. Channels without an
/// explicit position use zero delay. The output is extended by the largest
/// resolved channel delay, so shorter-delay channels receive trailing silence
/// to preserve planar channel alignment.
///
/// # Errors
///
/// [`Self::process_buffer`] returns [`EffectError::DelayTooManyPositions`] when
/// more positions than input channels are provided, or
/// [`EffectError::DelayLengthOverflow`] when the output shape cannot be
/// represented.
///
/// # Examples
///
/// ```
/// use auralis_core::{
///     AudioBuffer, AudioSpec, ChannelCount, FrameCount, SampleFormat, SampleRate,
/// };
/// use auralis_effects::{Delay, DelayPosition};
///
/// let spec = AudioSpec::new(
///     SampleRate::new(48_000)?,
///     ChannelCount::new(1)?,
///     SampleFormat::Float32,
/// );
/// let audio = AudioBuffer::from_planar_f32(spec, FrameCount::new(2), vec![0.25, 0.5])?;
///
/// let delayed = Delay::new([FrameCount::new(1)]).process_buffer(&audio)?;
///
/// assert_eq!(delayed.as_planar_f32(), &[0.0, 0.25, 0.5]);
/// assert_eq!(DelayPosition::frames(FrameCount::new(1)).anchor(), auralis_effects::DelayAnchor::Start);
/// # Ok::<(), Box<dyn std::error::Error>>(())
/// ```
#[derive(Debug, Clone, PartialEq)]
pub struct Delay {
    positions: Vec<DelayPosition>,
}

impl Delay {
    /// Creates a delay from absolute frame counts.
    pub fn new<I>(frames_by_channel: I) -> Self
    where
        I: IntoIterator<Item = FrameCount>,
    {
        Self {
            positions: frames_by_channel
                .into_iter()
                .map(DelayPosition::frames)
                .collect(),
        }
    }

    /// Creates a delay from SoX-ng-style positions.
    pub fn with_positions<I>(positions: I) -> Self
    where
        I: IntoIterator<Item = DelayPosition>,
    {
        Self {
            positions: positions.into_iter().collect(),
        }
    }

    /// Returns the configured channel positions.
    #[must_use]
    pub fn positions(&self) -> &[DelayPosition] {
        &self.positions
    }

    /// Applies channel delay and returns the extended output buffer.
    ///
    /// # Errors
    ///
    /// Returns [`EffectError::DelayTooManyPositions`] when there are more
    /// configured positions than input channels, or
    /// [`EffectError::DelayLengthOverflow`] when the resolved output shape
    /// cannot be represented.
    pub fn process_buffer(&self, audio: &AudioBuffer) -> Result<AudioBuffer> {
        let delays = self.resolved_delays(audio)?;
        let max_delay = delays.iter().map(|delay| delay.as_u64()).max().unwrap_or(0);
        if max_delay == 0 {
            return Ok(audio.clone());
        }

        let output_frames = audio
            .frames()
            .as_u64()
            .checked_add(max_delay)
            .map(FrameCount::new)
            .ok_or(EffectError::DelayLengthOverflow)?;
        let output_frames_usize = usize::try_from(output_frames.as_u64())
            .map_err(|_| EffectError::DelayLengthOverflow)?;
        let capacity = audio
            .channels()
            .as_usize()
            .checked_mul(output_frames_usize)
            .ok_or(EffectError::DelayLengthOverflow)?;
        let mut output = Vec::with_capacity(capacity);

        for channel_index in 0..audio.channels().as_usize() {
            let delay = delays
                .get(channel_index)
                .copied()
                .unwrap_or(FrameCount::new(0));
            let delay_usize =
                usize::try_from(delay.as_u64()).map_err(|_| EffectError::DelayLengthOverflow)?;
            let channel = audio
                .channel(channel_index)
                .ok_or(EffectError::DelayLengthOverflow)?;
            output.resize(output.len() + delay_usize, 0.0);
            output.extend_from_slice(channel);
            output.resize((channel_index + 1) * output_frames_usize, 0.0);
        }

        Ok(AudioBuffer::from_planar_f32(
            audio.spec(),
            output_frames,
            output,
        )?)
    }

    fn resolved_delays(&self, audio: &AudioBuffer) -> Result<Vec<FrameCount>> {
        if self.positions.len() > audio.channels().as_usize() {
            return Err(EffectError::DelayTooManyPositions);
        }

        let mut previous = FrameCount::new(0);
        let mut delays = Vec::with_capacity(self.positions.len());
        for position in &self.positions {
            let delay = position.resolved(
                audio.spec().sample_rate().as_u32(),
                previous,
                audio.frames(),
            )?;
            previous = delay;
            delays.push(delay);
        }

        Ok(delays)
    }
}

fn validate_seconds(seconds: f64) -> Result<()> {
    if seconds.is_finite() && seconds >= 0.0 {
        Ok(())
    } else {
        Err(EffectError::InvalidDelayPosition)
    }
}

#[cfg(test)]
mod tests {
    use super::{Delay, DelayAmount, DelayAnchor, DelayPosition};
    use crate::{EffectError, test_support::stereo_audio_buffer};
    use auralis_core::{FrameCount, SampleRate};

    #[test]
    fn per_channel_delay_extends_to_maximum_delay() {
        let audio = stereo_audio_buffer(vec![0.25, 0.5, -0.25, -0.5]);

        let delayed = Delay::new([FrameCount::new(2), FrameCount::new(1)])
            .process_buffer(&audio)
            .unwrap();

        assert_eq!(delayed.frames(), FrameCount::new(4));
        assert_eq!(
            delayed.as_planar_f32(),
            &[0.0, 0.0, 0.25, 0.5, 0.0, -0.25, -0.5, 0.0]
        );
    }

    #[test]
    fn channels_without_position_get_trailing_silence_only() {
        let audio = stereo_audio_buffer(vec![0.25, 0.5, -0.25, -0.5]);

        let delayed = Delay::new([FrameCount::new(1)])
            .process_buffer(&audio)
            .unwrap();

        assert_eq!(delayed.as_planar_f32(), &[0.0, 0.25, 0.5, -0.25, -0.5, 0.0]);
    }

    #[test]
    fn seconds_resolve_with_input_sample_rate() {
        let position = DelayPosition::seconds(0.25).unwrap();
        let frames = position
            .resolved(
                SampleRate::new(48_000).unwrap().as_u32(),
                FrameCount::new(0),
                FrameCount::new(100),
            )
            .unwrap();

        assert_eq!(frames, FrameCount::new(12_000));
    }

    #[test]
    fn relative_and_end_anchors_resolve_against_context() {
        let relative = DelayPosition::new(
            DelayAnchor::Previous,
            DelayAmount::Frames(FrameCount::new(3)),
        );
        let before_end =
            DelayPosition::new(DelayAnchor::End, DelayAmount::Frames(FrameCount::new(2)));

        assert_eq!(
            relative
                .resolved(48_000, FrameCount::new(4), FrameCount::new(10))
                .unwrap(),
            FrameCount::new(7)
        );
        assert_eq!(
            before_end
                .resolved(48_000, FrameCount::new(0), FrameCount::new(10))
                .unwrap(),
            FrameCount::new(8)
        );
    }

    #[test]
    fn rejects_more_positions_than_input_channels() {
        let audio = stereo_audio_buffer(vec![0.25, -0.25]);

        let error = Delay::new([FrameCount::new(1), FrameCount::new(2), FrameCount::new(3)])
            .process_buffer(&audio)
            .unwrap_err();

        assert_eq!(error, EffectError::DelayTooManyPositions);
    }
}

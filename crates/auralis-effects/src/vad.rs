use auralis_core::{AudioBuffer, FrameCount};

use crate::{EffectError, Result};

/// A deterministic whole-buffer voice activity detector core.
///
/// This processor removes leading non-voice material, optionally retaining a
/// fixed pre-trigger span before the detected voice. It is intentionally a
/// typed core only: command parsing and SoX-ng option compatibility are owned
/// by the later VAD options feature.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Vad {
    trigger_threshold: f32,
    trigger_frames: FrameCount,
    pre_trigger: FrameCount,
    allowed_gap: FrameCount,
}

impl Vad {
    /// Creates a VAD core with explicit frame-domain settings.
    ///
    /// `trigger_threshold` is a normalized full-scale absolute sample
    /// threshold. Detection triggers after `trigger_frames` voice frames have
    /// been observed, tolerating up to `allowed_gap` quiet frames after the
    /// first voice frame. `pre_trigger` frames before the detected start are
    /// retained in the output.
    ///
    /// # Errors
    ///
    /// Returns [`EffectError::InvalidVad`] when the threshold is not finite or
    /// outside `0..=1`.
    pub fn new(
        trigger_threshold: f32,
        trigger_frames: FrameCount,
        pre_trigger: FrameCount,
        allowed_gap: FrameCount,
    ) -> Result<Self> {
        if !trigger_threshold.is_finite() || !(0.0..=1.0).contains(&trigger_threshold) {
            return Err(EffectError::InvalidVad);
        }

        Ok(Self {
            trigger_threshold,
            trigger_frames,
            pre_trigger,
            allowed_gap,
        })
    }

    /// Creates the default core profile.
    #[must_use]
    pub const fn default_profile() -> Self {
        Self {
            trigger_threshold: 0.02,
            trigger_frames: FrameCount::new(1),
            pre_trigger: FrameCount::new(0),
            allowed_gap: FrameCount::new(0),
        }
    }

    /// Returns the normalized full-scale voice threshold.
    #[must_use]
    pub const fn trigger_threshold(&self) -> f32 {
        self.trigger_threshold
    }

    /// Returns the number of detected voice frames required to trigger.
    #[must_use]
    pub const fn trigger_frames(&self) -> FrameCount {
        self.trigger_frames
    }

    /// Returns the number of frames retained before the detected voice start.
    #[must_use]
    pub const fn pre_trigger(&self) -> FrameCount {
        self.pre_trigger
    }

    /// Returns the number of quiet frames tolerated inside the trigger search.
    #[must_use]
    pub const fn allowed_gap(&self) -> FrameCount {
        self.allowed_gap
    }

    /// Applies leading VAD trimming and returns the output buffer.
    ///
    /// If no voice is detected, the result is an empty buffer with the same
    /// audio specification. Once voice is detected, the processor copies from
    /// the retained trigger point through the end of the input.
    ///
    /// # Errors
    ///
    /// Returns [`EffectError::VadLengthOverflow`] when the input or output
    /// shape cannot be represented by the current platform.
    pub fn process_buffer(&self, audio: &AudioBuffer) -> Result<AudioBuffer> {
        let frames =
            usize::try_from(audio.frames().as_u64()).map_err(|_| EffectError::VadLengthOverflow)?;
        let channels = audio.channels().as_usize();
        let start = self.detect_start(audio, frames)?;
        let output_frames = frames
            .checked_sub(start)
            .ok_or(EffectError::VadLengthOverflow)?;
        let capacity = output_frames
            .checked_mul(channels)
            .ok_or(EffectError::VadLengthOverflow)?;
        let mut output = Vec::with_capacity(capacity);

        for channel_index in 0..channels {
            let channel = audio
                .channel(channel_index)
                .ok_or(EffectError::VadLengthOverflow)?;
            output.extend_from_slice(&channel[start..]);
        }

        AudioBuffer::from_planar_f32(
            audio.spec(),
            FrameCount::new(
                u64::try_from(output_frames).map_err(|_| EffectError::VadLengthOverflow)?,
            ),
            output,
        )
        .map_err(|_| EffectError::VadLengthOverflow)
    }

    fn detect_start(&self, audio: &AudioBuffer, frames: usize) -> Result<usize> {
        let required = usize::try_from(self.trigger_frames.as_u64().max(1))
            .map_err(|_| EffectError::VadLengthOverflow)?;
        let allowed_gap = usize::try_from(self.allowed_gap.as_u64())
            .map_err(|_| EffectError::VadLengthOverflow)?;
        let pre_trigger = usize::try_from(self.pre_trigger.as_u64())
            .map_err(|_| EffectError::VadLengthOverflow)?;
        let mut run_start = 0;
        let mut active_count = 0;
        let mut gap_count = 0;

        for frame in 0..frames {
            if self.frame_is_voice(audio, frame) {
                if active_count == 0 {
                    run_start = frame;
                }
                active_count += 1;
                gap_count = 0;
                if active_count >= required {
                    return Ok(run_start.saturating_sub(pre_trigger));
                }
            } else if active_count > 0 && gap_count < allowed_gap {
                gap_count += 1;
            } else {
                active_count = 0;
                gap_count = 0;
            }
        }

        Ok(frames)
    }

    fn frame_is_voice(&self, audio: &AudioBuffer, frame: usize) -> bool {
        (0..audio.channels().as_usize()).any(|channel| {
            audio
                .sample(channel, frame)
                .is_some_and(|sample| sample.abs() >= self.trigger_threshold)
        })
    }
}

impl Default for Vad {
    fn default() -> Self {
        Self::default_profile()
    }
}

#[cfg(test)]
mod tests {
    use auralis_core::{AudioSpec, ChannelCount, SampleFormat, SampleRate};

    use super::Vad;
    use crate::EffectError;

    #[test]
    fn rejects_invalid_thresholds() {
        assert_eq!(
            Vad::new(
                f32::NAN,
                default_frames(1),
                default_frames(0),
                default_frames(0)
            )
            .unwrap_err(),
            EffectError::InvalidVad
        );
        assert_eq!(
            Vad::new(1.1, default_frames(1), default_frames(0), default_frames(0)).unwrap_err(),
            EffectError::InvalidVad
        );
    }

    #[test]
    fn default_profile_uses_single_frame_trigger() {
        let vad = Vad::default();

        assert_eq!(vad.trigger_threshold().to_bits(), 0.02_f32.to_bits());
        assert_eq!(vad.trigger_frames().as_u64(), 1);
        assert_eq!(vad.pre_trigger().as_u64(), 0);
        assert_eq!(vad.allowed_gap().as_u64(), 0);
    }

    fn default_frames(frames: u64) -> auralis_core::FrameCount {
        auralis_core::FrameCount::new(frames)
    }

    #[allow(
        dead_code,
        reason = "keeps rustdoc links in this module exercised during refactors"
    )]
    fn _spec() -> AudioSpec {
        AudioSpec::new(
            SampleRate::new(48_000).unwrap(),
            ChannelCount::new(1).unwrap(),
            SampleFormat::Float32,
        )
    }
}

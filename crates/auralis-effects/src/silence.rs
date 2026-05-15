use auralis_core::{AudioBuffer, FrameCount};

use crate::{EffectError, Result};

/// A SoX-ng-style silence duration.
///
/// Bare command numbers and the `s` suffix are frame counts; decimal, colon,
/// and `t`-suffixed values are seconds resolved using the input sample rate.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum SilenceDuration {
    /// A direct frame count.
    Frames(FrameCount),
    /// Seconds resolved at processing time.
    Seconds(f64),
}

/// Threshold unit used to classify silence.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum SilenceThreshold {
    /// Percentage of full scale in `0..=100`.
    Percent(f64),
    /// Negative dBFS threshold.
    Decibels(f64),
}

/// A non-silence or silence period detector.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SilencePeriod {
    /// Number of qualifying periods to find.
    pub periods: u32,
    /// Minimum duration of a qualifying period.
    pub duration: SilenceDuration,
    /// Threshold used to classify samples.
    pub threshold: SilenceThreshold,
}

/// SoX-ng-style silence trimming.
///
/// `silence` removes leading, trailing, or middle quiet regions. Leading trim
/// starts copying after `above_periods` non-silent periods have each lasted at
/// least `duration`. Trailing trim stops or restarts when `below_periods`
/// silent periods have each lasted at least `duration`. The current Auralis
/// processor is whole-buffer and deterministic; streaming state can be added
/// later without changing this typed configuration.
#[derive(Debug, Clone, PartialEq)]
pub struct Silence {
    above_periods: u32,
    above: Option<SilencePeriod>,
    below: Option<SilencePeriod>,
    restart: bool,
    retain_quiet: bool,
}

impl Silence {
    /// Creates a silence trimmer.
    ///
    /// # Errors
    ///
    /// Returns [`EffectError::InvalidSilence`] when period counts, durations,
    /// thresholds, or option combinations do not match SoX-ng's command model.
    pub fn new(
        above_periods: u32,
        above: Option<(SilenceDuration, SilenceThreshold)>,
        below: Option<(i32, SilenceDuration, SilenceThreshold)>,
        leave_silence: bool,
    ) -> Result<Self> {
        let above = match (above_periods, above) {
            (0, None) => None,
            (0, Some(_)) | (_, None) => return Err(EffectError::InvalidSilence),
            (periods, Some((duration, threshold))) => {
                validate_duration(duration)?;
                threshold.validate()?;
                Some(SilencePeriod {
                    periods,
                    duration,
                    threshold,
                })
            }
        };

        let (below, restart) = match below {
            Some((periods, duration, threshold)) if periods != 0 => {
                validate_duration(duration)?;
                threshold.validate()?;
                (
                    Some(SilencePeriod {
                        periods: periods.unsigned_abs(),
                        duration,
                        threshold,
                    }),
                    periods < 0,
                )
            }
            Some(_) => return Err(EffectError::InvalidSilence),
            None => (None, false),
        };

        if leave_silence && below.is_none() {
            return Err(EffectError::InvalidSilence);
        }

        Ok(Self {
            above_periods,
            above,
            below,
            restart,
            retain_quiet: leave_silence,
        })
    }

    /// Creates a copy-through `silence 0` command.
    #[must_use]
    pub const fn copy_through() -> Self {
        Self {
            above_periods: 0,
            above: None,
            below: None,
            restart: false,
            retain_quiet: false,
        }
    }

    /// Returns the configured `above-periods` value.
    #[must_use]
    pub const fn above_periods(&self) -> u32 {
        self.above_periods
    }

    /// Returns the leading non-silence detector, when enabled.
    #[must_use]
    pub const fn above(&self) -> Option<SilencePeriod> {
        self.above
    }

    /// Returns the trailing or middle silence detector, when enabled.
    #[must_use]
    pub const fn below(&self) -> Option<SilencePeriod> {
        self.below
    }

    /// Returns whether negative `below-periods` restart detection after a cut.
    #[must_use]
    pub const fn restart(&self) -> bool {
        self.restart
    }

    /// Returns whether `-l` keeps the first below-duration of each silence.
    #[must_use]
    pub const fn leave_silence(&self) -> bool {
        self.retain_quiet
    }

    /// Applies whole-buffer silence trimming and returns the output.
    ///
    /// # Errors
    ///
    /// Returns [`EffectError::SilenceLengthOverflow`] when the resolved output
    /// shape cannot be represented.
    pub fn process_buffer(&self, audio: &AudioBuffer) -> Result<AudioBuffer> {
        let frames = usize::try_from(audio.frames().as_u64())
            .map_err(|_| EffectError::SilenceLengthOverflow)?;
        let channels = audio.channels().as_usize();
        let channel_data = (0..channels)
            .map(|channel| {
                audio
                    .channel(channel)
                    .ok_or(EffectError::SilenceLengthOverflow)
            })
            .collect::<Result<Vec<_>>>()?;
        let mut ranges = Vec::new();
        let mut cursor = self.resolve_start(audio, &channel_data, 0, frames)?;

        loop {
            let Some(below) = self.below else {
                if cursor < frames {
                    ranges.push(cursor..frames);
                }
                break;
            };

            let Some((silent_start, silent_end)) =
                Self::find_silence_run(audio, &channel_data, cursor, frames, below)?
            else {
                if cursor < frames {
                    ranges.push(cursor..frames);
                }
                break;
            };

            let keep_end = if self.retain_quiet {
                silent_end
            } else {
                silent_start
            };
            if cursor < keep_end {
                ranges.push(cursor..keep_end);
            }
            if !self.restart {
                break;
            }
            cursor = self.resolve_start(audio, &channel_data, silent_end, frames)?;
        }

        let output_frames = ranges.iter().try_fold(0_usize, |total, range| {
            total
                .checked_add(range.end - range.start)
                .ok_or(EffectError::SilenceLengthOverflow)
        })?;
        let capacity = channels
            .checked_mul(output_frames)
            .ok_or(EffectError::SilenceLengthOverflow)?;
        let mut output = Vec::with_capacity(capacity);

        for &channel in &channel_data {
            for range in &ranges {
                output.extend_from_slice(&channel[range.clone()]);
            }
        }

        AudioBuffer::from_planar_f32(
            audio.spec(),
            FrameCount::new(
                u64::try_from(output_frames).map_err(|_| EffectError::SilenceLengthOverflow)?,
            ),
            output,
        )
        .map_err(|_| EffectError::SilenceLengthOverflow)
    }

    fn resolve_start(
        &self,
        audio: &AudioBuffer,
        channels: &[&[f32]],
        start: usize,
        frames: usize,
    ) -> Result<usize> {
        let Some(above) = self.above else {
            return Ok(start);
        };
        let required = above
            .duration
            .resolved_frames(audio.spec().sample_rate().as_u32())?;
        let threshold = above.threshold.linear_amplitude_threshold();
        if required == 0 {
            return (start..frames)
                .find(|&frame| Self::above_threshold_frame(channels, frame, threshold))
                .ok_or(EffectError::SilenceLengthOverflow)
                .or(Ok(frames));
        }

        let mut found_periods = 0_u32;
        let mut run_start = start;
        let mut run_len = 0_usize;

        for frame in start..frames {
            if Self::above_threshold_frame(channels, frame, threshold) {
                if run_len == 0 {
                    run_start = frame;
                }
                run_len += 1;
                if run_len >= required {
                    found_periods += 1;
                    if found_periods >= above.periods {
                        return Ok(run_start);
                    }
                    run_len = 0;
                }
            } else {
                run_len = 0;
            }
        }

        Ok(frames)
    }

    fn find_silence_run(
        audio: &AudioBuffer,
        channels: &[&[f32]],
        start: usize,
        frames: usize,
        below: SilencePeriod,
    ) -> Result<Option<(usize, usize)>> {
        let required = below
            .duration
            .resolved_frames(audio.spec().sample_rate().as_u32())?;
        let threshold = below.threshold.linear_amplitude_threshold();
        let mut found_periods = 0_u32;
        let mut run_start = start;
        let mut run_len = 0_usize;

        for frame in start..frames {
            if Self::below_threshold_frame(channels, frame, threshold) {
                if run_len == 0 {
                    run_start = frame;
                }
                run_len += 1;
                if run_len >= required {
                    found_periods += 1;
                    if found_periods >= below.periods {
                        return Ok(Some((run_start, frame + 1)));
                    }
                    run_len = 0;
                }
            } else {
                run_len = 0;
            }
        }

        Ok(None)
    }

    fn above_threshold_frame(channels: &[&[f32]], frame: usize, threshold: f32) -> bool {
        channels.iter().any(|channel| {
            channel
                .get(frame)
                .is_some_and(|sample| sample.abs() > threshold)
        })
    }

    fn below_threshold_frame(channels: &[&[f32]], frame: usize, threshold: f32) -> bool {
        !channels.iter().all(|channel| {
            channel
                .get(frame)
                .is_some_and(|sample| sample.abs() > threshold)
        })
    }
}

impl Default for Silence {
    fn default() -> Self {
        Self::copy_through()
    }
}

impl SilenceDuration {
    /// Creates a direct frame-count duration.
    #[must_use]
    pub const fn frames(frames: FrameCount) -> Self {
        Self::Frames(frames)
    }

    /// Creates a seconds-based duration.
    ///
    /// # Errors
    ///
    /// Returns [`EffectError::InvalidSilence`] when `seconds` is negative or
    /// non-finite.
    pub fn seconds(seconds: f64) -> Result<Self> {
        if seconds.is_finite() && seconds >= 0.0 {
            Ok(Self::Seconds(seconds))
        } else {
            Err(EffectError::InvalidSilence)
        }
    }

    #[allow(
        clippy::cast_possible_truncation,
        clippy::cast_precision_loss,
        clippy::cast_sign_loss,
        reason = "seconds are validated non-negative and finite before SoX-ng-style rounding"
    )]
    fn resolved_frames(self, sample_rate_hz: u32) -> Result<usize> {
        let frames = match self {
            Self::Frames(frames) => frames.as_u64(),
            Self::Seconds(seconds) => {
                if !seconds.is_finite() || seconds < 0.0 {
                    return Err(EffectError::InvalidSilence);
                }
                let rounded = seconds.mul_add(f64::from(sample_rate_hz), 0.5).floor();
                if rounded > u64::MAX as f64 {
                    return Err(EffectError::SilenceLengthOverflow);
                }
                rounded as u64
            }
        };

        usize::try_from(frames).map_err(|_| EffectError::SilenceLengthOverflow)
    }
}

impl SilenceThreshold {
    /// Creates a percent-of-full-scale threshold.
    ///
    /// # Errors
    ///
    /// Returns [`EffectError::InvalidSilence`] when `percent` is not finite or
    /// falls outside `0..=100`.
    pub fn percent(percent: f64) -> Result<Self> {
        let threshold = Self::Percent(percent);
        threshold.validate()?;
        Ok(threshold)
    }

    /// Creates a dBFS threshold.
    ///
    /// # Errors
    ///
    /// Returns [`EffectError::InvalidSilence`] when `db` is not finite or is
    /// greater than or equal to zero.
    pub fn decibels(db: f64) -> Result<Self> {
        let threshold = Self::Decibels(db);
        threshold.validate()?;
        Ok(threshold)
    }

    fn validate(self) -> Result<()> {
        match self {
            Self::Percent(percent) if percent.is_finite() && (0.0..=100.0).contains(&percent) => {
                Ok(())
            }
            Self::Decibels(db) if db.is_finite() && db < 0.0 => Ok(()),
            _ => Err(EffectError::InvalidSilence),
        }
    }

    #[allow(
        clippy::cast_possible_truncation,
        reason = "silence thresholds are compared against normalized f32 samples"
    )]
    fn linear_amplitude_threshold(self) -> f32 {
        match self {
            Self::Percent(percent) => (percent / 100.0) as f32,
            Self::Decibels(db) => 10.0_f64.powf(db / 20.0) as f32,
        }
    }
}

fn validate_duration(duration: SilenceDuration) -> Result<()> {
    match duration {
        SilenceDuration::Frames(_) => Ok(()),
        SilenceDuration::Seconds(seconds) if seconds.is_finite() && seconds >= 0.0 => Ok(()),
        SilenceDuration::Seconds(_) => Err(EffectError::InvalidSilence),
    }
}

#[cfg(test)]
mod tests {
    use auralis_core::FrameCount;

    use super::{Silence, SilenceDuration, SilenceThreshold};
    use crate::{EffectError, test_support::audio_buffer};

    #[test]
    fn trims_leading_silence_after_required_non_silent_duration() {
        let audio = audio_buffer(vec![0.0, 0.0, 0.25, 0.5, 0.0]);
        let silence = Silence::new(
            1,
            Some((
                SilenceDuration::frames(FrameCount::new(2)),
                SilenceThreshold::percent(0.0).unwrap(),
            )),
            None,
            false,
        )
        .unwrap();

        let trimmed = silence.process_buffer(&audio).unwrap();

        assert_eq!(trimmed.as_planar_f32(), &[0.25, 0.5, 0.0]);
    }

    #[test]
    fn trims_from_first_qualifying_trailing_silence() {
        let audio = audio_buffer(vec![0.25, 0.5, 0.0, 0.0, 0.75]);
        let silence = Silence::new(
            0,
            None,
            Some((
                1,
                SilenceDuration::frames(FrameCount::new(2)),
                SilenceThreshold::percent(0.0).unwrap(),
            )),
            false,
        )
        .unwrap();

        let trimmed = silence.process_buffer(&audio).unwrap();

        assert_eq!(trimmed.as_planar_f32(), &[0.25, 0.5]);
    }

    #[test]
    fn restart_removes_middle_silence_and_leaves_requested_duration() {
        let audio = audio_buffer(vec![0.0, 0.25, 0.5, 0.0, 0.0, 0.75, 0.25]);
        let silence = Silence::new(
            1,
            Some((
                SilenceDuration::frames(FrameCount::new(1)),
                SilenceThreshold::percent(0.0).unwrap(),
            )),
            Some((
                -1,
                SilenceDuration::frames(FrameCount::new(1)),
                SilenceThreshold::percent(0.0).unwrap(),
            )),
            true,
        )
        .unwrap();

        let trimmed = silence.process_buffer(&audio).unwrap();

        assert_eq!(trimmed.as_planar_f32(), &[0.25, 0.5, 0.0, 0.75, 0.25]);
    }

    #[test]
    fn rejects_invalid_thresholds_and_option_combinations() {
        assert_eq!(
            Silence::new(0, None, None, true).unwrap_err(),
            EffectError::InvalidSilence
        );
        assert_eq!(
            SilenceThreshold::percent(101.0).unwrap_err(),
            EffectError::InvalidSilence
        );
        assert_eq!(
            SilenceThreshold::decibels(0.0).unwrap_err(),
            EffectError::InvalidSilence
        );
    }
}

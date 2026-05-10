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
    sox_options: Option<VadOptions>,
}

/// SoX-ng-style VAD command options.
///
/// Auralis stores the complete validated option profile so command parsing and
/// rendering are stable, then maps the detection timing onto the deterministic
/// frame-domain VAD core when audio is processed.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct VadOptions {
    /// Initial noise bootstrapping time in seconds.
    pub boot_time: f64,
    /// Noise estimate rise time constant in seconds.
    pub noise_tc_up: f64,
    /// Noise estimate fall time constant in seconds.
    pub noise_tc_down: f64,
    /// Noise-reduction amount.
    pub noise_reduction_amount: f64,
    /// Measurement frequency in Hz.
    pub measure_frequency: f64,
    /// Measurement duration in seconds.
    pub measure_duration: f64,
    /// Measurement time constant in seconds.
    pub measure_tc: f64,
    /// Spectral high-pass frequency in Hz.
    pub high_pass_frequency: f64,
    /// Spectral low-pass frequency in Hz.
    pub low_pass_frequency: f64,
    /// Cepstral high-pass lifter frequency in Hz.
    pub high_pass_lifter_frequency: f64,
    /// Cepstral low-pass lifter frequency in Hz.
    pub low_pass_lifter_frequency: f64,
    /// Trigger smoothing time constant in seconds.
    pub trigger_time: f64,
    /// Trigger level in SoX-ng's `0..=20` range.
    pub trigger_level: f64,
    /// Search time in seconds.
    pub search_time: f64,
    /// Quiet gap tolerated while searching in seconds.
    pub gap_time: f64,
    /// Pre-trigger audio retained before the detected start in seconds.
    pub pre_trigger_time: f64,
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
            sox_options: None,
        })
    }

    /// Creates a VAD command profile from validated SoX-ng-style options.
    ///
    /// # Errors
    ///
    /// Returns [`EffectError::InvalidVad`] when any option is outside the
    /// SoX-ng-compatible range accepted by Auralis.
    pub fn from_sox_options(options: VadOptions) -> Result<Self> {
        options.validate()?;
        Ok(Self {
            trigger_threshold: 0.02,
            trigger_frames: FrameCount::new(1),
            pre_trigger: FrameCount::new(0),
            allowed_gap: FrameCount::new(0),
            sox_options: Some(options),
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
            sox_options: None,
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

    /// Returns the SoX-ng command option profile, if this VAD was constructed
    /// from command options.
    #[must_use]
    pub const fn sox_options(&self) -> Option<VadOptions> {
        self.sox_options
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
        let resolved = self.resolve_for_sample_rate(audio.spec().sample_rate().as_u32())?;
        let frames =
            usize::try_from(audio.frames().as_u64()).map_err(|_| EffectError::VadLengthOverflow)?;
        let channels = audio.channels().as_usize();
        let start = resolved.detect_start(audio, frames)?;
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

    #[allow(
        clippy::cast_possible_truncation,
        reason = "validated SoX-ng trigger level is in 0..=20 before mapping to normalized f32"
    )]
    fn resolve_for_sample_rate(&self, sample_rate: u32) -> Result<Self> {
        let Some(options) = self.sox_options else {
            return Ok(*self);
        };
        let threshold = (options.trigger_level / 20.0) as f32;
        Self::new(
            threshold,
            frames_from_measure_time(options.trigger_time, options.measure_frequency, 1)?,
            frames_from_seconds(options.pre_trigger_time, sample_rate, 0)?,
            frames_from_measure_time(options.gap_time, options.measure_frequency, 0)?,
        )
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

impl VadOptions {
    /// Returns SoX-ng-compatible default VAD options.
    #[must_use]
    pub const fn sox_defaults() -> Self {
        Self {
            boot_time: 0.35,
            noise_tc_up: 0.1,
            noise_tc_down: 0.01,
            noise_reduction_amount: 1.35,
            measure_frequency: 20.0,
            measure_duration: 0.1,
            measure_tc: 0.4,
            high_pass_frequency: 50.0,
            low_pass_frequency: 6000.0,
            high_pass_lifter_frequency: 150.0,
            low_pass_lifter_frequency: 2000.0,
            trigger_time: 0.25,
            trigger_level: 7.0,
            search_time: 1.0,
            gap_time: 0.25,
            pre_trigger_time: 0.0,
        }
    }

    /// Validates the option profile against SoX-ng's accepted numeric ranges.
    ///
    /// # Errors
    ///
    /// Returns [`EffectError::InvalidVad`] when any value is non-finite or out
    /// of range.
    pub fn validate(self) -> Result<()> {
        validate_range(self.boot_time, 0.1, 10.0)?;
        validate_range(self.noise_tc_up, 0.1, 10.0)?;
        validate_range(self.noise_tc_down, 0.001, 0.1)?;
        validate_range(self.noise_reduction_amount, 0.0, 2.0)?;
        validate_range(self.measure_frequency, 5.0, 50.0)?;
        validate_range(self.measure_duration, 0.01, 1.0)?;
        validate_range(self.measure_tc, 0.1, 1.0)?;
        validate_range(self.high_pass_frequency, 10.0, f64::MAX)?;
        validate_range(self.low_pass_frequency, 1000.0, f64::MAX)?;
        validate_range(self.high_pass_lifter_frequency, 10.0, f64::MAX)?;
        validate_range(self.low_pass_lifter_frequency, 1000.0, f64::MAX)?;
        validate_range(self.trigger_time, 0.01, 1.0)?;
        validate_range(self.trigger_level, 0.0, 20.0)?;
        validate_range(self.search_time, 0.1, 4.0)?;
        validate_range(self.gap_time, 0.1, 1.0)?;
        validate_range(self.pre_trigger_time, 0.0, 4.0)
    }
}

impl Default for VadOptions {
    fn default() -> Self {
        Self::sox_defaults()
    }
}

fn validate_range(value: f64, min: f64, max: f64) -> Result<()> {
    if value.is_finite() && value >= min && value <= max {
        Ok(())
    } else {
        Err(EffectError::InvalidVad)
    }
}

#[allow(
    clippy::cast_possible_truncation,
    clippy::cast_precision_loss,
    clippy::cast_sign_loss,
    reason = "validated SoX-ng time ranges round to representable frame counts"
)]
fn frames_from_seconds(seconds: f64, sample_rate: u32, minimum: u64) -> Result<FrameCount> {
    let frames = (seconds * f64::from(sample_rate) + 0.5).floor();
    if frames.is_finite() && frames >= 0.0 {
        Ok(FrameCount::new((frames as u64).max(minimum)))
    } else {
        Err(EffectError::InvalidVad)
    }
}

#[allow(
    clippy::cast_possible_truncation,
    clippy::cast_precision_loss,
    clippy::cast_sign_loss,
    reason = "validated SoX-ng measurement ranges round to small frame counts"
)]
fn frames_from_measure_time(
    seconds: f64,
    measure_frequency: f64,
    minimum: u64,
) -> Result<FrameCount> {
    let frames = (seconds * measure_frequency + 0.5).floor();
    if frames.is_finite() && frames >= 0.0 {
        Ok(FrameCount::new((frames as u64).max(minimum)))
    } else {
        Err(EffectError::InvalidVad)
    }
}

impl Default for Vad {
    fn default() -> Self {
        Self::default_profile()
    }
}

#[cfg(test)]
mod tests {
    use auralis_core::{AudioBuffer, AudioSpec, ChannelCount, SampleFormat, SampleRate};

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
        assert_eq!(vad.sox_options(), None);
    }

    #[test]
    fn sox_options_validate_documented_ranges() {
        let options = super::VadOptions {
            trigger_level: 20.0,
            ..super::VadOptions::default()
        };
        assert!(options.validate().is_ok());

        let invalid = super::VadOptions {
            trigger_level: 20.1,
            ..super::VadOptions::default()
        };
        assert_eq!(invalid.validate().unwrap_err(), EffectError::InvalidVad);
    }

    #[test]
    fn sox_profile_maps_timing_at_process_time() {
        let options = super::VadOptions {
            trigger_level: 1.0,
            trigger_time: 0.01,
            pre_trigger_time: 0.000_05,
            ..super::VadOptions::default()
        };

        let vad = Vad::from_sox_options(options).unwrap();
        let source = AudioBuffer::from_planar_f32(
            test_spec(),
            auralis_core::FrameCount::new(5),
            vec![0.0, 0.0, 0.1, 0.2, 0.3],
        )
        .unwrap();
        let trimmed = vad.process_buffer(&source).unwrap();

        assert_eq!(trimmed.as_planar_f32(), &[0.0, 0.0, 0.1, 0.2, 0.3]);
    }

    fn default_frames(frames: u64) -> auralis_core::FrameCount {
        auralis_core::FrameCount::new(frames)
    }

    fn test_spec() -> AudioSpec {
        AudioSpec::new(
            SampleRate::new(48_000).unwrap(),
            ChannelCount::new(1).unwrap(),
            SampleFormat::Float32,
        )
    }
}

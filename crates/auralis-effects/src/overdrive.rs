use auralis_core::AudioBuffer;

use crate::{EffectError, Result};

const DEFAULT_GAIN_DB: f64 = 20.0;
const DEFAULT_COLOR: f64 = 20.0;
const HIGH_PASS_DECAY: f64 = 0.995;

/// SoX-ng-style overdrive distortion.
///
/// `Overdrive` first applies a gain in decibels, adds a color bias scaled as
/// `color / 200`, folds the driven sample through SoX-ng's cubic soft-clipping
/// transfer function, and then applies the same one-pole high-pass output
/// blend used by SoX-ng. The public `gain_db` and `color` values use SoX-ng's
/// documented `0..=100` range and default to `20`.
///
/// The processor is deterministic and stateful within a buffer. For
/// multi-channel buffers, each channel has independent state, matching SoX-ng's
/// channel-local effect flow. When `gain_db` is exactly `0`, the non-keymapped
/// SoX-ng effect becomes a null effect, so Auralis preserves the input
/// unchanged.
///
/// # Examples
///
/// ```
/// use auralis_effects::Overdrive;
///
/// let mut samples = [0.0, 0.25, -0.25];
/// Overdrive::new(20.0, 20.0)?.process_mono_samples(&mut samples);
///
/// assert!(samples.iter().all(|sample| sample.is_finite()));
/// # Ok::<(), auralis_effects::EffectError>(())
/// ```
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Overdrive {
    /// Gain in decibels, in SoX-ng's `0..=100` range.
    pub gain_db: f64,

    /// Even-harmonic color amount, in SoX-ng's `0..=100` range.
    pub color: f64,

    gain_linear: f64,
    color_bias: f64,
    is_null_effect: bool,
}

impl Overdrive {
    /// Creates an overdrive processor.
    ///
    /// # Errors
    ///
    /// Returns [`EffectError::InvalidOverdrive`] when either value is not
    /// finite or is outside SoX-ng's supported `0..=100` range.
    pub fn new(gain_db: f64, color: f64) -> Result<Self> {
        if !is_valid_percent(gain_db) || !is_valid_percent(color) {
            return Err(EffectError::InvalidOverdrive);
        }

        let gain_linear = 10.0_f64.powf(gain_db / 20.0);

        Ok(Self {
            gain_db,
            color,
            gain_linear,
            color_bias: color / 200.0,
            is_null_effect: gain_linear.to_bits() == 1.0_f64.to_bits(),
        })
    }

    /// Creates the SoX-ng default `overdrive` processor.
    #[must_use]
    pub fn default_settings() -> Self {
        Self {
            gain_db: DEFAULT_GAIN_DB,
            color: DEFAULT_COLOR,
            gain_linear: 10.0,
            color_bias: 0.1,
            is_null_effect: false,
        }
    }

    /// Applies overdrive to all samples in an audio buffer.
    ///
    /// # Panics
    ///
    /// Panics only if a validated [`AudioBuffer`] cannot return one of its
    /// declared channels.
    pub fn process_buffer(self, audio: &mut AudioBuffer) {
        if self.is_null_effect() {
            return;
        }

        for channel_index in 0..audio.channels().as_usize() {
            let channel = audio
                .channel_mut(channel_index)
                .expect("channel index is within the audio shape");
            let mut state = OverdriveState::new(self);
            state.process_mono_samples(channel);
        }
    }

    /// Applies overdrive to a mono sample slice.
    ///
    /// Multi-channel processing should use [`Self::process_buffer`] so the
    /// shared frame-order state matches SoX-ng.
    pub fn process_mono_samples(self, samples: &mut [f32]) {
        if self.is_null_effect() {
            return;
        }

        let mut state = OverdriveState::new(self);
        state.process_mono_samples(samples);
    }

    #[must_use]
    fn is_null_effect(self) -> bool {
        self.is_null_effect
    }
}

impl Default for Overdrive {
    fn default() -> Self {
        Self::default_settings()
    }
}

/// Stateful SoX-ng overdrive runtime for one channel.
#[derive(Debug, Clone, Copy)]
pub struct OverdriveState {
    config: Overdrive,
    last_in: f64,
    last_out: f64,
}

impl OverdriveState {
    /// Creates a runtime state for `config`.
    #[must_use]
    pub const fn new(config: Overdrive) -> Self {
        Self {
            config,
            last_in: 0.0,
            last_out: 0.0,
        }
    }

    /// Applies overdrive to a mono sample segment while preserving state.
    pub fn process_mono_samples(&mut self, samples: &mut [f32]) {
        if self.config.is_null_effect() {
            return;
        }

        for sample in samples {
            *sample = self.process_sample(*sample);
        }
    }

    #[allow(
        clippy::cast_possible_truncation,
        reason = "the public effect sample format is f32, while SoX-ng evaluates the overdrive transfer function in double precision"
    )]
    fn process_sample(&mut self, sample: f32) -> f32 {
        let dry = f64::from(sample);
        let mut driven = (dry * self.config.gain_linear) + self.config.color_bias;
        driven = shape_sample(driven);

        self.last_out = driven - self.last_in + (HIGH_PASS_DECAY * self.last_out);
        if !self.last_out.is_normal() {
            self.last_out = 0.0;
        }
        self.last_in = driven;

        ((dry * 0.5) + (self.last_out * 0.75)) as f32
    }
}

fn is_valid_percent(value: f64) -> bool {
    value.is_finite() && (0.0..=100.0).contains(&value)
}

fn shape_sample(sample: f64) -> f64 {
    if sample < -1.0 {
        -2.0 / 3.0
    } else if sample > 1.0 {
        2.0 / 3.0
    } else {
        sample - (sample * sample * sample / 3.0)
    }
}

#[cfg(test)]
mod tests {
    use super::{Overdrive, OverdriveState, shape_sample};
    use crate::EffectError;
    use auralis_core::{
        AudioBuffer, AudioSpec, ChannelCount, FrameCount, SampleFormat, SampleRate,
    };

    #[test]
    fn default_transfer_matches_sox_ng_formula() {
        let mut samples = [0.0, 0.1, -0.1];

        Overdrive::default().process_mono_samples(&mut samples);

        assert_samples_close(&samples, &[0.074_75, 0.549_626_23, -0.545_621_9]);
    }

    #[test]
    fn cubic_soft_clip_limits_driven_stage_to_two_thirds() {
        assert_close(shape_sample(2.0), 2.0 / 3.0);
        assert_close(shape_sample(-2.0), -2.0 / 3.0);
        assert_close(shape_sample(0.5), 0.458_333_333_333_333_3);
    }

    #[test]
    fn zero_db_gain_is_a_sox_ng_null_effect() {
        let mut samples = [0.0, 0.5, -0.5];

        Overdrive::new(0.0, 100.0)
            .unwrap()
            .process_mono_samples(&mut samples);

        assert_samples_close(&samples, &[0.0, 0.5, -0.5]);
    }

    #[test]
    fn stereo_processing_uses_independent_channel_state() {
        let mut audio = stereo_audio_buffer(vec![0.0, 0.1, -0.1, 0.2]);

        Overdrive::default().process_buffer(&mut audio);

        assert_samples_close(
            audio.as_planar_f32(),
            &[0.074_75, 0.549_626_23, -0.542_75, 0.602_463_7],
        );
    }

    #[test]
    fn stateful_chunked_processing_matches_whole_buffer() {
        let config = Overdrive::new(12.0, 25.0).unwrap();
        let source = stereo_audio_buffer(vec![0.0, 0.1, -0.1, 0.2, -0.2, 0.3, -0.3, 0.4]);
        let mut whole = source.clone();
        let mut chunked = source;

        config.process_buffer(&mut whole);

        for channel in 0..2 {
            let mut state = OverdriveState::new(config);
            for (chunk_start, chunk_len) in [(0, 2), (2, 1), (3, 1)] {
                let start = channel * 4 + chunk_start;
                state.process_mono_samples(
                    &mut chunked.as_planar_f32_mut()[start..start + chunk_len],
                );
            }
        }

        assert_samples_close(chunked.as_planar_f32(), whole.as_planar_f32());
    }

    #[test]
    fn invalid_values_are_rejected() {
        assert_eq!(
            Overdrive::new(-0.1, 20.0).unwrap_err(),
            EffectError::InvalidOverdrive
        );
        assert_eq!(
            Overdrive::new(20.0, 100.1).unwrap_err(),
            EffectError::InvalidOverdrive
        );
        assert_eq!(
            Overdrive::new(f64::NAN, 20.0).unwrap_err(),
            EffectError::InvalidOverdrive
        );
    }

    fn stereo_audio_buffer(samples: Vec<f32>) -> AudioBuffer {
        assert_eq!(samples.len() % 2, 0);
        let frames = u64::try_from(samples.len() / 2).unwrap();
        let spec = AudioSpec::new(
            SampleRate::new(48_000).unwrap(),
            ChannelCount::new(2).unwrap(),
            SampleFormat::Float32,
        );
        AudioBuffer::from_planar_f32(spec, FrameCount::new(frames), samples).unwrap()
    }

    fn assert_samples_close(actual: &[f32], expected: &[f32]) {
        assert_eq!(actual.len(), expected.len());
        for (index, (&actual, &expected)) in actual.iter().zip(expected).enumerate() {
            assert!(
                (actual - expected).abs() <= 0.000_001,
                "sample {index}: actual={actual}, expected={expected}"
            );
        }
    }

    fn assert_close(actual: f64, expected: f64) {
        assert!(
            (actual - expected).abs() <= 1.0e-12,
            "actual={actual}, expected={expected}"
        );
    }
}

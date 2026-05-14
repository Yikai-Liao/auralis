use auralis_core::{AudioBuffer, SampleRate};
use auralis_dsp::linear_gain;

use crate::{EffectError, Result};

const DEFAULT_VOLUME: f32 = 1.0;
const DEFAULT_DOUBLE_TIME_SECONDS: f32 = 0.0;
const DEFAULT_HEADROOM_DB: f32 = 0.0;

/// SoX-ng-style soft volume control.
///
/// `SoftVol` starts with an initial volume multiplier, reduces that multiplier
/// before any frame that would exceed the configured headroom-adjusted full
/// scale, and can then recover upward at a rate that doubles over
/// `double_time_seconds`. The limiter decision is frame-local across all
/// channels: one loud channel reduces the multiplier used for every channel in
/// that frame.
///
/// Processing is deterministic and stateful within a buffer. It is not a SIMD
/// multiply kernel because the multiplier can change per frame and depends on
/// all channels at that frame.
///
/// # Examples
///
/// ```
/// use auralis_core::SampleRate;
/// use auralis_effects::SoftVol;
///
/// let mut samples = [0.25, 0.75, -1.0];
/// SoftVol::new(2.0, 0.0, 0.0)?.process_mono_samples(&mut samples, SampleRate::new(48_000)?);
///
/// assert_eq!(samples, [0.5, 1.0, -1.0]);
/// # Ok::<(), Box<dyn std::error::Error>>(())
/// ```
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SoftVol {
    /// Initial volume multiplier.
    pub volume: f32,

    /// Seconds required for automatic recovery to double the current volume.
    ///
    /// A value of `0` disables automatic recovery.
    pub double_time_seconds: f32,

    /// Headroom in dB below full scale.
    pub headroom_db: f32,

    max_amplitude: f32,
}

impl SoftVol {
    /// Creates a `softvol` processor.
    ///
    /// # Errors
    ///
    /// Returns [`EffectError::InvalidSoftVol`] when any value is not finite or
    /// is negative.
    pub fn new(volume: f32, double_time_seconds: f32, headroom_db: f32) -> Result<Self> {
        if !volume.is_finite()
            || !double_time_seconds.is_finite()
            || !headroom_db.is_finite()
            || volume < 0.0
            || double_time_seconds < 0.0
            || headroom_db < 0.0
        {
            return Err(EffectError::InvalidSoftVol);
        }

        let max_amplitude = linear_gain(
            auralis_core::Decibels::new(-f64::from(headroom_db))
                .map_err(|_| EffectError::InvalidSoftVol)?,
        );
        Ok(Self {
            volume,
            double_time_seconds,
            headroom_db,
            max_amplitude,
        })
    }

    /// Creates the SoX-ng default `softvol` processor.
    #[must_use]
    pub const fn default_settings() -> Self {
        Self {
            volume: DEFAULT_VOLUME,
            double_time_seconds: DEFAULT_DOUBLE_TIME_SECONDS,
            headroom_db: DEFAULT_HEADROOM_DB,
            max_amplitude: 1.0,
        }
    }

    /// Returns the maximum output amplitude after headroom is reserved.
    #[must_use]
    pub const fn max_amplitude(self) -> f32 {
        self.max_amplitude
    }

    /// Applies soft volume control to all samples in an audio buffer.
    pub fn process_buffer(self, audio: &mut AudioBuffer) {
        let sample_rate = audio.spec().sample_rate().as_u32();
        let mut state = SoftVolState::new(self, sample_rate);
        state.process_buffer(audio);
    }

    /// Applies soft volume control to a mono sample slice.
    ///
    /// This helper is intended for simple direct API use and tests. Multi-
    /// channel processing should use [`Self::process_buffer`] so frame-local
    /// cross-channel peak checks match SoX-ng.
    pub fn process_mono_samples(self, samples: &mut [f32], sample_rate: SampleRate) {
        let mut state = SoftVolState::new(self, sample_rate.as_u32());
        state.process_mono_samples(samples);
    }
}

impl Default for SoftVol {
    fn default() -> Self {
        Self::default_settings()
    }
}

#[derive(Debug, Clone, Copy)]
pub(crate) struct SoftVolState {
    volume: f32,
    max_amplitude: f32,
    multiplier_per_frame: Option<f32>,
}

impl SoftVolState {
    pub(crate) fn new(config: SoftVol, sample_rate: u32) -> Self {
        let multiplier_per_frame =
            (config.double_time_seconds > 0.0).then(|| recovery_multiplier(config, sample_rate));
        Self {
            volume: config.volume,
            max_amplitude: config.max_amplitude,
            multiplier_per_frame,
        }
    }

    pub(crate) fn process_buffer(&mut self, audio: &mut AudioBuffer) {
        let frames = usize::try_from(audio.frames().as_u64())
            .expect("AudioBuffer construction already validated frame count");
        let channels = audio.channels().as_usize();
        let samples = audio.as_planar_f32_mut();

        match channels {
            1 => self.process_mono_samples(&mut samples[..frames]),
            2 => {
                let (left, right) = samples.split_at_mut(frames);
                self.process_stereo_samples(left, &mut right[..frames]);
            }
            _ => {
                for frame in 0..frames {
                    self.process_planar_frame(samples, channels, frames, frame);
                }
            }
        }
    }

    fn process_mono_samples(&mut self, samples: &mut [f32]) {
        let mut volume = self.volume;

        if let Some(multiplier) = self.multiplier_per_frame {
            for sample in samples {
                let max_sample = sample.abs();
                if max_sample > 0.0 && max_sample * volume > self.max_amplitude {
                    volume = self.max_amplitude / max_sample;
                }
                *sample *= volume;
                volume *= multiplier;
            }
        } else {
            for sample in samples {
                let max_sample = sample.abs();
                if max_sample > 0.0 && max_sample * volume > self.max_amplitude {
                    volume = self.max_amplitude / max_sample;
                }
                *sample *= volume;
            }
        }

        self.volume = volume;
    }

    fn process_stereo_samples(&mut self, left: &mut [f32], right: &mut [f32]) {
        let mut volume = self.volume;

        if let Some(multiplier) = self.multiplier_per_frame {
            for (left_sample, right_sample) in left.iter_mut().zip(right.iter_mut()) {
                let max_sample = left_sample.abs().max(right_sample.abs());
                if max_sample > 0.0 && max_sample * volume > self.max_amplitude {
                    volume = self.max_amplitude / max_sample;
                }
                *left_sample *= volume;
                *right_sample *= volume;
                volume *= multiplier;
            }
        } else {
            for (left_sample, right_sample) in left.iter_mut().zip(right.iter_mut()) {
                let max_sample = left_sample.abs().max(right_sample.abs());
                if max_sample > 0.0 && max_sample * volume > self.max_amplitude {
                    volume = self.max_amplitude / max_sample;
                }
                *left_sample *= volume;
                *right_sample *= volume;
            }
        }

        self.volume = volume;
    }

    fn process_planar_frame(
        &mut self,
        samples: &mut [f32],
        channels: usize,
        frames: usize,
        frame: usize,
    ) {
        let mut max_sample = 0.0_f32;
        for channel in 0..channels {
            max_sample = max_sample.max(samples[channel * frames + frame].abs());
        }

        self.update_volume_for_peak(max_sample);

        for channel in 0..channels {
            samples[channel * frames + frame] *= self.volume;
        }

        self.recover_after_frame();
    }

    fn update_volume_for_peak(&mut self, max_sample: f32) {
        if max_sample > 0.0 && max_sample * self.volume > self.max_amplitude {
            self.volume = self.max_amplitude / max_sample;
        }
    }

    fn recover_after_frame(&mut self) {
        if let Some(multiplier) = self.multiplier_per_frame {
            self.volume *= multiplier;
        }
    }
}

#[allow(
    clippy::cast_precision_loss,
    reason = "sample rate is intentionally converted to the floating-point domain for SoX-ng's per-frame recovery formula"
)]
fn recovery_multiplier(config: SoftVol, sample_rate: u32) -> f32 {
    2.0_f32.powf(1.0 / (config.double_time_seconds * sample_rate as f32))
}

#[cfg(test)]
mod tests {
    use super::{SoftVol, SoftVolState};
    use crate::EffectError;
    use auralis_core::{
        AudioBuffer, AudioSpec, ChannelCount, FrameCount, SampleFormat, SampleRate,
    };

    #[test]
    fn loud_frames_reduce_volume_before_output() {
        let mut samples = [0.25, 0.75, -1.0];

        SoftVol::new(2.0, 0.0, 0.0)
            .unwrap()
            .process_mono_samples(&mut samples, SampleRate::new(48_000).unwrap());

        assert_samples_close(&samples, &[0.5, 1.0, -1.0]);
    }

    #[test]
    fn headroom_limits_peak_below_full_scale() {
        let softvol = SoftVol::new(2.0, 0.0, 6.0).unwrap();
        let mut samples = [1.0];

        softvol.process_mono_samples(&mut samples, SampleRate::new(48_000).unwrap());

        assert!((softvol.max_amplitude() - 0.501_187_2).abs() <= 0.000_001);
        assert_samples_close(&samples, &[softvol.max_amplitude()]);
    }

    #[test]
    fn recovery_raises_volume_after_each_frame() {
        let mut samples = [1.0, 0.25, 0.25];

        SoftVol::new(2.0, 1.0, 0.0)
            .unwrap()
            .process_mono_samples(&mut samples, SampleRate::new(1).unwrap());

        assert_samples_close(&samples, &[1.0, 0.5, 1.0]);
    }

    #[test]
    fn stereo_frame_uses_loudest_channel_for_both_channels() {
        let mut audio = stereo_audio_buffer(vec![0.25, 0.25, 1.0, 0.25]);

        SoftVol::new(2.0, 0.0, 0.0)
            .unwrap()
            .process_buffer(&mut audio);

        assert_samples_close(audio.as_planar_f32(), &[0.25, 0.25, 1.0, 0.25]);
    }

    #[test]
    fn stateful_chunked_processing_matches_whole_buffer() {
        let config = SoftVol::new(2.0, 2.0, 0.1).unwrap();
        let source = stereo_audio_buffer(vec![1.0, 0.2, 0.8, 0.2, 0.4, 0.9, 0.3, 0.2]);
        let mut whole = source.clone();
        let mut chunked = source;

        config.process_buffer(&mut whole);

        let mut state = SoftVolState::new(config, chunked.spec().sample_rate().as_u32());
        let mut frame = 0;
        for frame_count in [1_usize, 2, 1] {
            for _ in 0..frame_count {
                state.process_planar_frame(chunked.as_planar_f32_mut(), 2, 4, frame);
                frame += 1;
            }
        }

        assert_samples_close(chunked.as_planar_f32(), whole.as_planar_f32());
    }

    #[test]
    fn invalid_values_are_rejected() {
        assert_eq!(
            SoftVol::new(-0.1, 0.0, 0.0).unwrap_err(),
            EffectError::InvalidSoftVol
        );
        assert_eq!(
            SoftVol::new(1.0, f32::NAN, 0.0).unwrap_err(),
            EffectError::InvalidSoftVol
        );
        assert_eq!(
            SoftVol::new(1.0, 0.0, -0.1).unwrap_err(),
            EffectError::InvalidSoftVol
        );
    }

    fn stereo_audio_buffer(samples: Vec<f32>) -> AudioBuffer {
        assert_eq!(samples.len() % 2, 0);
        let spec = AudioSpec::new(
            SampleRate::new(1).unwrap(),
            ChannelCount::new(2).unwrap(),
            SampleFormat::Float32,
        );
        let frames = u64::try_from(samples.len() / 2).unwrap();
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
}

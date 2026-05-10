use auralis_core::{AudioBuffer, FrameCount};

use crate::{EffectError, Result};

const DEFAULT_GAIN_IN: f64 = 0.5;
const DEFAULT_GAIN_OUT: f64 = 1.0;
const DEFAULT_DELAY_MS: f64 = 50.0;
const DEFAULT_DECAY: f64 = 0.5;
const DEFAULT_SPEED_HZ: f64 = 0.25;
const DEFAULT_DEPTH_MS: f64 = 2.0;

/// One scalar sinusoidal chorus delay stage.
///
/// This is the core data model for the SoX-ng-style chorus family. It covers a
/// single sine-modulated delay line; command options, interpolation modes, and
/// multi-stage command parsing are reserved for the follow-up chorus feature.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ChorusStage {
    delay_ms: f64,
    decay: f64,
    speed_hz: f64,
    depth_ms: f64,
}

impl ChorusStage {
    /// Creates a chorus stage from delay, decay, modulation speed, and depth.
    ///
    /// # Errors
    ///
    /// Returns [`EffectError::InvalidChorus`] when `decay` is outside
    /// `-1..=1`, when delay/depth/speed are negative, or when any value is
    /// non-finite.
    pub fn new(delay_ms: f64, decay: f64, speed_hz: f64, depth_ms: f64) -> Result<Self> {
        if !delay_ms.is_finite()
            || delay_ms < 0.0
            || !decay.is_finite()
            || !(-1.0..=1.0).contains(&decay)
            || !speed_hz.is_finite()
            || speed_hz < 0.0
            || !depth_ms.is_finite()
            || depth_ms < 0.0
        {
            return Err(EffectError::InvalidChorus);
        }

        Ok(Self {
            delay_ms,
            decay,
            speed_hz,
            depth_ms,
        })
    }

    /// Creates a stage using SoX-ng's scalar chorus defaults.
    #[must_use]
    pub const fn default_stage() -> Self {
        Self {
            delay_ms: DEFAULT_DELAY_MS,
            decay: DEFAULT_DECAY,
            speed_hz: DEFAULT_SPEED_HZ,
            depth_ms: DEFAULT_DEPTH_MS,
        }
    }

    /// Returns the fixed delay component in milliseconds.
    #[must_use]
    pub const fn delay_ms(self) -> f64 {
        self.delay_ms
    }

    /// Returns the delayed signal multiplier.
    #[must_use]
    pub const fn decay(self) -> f64 {
        self.decay
    }

    /// Returns the sinusoidal modulation speed in hertz.
    #[must_use]
    pub const fn speed_hz(self) -> f64 {
        self.speed_hz
    }

    /// Returns the additional modulated delay depth in milliseconds.
    #[must_use]
    pub const fn depth_ms(self) -> f64 {
        self.depth_ms
    }

    fn resolve(self, sample_rate_hz: u32) -> Result<ResolvedChorusStage> {
        let sample_rate = f64::from(sample_rate_hz);
        if self.speed_hz > sample_rate {
            return Err(EffectError::InvalidChorus);
        }

        let base_delay = frames_from_ms(self.delay_ms, sample_rate)?;
        let depth = frames_from_ms(self.depth_ms, sample_rate)?;
        let max_delay = (base_delay + depth).ceil();
        if max_delay < 1.0 {
            return Err(EffectError::InvalidChorus);
        }

        #[allow(
            clippy::cast_possible_truncation,
            clippy::cast_sign_loss,
            reason = "resolved chorus delay is finite, positive, and range-checked"
        )]
        Ok(ResolvedChorusStage {
            base_delay,
            depth,
            max_delay: max_delay as usize,
            decay: self.decay,
            speed_hz: self.speed_hz,
            sample_rate,
        })
    }
}

impl Default for ChorusStage {
    fn default() -> Self {
        Self::default_stage()
    }
}

/// Single-stage sinusoidal chorus processor.
///
/// `Chorus` applies a clean input gain, adds one sine-modulated delayed copy
/// of the input scaled by the stage decay, applies a final output gain, clips
/// to normalized full scale, and extends the output by the maximum resolved
/// delay. The current core is scalar and whole-buffer oriented; chunked
/// streaming would be exact only when callers preserve the per-channel delay
/// line, modulation phase, and explicit tail flush state.
///
/// # Examples
///
/// ```
/// use auralis_core::{
///     AudioBuffer, AudioSpec, ChannelCount, FrameCount, SampleFormat, SampleRate,
/// };
/// use auralis_effects::{Chorus, ChorusStage};
///
/// let spec = AudioSpec::new(
///     SampleRate::new(1_000)?,
///     ChannelCount::new(1)?,
///     SampleFormat::Float32,
/// );
/// let audio = AudioBuffer::from_planar_f32(spec, FrameCount::new(2), vec![1.0, 0.0])?;
///
/// let chorus = Chorus::new(0.5, 1.0, ChorusStage::new(1.0, 0.25, 0.0, 0.0)?)?;
/// let processed = chorus.process_buffer(&audio)?;
///
/// assert_eq!(processed.as_planar_f32(), &[0.5, 0.25, 0.0]);
/// # Ok::<(), Box<dyn std::error::Error>>(())
/// ```
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Chorus {
    gain_in: f64,
    gain_out: f64,
    stage: ChorusStage,
}

impl Chorus {
    /// Creates a single-stage chorus processor.
    ///
    /// # Errors
    ///
    /// Returns [`EffectError::InvalidChorus`] when either gain is outside
    /// `-1..=1` or non-finite.
    pub fn new(gain_in: f64, gain_out: f64, stage: ChorusStage) -> Result<Self> {
        if !gain_in.is_finite()
            || !(-1.0..=1.0).contains(&gain_in)
            || !gain_out.is_finite()
            || !(-1.0..=1.0).contains(&gain_out)
        {
            return Err(EffectError::InvalidChorus);
        }

        Ok(Self {
            gain_in,
            gain_out,
            stage,
        })
    }

    /// Creates a chorus processor using SoX-ng's scalar defaults.
    ///
    /// # Errors
    ///
    /// This constructor currently cannot fail because all defaults are valid.
    pub fn with_defaults() -> Result<Self> {
        Self::new(
            DEFAULT_GAIN_IN,
            DEFAULT_GAIN_OUT,
            ChorusStage::default_stage(),
        )
    }

    /// Returns the clean input multiplier.
    #[must_use]
    pub const fn gain_in(self) -> f64 {
        self.gain_in
    }

    /// Returns the final output multiplier.
    #[must_use]
    pub const fn gain_out(self) -> f64 {
        self.gain_out
    }

    /// Returns the configured chorus stage.
    #[must_use]
    pub const fn stage(self) -> ChorusStage {
        self.stage
    }

    /// Applies chorus and returns an output buffer extended by the delay tail.
    ///
    /// # Errors
    ///
    /// Returns [`EffectError::InvalidChorus`] when the stage cannot be resolved
    /// at the input sample rate, or [`EffectError::ChorusLengthOverflow`] when
    /// output allocation would overflow.
    pub fn process_buffer(self, audio: &AudioBuffer) -> Result<AudioBuffer> {
        let resolved = self.stage.resolve(audio.spec().sample_rate().as_u32())?;
        let input_frames = usize::try_from(audio.frames().as_u64())
            .map_err(|_| EffectError::ChorusLengthOverflow)?;
        let output_frames = input_frames
            .checked_add(resolved.max_delay)
            .ok_or(EffectError::ChorusLengthOverflow)?;
        let capacity = audio
            .channels()
            .as_usize()
            .checked_mul(output_frames)
            .ok_or(EffectError::ChorusLengthOverflow)?;
        let mut output = Vec::with_capacity(capacity);

        for channel_index in 0..audio.channels().as_usize() {
            let channel = audio
                .channel(channel_index)
                .ok_or(EffectError::ChorusLengthOverflow)?;
            process_channel(
                channel,
                output_frames,
                self.gain_in,
                self.gain_out,
                resolved,
                &mut output,
            );
        }

        Ok(AudioBuffer::from_planar_f32(
            audio.spec(),
            FrameCount::new(
                u64::try_from(output_frames).map_err(|_| EffectError::ChorusLengthOverflow)?,
            ),
            output,
        )?)
    }
}

#[derive(Debug, Clone, Copy)]
struct ResolvedChorusStage {
    base_delay: f64,
    depth: f64,
    max_delay: usize,
    decay: f64,
    speed_hz: f64,
    sample_rate: f64,
}

fn process_channel(
    input: &[f32],
    output_frames: usize,
    gain_in: f64,
    gain_out: f64,
    stage: ResolvedChorusStage,
    output: &mut Vec<f32>,
) {
    let mut delay_line = vec![0.0_f64; stage.max_delay];
    let mut cursor = 0_usize;

    for frame in 0..output_frames {
        let input_sample = input.get(frame).copied().map_or(0.0, f64::from);
        let delay = stage.delay_at_frame(frame);
        let delayed = if delay == 0 {
            input_sample
        } else {
            let delayed_index = (cursor + stage.max_delay - delay) % stage.max_delay;
            delay_line[delayed_index]
        };
        delay_line[cursor] = input_sample;
        cursor = (cursor + 1) % stage.max_delay;

        let output_sample = (input_sample * gain_in + delayed * stage.decay) * gain_out;
        output.push(f64_to_f32_clamped(output_sample));
    }
}

impl ResolvedChorusStage {
    fn delay_at_frame(self, frame: usize) -> usize {
        let delay = self.base_delay + self.depth * self.modulation_at_frame(frame);
        #[allow(
            clippy::cast_precision_loss,
            reason = "max_delay was range-checked before allocation and is only used as an f64 clamp bound"
        )]
        let bounded = delay.round().clamp(0.0, self.max_delay as f64);

        #[allow(
            clippy::cast_possible_truncation,
            clippy::cast_sign_loss,
            reason = "delay is clamped into the allocated delay-line range"
        )]
        {
            bounded as usize
        }
    }

    fn modulation_at_frame(self, frame: usize) -> f64 {
        if self.depth == 0.0 || self.speed_hz == 0.0 {
            return 0.0;
        }

        #[allow(
            clippy::cast_precision_loss,
            reason = "chorus modulation phase is evaluated in f64 from frame indices"
        )]
        let phase = std::f64::consts::TAU * self.speed_hz * frame as f64 / self.sample_rate
            - std::f64::consts::FRAC_PI_2;
        0.5 + 0.5 * phase.sin()
    }
}

#[allow(
    clippy::cast_precision_loss,
    reason = "this is a conservative preflight bound before converting frame counts to usize"
)]
fn frames_from_ms(milliseconds: f64, sample_rate: f64) -> Result<f64> {
    let frames = milliseconds * sample_rate / 1000.0;
    if frames.is_finite() && frames <= usize::MAX as f64 {
        Ok(frames)
    } else {
        Err(EffectError::ChorusLengthOverflow)
    }
}

fn f64_to_f32_clamped(sample: f64) -> f32 {
    #[allow(
        clippy::cast_possible_truncation,
        reason = "chorus output is explicitly clipped to normalized f32 full scale"
    )]
    {
        sample.clamp(-1.0, 1.0) as f32
    }
}

#[cfg(test)]
mod tests {
    use super::{Chorus, ChorusStage};
    use crate::EffectError;
    use auralis_core::{
        AudioBuffer, AudioSpec, ChannelCount, FrameCount, SampleFormat, SampleRate,
    };

    #[test]
    fn depth_zero_core_behaves_like_one_parallel_delay() {
        let audio = mono_audio_buffer(1_000, &[1.0, 0.0]);
        let chorus = Chorus::new(0.5, 1.0, ChorusStage::new(1.0, 0.25, 0.0, 0.0).unwrap()).unwrap();

        let processed = chorus.process_buffer(&audio).unwrap();

        assert_eq!(processed.frames(), FrameCount::new(3));
        assert_eq!(processed.as_planar_f32(), &[0.5, 0.25, 0.0]);
    }

    #[test]
    fn sine_modulation_extends_by_maximum_delay() {
        let audio = mono_audio_buffer(1_000, &[1.0, 0.0, 0.5]);
        let chorus = Chorus::new(1.0, 1.0, ChorusStage::new(1.0, 0.5, 1.0, 2.0).unwrap()).unwrap();

        let processed = chorus.process_buffer(&audio).unwrap();

        assert_eq!(processed.frames(), FrameCount::new(6));
        assert!(
            processed
                .as_planar_f32()
                .iter()
                .all(|sample| sample.is_finite())
        );
    }

    #[test]
    fn clips_inside_effect() {
        let audio = mono_audio_buffer(1_000, &[1.0]);
        let chorus = Chorus::new(1.0, 1.0, ChorusStage::new(0.0, 1.0, 0.0, 1.0).unwrap()).unwrap();

        let processed = chorus.process_buffer(&audio).unwrap();

        assert_eq!(processed.as_planar_f32(), &[1.0, 0.0]);
    }

    #[test]
    fn rejects_invalid_configuration() {
        assert_eq!(
            ChorusStage::new(-1.0, 0.5, 0.25, 2.0).unwrap_err(),
            EffectError::InvalidChorus
        );
        assert_eq!(
            ChorusStage::new(50.0, 1.5, 0.25, 2.0).unwrap_err(),
            EffectError::InvalidChorus
        );
        assert_eq!(
            Chorus::new(f64::NAN, 1.0, ChorusStage::default_stage()).unwrap_err(),
            EffectError::InvalidChorus
        );
    }

    #[test]
    fn rejects_zero_resolved_delay_at_processing_time() {
        let audio = mono_audio_buffer(1_000, &[1.0]);
        let chorus = Chorus::new(0.5, 1.0, ChorusStage::new(0.0, 0.5, 0.25, 0.0).unwrap()).unwrap();

        assert_eq!(
            chorus.process_buffer(&audio).unwrap_err(),
            EffectError::InvalidChorus
        );
    }

    fn mono_audio_buffer(sample_rate_hz: u32, samples: &[f32]) -> AudioBuffer {
        let spec = AudioSpec::new(
            SampleRate::new(sample_rate_hz).unwrap(),
            ChannelCount::new(1).unwrap(),
            SampleFormat::Float32,
        );
        AudioBuffer::from_planar_f32(
            spec,
            FrameCount::new(u64::try_from(samples.len()).unwrap()),
            samples.to_vec(),
        )
        .unwrap()
    }
}

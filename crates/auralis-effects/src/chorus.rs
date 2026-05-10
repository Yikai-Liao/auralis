use auralis_core::{AudioBuffer, FrameCount};

use crate::{EffectError, Result};

const DEFAULT_GAIN_IN: f64 = 0.5;
const DEFAULT_GAIN_OUT: f64 = 1.0;
const DEFAULT_DELAY_MS: f64 = 50.0;
const DEFAULT_DECAY: f64 = 0.5;
const DEFAULT_SPEED_HZ: f64 = 0.25;
const DEFAULT_DEPTH_MS: f64 = 2.0;

/// SoX-ng chorus delay-line interpolation mode.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ChorusInterpolation {
    /// Use the nearest integer modulated delay offset.
    #[default]
    None,
    /// Linearly interpolate between adjacent delay-line samples.
    Linear,
    /// Use SoX-ng's quadratic interpolation across three delay-line samples.
    Quadratic,
}

/// SoX-ng chorus modulation waveform.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ChorusWave {
    /// Sine modulation.
    #[default]
    Sine,
    /// Triangle modulation.
    Triangle,
}

/// One scalar SoX-ng-style chorus delay stage.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ChorusStage {
    delay_ms: f64,
    decay: f64,
    speed_hz: f64,
    depth_ms: f64,
    wave: ChorusWave,
}

impl ChorusStage {
    /// Creates a sine-modulated chorus stage.
    ///
    /// # Errors
    ///
    /// Returns [`EffectError::InvalidChorus`] when `decay` is outside
    /// `-1..=1`, delay/depth are outside SoX-ng's millisecond range, speed is
    /// outside `0..=192000`, or any value is non-finite. A zero speed is
    /// accepted at construction but rejected at processing start to mirror
    /// SoX-ng's runtime validation.
    pub fn new(delay_ms: f64, decay: f64, speed_hz: f64, depth_ms: f64) -> Result<Self> {
        Self::with_wave(delay_ms, decay, speed_hz, depth_ms, ChorusWave::Sine)
    }

    /// Creates a chorus stage with an explicit modulation waveform.
    ///
    /// # Errors
    ///
    /// Returns [`EffectError::InvalidChorus`] when any numeric parameter is
    /// outside SoX-ng's accepted range or non-finite.
    pub fn with_wave(
        delay_ms: f64,
        decay: f64,
        speed_hz: f64,
        depth_ms: f64,
        wave: ChorusWave,
    ) -> Result<Self> {
        if !delay_ms.is_finite()
            || !(0.0..=86_400_000.0).contains(&delay_ms)
            || !decay.is_finite()
            || !(-1.0..=1.0).contains(&decay)
            || !speed_hz.is_finite()
            || !(0.0..=192_000.0).contains(&speed_hz)
            || !depth_ms.is_finite()
            || !(0.0..=86_400_000.0).contains(&depth_ms)
        {
            return Err(EffectError::InvalidChorus);
        }

        Ok(Self {
            delay_ms,
            decay,
            speed_hz,
            depth_ms,
            wave,
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
            wave: ChorusWave::Sine,
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

    /// Returns the modulation speed in hertz.
    #[must_use]
    pub const fn speed_hz(self) -> f64 {
        self.speed_hz
    }

    /// Returns the additional modulated delay depth in milliseconds.
    #[must_use]
    pub const fn depth_ms(self) -> f64 {
        self.depth_ms
    }

    /// Returns the stage modulation waveform.
    #[must_use]
    pub const fn wave(self) -> ChorusWave {
        self.wave
    }

    fn resolve(
        self,
        sample_rate_hz: u32,
        interpolation: ChorusInterpolation,
    ) -> Result<ResolvedChorusStage> {
        let sample_rate = f64::from(sample_rate_hz);
        if self.speed_hz == 0.0 || self.speed_hz > sample_rate {
            return Err(EffectError::InvalidChorus);
        }

        let base_delay = frames_from_ms(self.delay_ms, sample_rate)?;
        let depth = frames_from_ms(self.depth_ms, sample_rate)?;
        let raw_delay_line_length = (base_delay + depth).ceil();
        if raw_delay_line_length < 1.0 {
            return Err(EffectError::InvalidChorus);
        }
        let interpolation_padding = match interpolation {
            ChorusInterpolation::None => 0,
            ChorusInterpolation::Linear => 1,
            ChorusInterpolation::Quadratic => 2,
        };
        let delay_line_length = raw_delay_line_length + f64::from(interpolation_padding);

        #[allow(
            clippy::cast_possible_truncation,
            clippy::cast_sign_loss,
            reason = "resolved chorus delay is finite, positive, and range-checked"
        )]
        Ok(ResolvedChorusStage {
            base_delay,
            depth,
            delay_line_length: delay_line_length as usize,
            decay: self.decay,
            speed_hz: self.speed_hz,
            sample_rate,
            wave: self.wave,
            interpolation,
        })
    }
}

impl Default for ChorusStage {
    fn default() -> Self {
        Self::default_stage()
    }
}

/// SoX-ng-style chorus processor.
///
/// `Chorus` applies a clean input gain, adds one or more modulated delayed
/// copies of the input scaled by each stage decay, applies a final output gain,
/// clips to normalized full scale, and extends the output by the largest
/// resolved delay line.
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
/// let chorus = Chorus::new(0.5, 1.0, ChorusStage::new(1.0, 0.25, 1.0, 0.0)?)?;
/// let processed = chorus.process_buffer(&audio)?;
///
/// assert_eq!(processed.as_planar_f32(), &[0.5, 0.25, 0.0, 0.0]);
/// # Ok::<(), Box<dyn std::error::Error>>(())
/// ```
#[derive(Debug, Clone, PartialEq)]
pub struct Chorus {
    gain_in: f64,
    gain_out: f64,
    stages: Vec<ChorusStage>,
    interpolation: ChorusInterpolation,
}

impl Chorus {
    /// Creates a single-stage chorus processor with no interpolation.
    ///
    /// # Errors
    ///
    /// Returns [`EffectError::InvalidChorus`] when either gain is outside
    /// `-1..=1` or non-finite.
    pub fn new(gain_in: f64, gain_out: f64, stage: ChorusStage) -> Result<Self> {
        Self::with_options(gain_in, gain_out, [stage], ChorusInterpolation::None)
    }

    /// Creates a chorus processor with explicit interpolation and stages.
    ///
    /// # Errors
    ///
    /// Returns [`EffectError::InvalidChorus`] when either gain is outside
    /// `-1..=1`, non-finite, or when no stages are supplied.
    pub fn with_options<I>(
        gain_in: f64,
        gain_out: f64,
        stages: I,
        interpolation: ChorusInterpolation,
    ) -> Result<Self>
    where
        I: IntoIterator<Item = ChorusStage>,
    {
        let stages = stages.into_iter().collect::<Vec<_>>();
        if !gain_in.is_finite()
            || !(-1.0..=1.0).contains(&gain_in)
            || !gain_out.is_finite()
            || !(-1.0..=1.0).contains(&gain_out)
            || stages.is_empty()
        {
            return Err(EffectError::InvalidChorus);
        }

        Ok(Self {
            gain_in,
            gain_out,
            stages,
            interpolation,
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
    pub const fn gain_in(&self) -> f64 {
        self.gain_in
    }

    /// Returns the final output multiplier.
    #[must_use]
    pub const fn gain_out(&self) -> f64 {
        self.gain_out
    }

    /// Returns the first configured chorus stage.
    #[must_use]
    pub fn stage(&self) -> ChorusStage {
        self.stages[0]
    }

    /// Returns all configured chorus stages.
    #[must_use]
    pub fn stages(&self) -> &[ChorusStage] {
        &self.stages
    }

    /// Returns the configured interpolation mode.
    #[must_use]
    pub const fn interpolation(&self) -> ChorusInterpolation {
        self.interpolation
    }

    /// Applies chorus and returns an output buffer extended by the delay tail.
    ///
    /// # Errors
    ///
    /// Returns [`EffectError::InvalidChorus`] when a stage cannot be resolved
    /// at the input sample rate, or [`EffectError::ChorusLengthOverflow`] when
    /// output allocation would overflow.
    pub fn process_buffer(&self, audio: &AudioBuffer) -> Result<AudioBuffer> {
        let resolved = self
            .stages
            .iter()
            .map(|stage| stage.resolve(audio.spec().sample_rate().as_u32(), self.interpolation))
            .collect::<Result<Vec<_>>>()?;
        let max_delay = resolved
            .iter()
            .map(|stage| stage.delay_line_length)
            .max()
            .ok_or(EffectError::InvalidChorus)?;
        let input_frames = usize::try_from(audio.frames().as_u64())
            .map_err(|_| EffectError::ChorusLengthOverflow)?;
        let output_frames = input_frames
            .checked_add(max_delay)
            .and_then(|frames| frames.checked_add(1))
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
                &resolved,
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
    delay_line_length: usize,
    decay: f64,
    speed_hz: f64,
    sample_rate: f64,
    wave: ChorusWave,
    interpolation: ChorusInterpolation,
}

struct ChorusStageState {
    resolved: ResolvedChorusStage,
    delay_line: Vec<f64>,
    delay_line_index: usize,
    wave_index: usize,
    wave_length: usize,
}

fn process_channel(
    input: &[f32],
    output_frames: usize,
    gain_in: f64,
    gain_out: f64,
    stages: &[ResolvedChorusStage],
    output: &mut Vec<f32>,
) {
    let mut stage_states = stages
        .iter()
        .copied()
        .map(ChorusStageState::new)
        .collect::<Vec<_>>();

    for frame in 0..output_frames {
        let input_sample = input.get(frame).copied().map_or(0.0, f64::from);
        let mut output_sample = input_sample * gain_in;

        for stage in &mut stage_states {
            output_sample += stage.process(input_sample) * stage.resolved.decay;
        }

        output_sample *= gain_out;
        output.push(f64_to_f32_clamped(output_sample));
    }
}

impl ChorusStageState {
    fn new(resolved: ResolvedChorusStage) -> Self {
        let wave_length = resolved.wave_length();
        Self {
            resolved,
            delay_line: vec![0.0; resolved.delay_line_length],
            delay_line_index: 0,
            wave_index: 0,
            wave_length,
        }
    }

    fn process(&mut self, input_sample: f64) -> f64 {
        let offset = self
            .resolved
            .delay_offset(self.wave_index, self.wave_length);
        let sample = match self.resolved.interpolation {
            ChorusInterpolation::None => self.process_none(input_sample, offset),
            ChorusInterpolation::Linear => self.process_linear(input_sample, offset),
            ChorusInterpolation::Quadratic => self.process_quadratic(input_sample, offset),
        };

        self.delay_line_index = (self.delay_line_index + 1) % self.resolved.delay_line_length;
        self.wave_index = (self.wave_index + 1) % self.wave_length;
        sample
    }

    fn process_none(&mut self, input_sample: f64, offset: f64) -> f64 {
        let offset = integer_wave_offset(offset);
        let delay_index = self.delay_index(offset);
        let sample = self.delay_line[delay_index];
        self.delay_line[self.delay_line_index] = input_sample;
        sample
    }

    fn process_linear(&mut self, input_sample: f64, offset: f64) -> f64 {
        let offset_i = offset.trunc();
        let frac = offset - offset_i;
        let delay_index = self.delay_index(float_offset_to_usize(offset_i));
        let delayed_0 = self.delay_line[delay_index];
        let delayed_1 = self.delay_line
            [(delay_index + self.resolved.delay_line_length - 1) % self.resolved.delay_line_length];
        self.delay_line[self.delay_line_index] = input_sample;
        delayed_0.mul_add(1.0 - frac, delayed_1 * frac)
    }

    fn process_quadratic(&mut self, input_sample: f64, offset: f64) -> f64 {
        let offset_i = offset.trunc();
        let frac = offset - offset_i;
        let delay_index = self.delay_index(float_offset_to_usize(offset_i));
        let delayed_0 = self.delay_line[delay_index];
        let delayed_1 = self.delay_line
            [(delay_index + self.resolved.delay_line_length - 1) % self.resolved.delay_line_length];
        let delayed_2 = self.delay_line
            [(delay_index + self.resolved.delay_line_length - 2) % self.resolved.delay_line_length];
        self.delay_line[self.delay_line_index] = input_sample;
        let adjusted_2 = delayed_2 - delayed_0;
        let adjusted_1 = delayed_1 - delayed_0;
        let a = adjusted_2 * 0.5 - adjusted_1;
        let b = adjusted_1 * 2.0 - adjusted_2 * 0.5;
        delayed_0 + (a * frac + b) * frac
    }

    fn delay_index(&self, offset: usize) -> usize {
        (self.delay_line_index + self.resolved.delay_line_length - offset)
            % self.resolved.delay_line_length
    }
}

impl ResolvedChorusStage {
    fn wave_length(self) -> usize {
        if self.speed_hz == 0.0 {
            return 1;
        }

        let length = self.sample_rate / self.speed_hz + 0.5;
        #[allow(
            clippy::cast_possible_truncation,
            clippy::cast_sign_loss,
            reason = "speed was validated against the sample rate, so wave length is finite and nonzero"
        )]
        {
            (length as usize).max(1)
        }
    }

    fn delay_offset(self, wave_index: usize, wave_length: usize) -> f64 {
        self.base_delay + self.depth * self.wave_value(wave_index, wave_length)
    }

    #[allow(
        clippy::cast_precision_loss,
        clippy::cast_possible_truncation,
        reason = "chorus wave-table math mirrors SoX-ng's bounded table index conversions"
    )]
    fn wave_value(self, wave_index: usize, wave_length: usize) -> f64 {
        if self.depth == 0.0 || self.speed_hz == 0.0 {
            return 0.0;
        }

        let phase_offset = (0.75_f64).mul_add(wave_length as f64, 0.5);
        #[allow(
            clippy::cast_possible_truncation,
            clippy::cast_sign_loss,
            reason = "wave length is finite and indexes are bounded by modulo"
        )]
        let point = (wave_index + phase_offset as usize) % wave_length;
        let value = match self.wave {
            ChorusWave::Sine => f64::midpoint(
                (point as f64 / wave_length as f64 * std::f64::consts::TAU).sin(),
                1.0,
            ),
            ChorusWave::Triangle => triangle_wave_value(point, wave_length),
        };

        if matches!(
            self.interpolation,
            ChorusInterpolation::Linear | ChorusInterpolation::Quadratic
        ) {
            f64::from(value as f32)
        } else {
            value
        }
    }
}

#[allow(
    clippy::cast_precision_loss,
    reason = "chorus wave-table math mirrors SoX-ng's bounded table index conversions"
)]
fn triangle_wave_value(point: usize, wave_length: usize) -> f64 {
    let d = point as f64 * 2.0 / wave_length as f64;
    match 4 * point / wave_length {
        0 => d + 0.5,
        1 | 2 => 1.5 - d,
        3 => d - 1.5,
        _ => 0.0,
    }
}

fn integer_wave_offset(offset: f64) -> usize {
    float_offset_to_usize(offset + 0.5)
}

#[allow(
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    reason = "chorus offsets are non-negative and bounded by the allocated delay line"
)]
fn float_offset_to_usize(offset: f64) -> usize {
    offset as usize
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
    use super::{Chorus, ChorusInterpolation, ChorusStage, ChorusWave};
    use crate::EffectError;
    use auralis_core::{
        AudioBuffer, AudioSpec, ChannelCount, FrameCount, SampleFormat, SampleRate,
    };

    #[test]
    fn depth_zero_core_behaves_like_one_parallel_delay() {
        let audio = mono_audio_buffer(1_000, &[1.0, 0.0]);
        let chorus = Chorus::new(0.5, 1.0, ChorusStage::new(1.0, 0.25, 1.0, 0.0).unwrap()).unwrap();

        let processed = chorus.process_buffer(&audio).unwrap();

        assert_eq!(processed.frames(), FrameCount::new(4));
        assert_eq!(processed.as_planar_f32(), &[0.5, 0.25, 0.0, 0.0]);
    }

    #[test]
    fn sine_modulation_extends_by_maximum_delay() {
        let audio = mono_audio_buffer(1_000, &[1.0, 0.0, 0.5]);
        let chorus = Chorus::new(1.0, 1.0, ChorusStage::new(1.0, 0.5, 1.0, 2.0).unwrap()).unwrap();

        let processed = chorus.process_buffer(&audio).unwrap();

        assert_eq!(processed.frames(), FrameCount::new(7));
        assert!(
            processed
                .as_planar_f32()
                .iter()
                .all(|sample| sample.is_finite())
        );
    }

    #[test]
    fn multi_stage_chorus_sums_delayed_stage_outputs() {
        let audio = mono_audio_buffer(1_000, &[1.0, 0.0]);
        let chorus = Chorus::with_options(
            0.5,
            1.0,
            [
                ChorusStage::new(1.0, 0.25, 1.0, 0.0).unwrap(),
                ChorusStage::new(2.0, 0.125, 1.0, 0.0).unwrap(),
            ],
            ChorusInterpolation::None,
        )
        .unwrap();

        let processed = chorus.process_buffer(&audio).unwrap();

        assert_eq!(processed.frames(), FrameCount::new(5));
        assert_eq!(processed.as_planar_f32(), &[0.5, 0.25, 0.125, 0.0, 0.0]);
    }

    #[test]
    fn interpolation_modes_add_sox_ng_delay_line_tail_padding() {
        let audio = mono_audio_buffer(1_000, &[1.0]);
        let stage = ChorusStage::new(1.0, 0.25, 1.0, 0.0).unwrap();

        let linear = Chorus::with_options(0.5, 1.0, [stage], ChorusInterpolation::Linear)
            .unwrap()
            .process_buffer(&audio)
            .unwrap();
        let quadratic = Chorus::with_options(0.5, 1.0, [stage], ChorusInterpolation::Quadratic)
            .unwrap()
            .process_buffer(&audio)
            .unwrap();

        assert_eq!(linear.frames(), FrameCount::new(4));
        assert_eq!(quadratic.frames(), FrameCount::new(5));
    }

    #[test]
    fn clips_inside_effect() {
        let audio = mono_audio_buffer(1_000, &[1.0]);
        let chorus = Chorus::new(1.0, 1.0, ChorusStage::new(0.0, 1.0, 1.0, 1.0).unwrap()).unwrap();

        let processed = chorus.process_buffer(&audio).unwrap();

        assert_eq!(processed.as_planar_f32(), &[1.0, 1.0, 0.0]);
    }

    #[test]
    fn triangle_wave_stage_configuration_is_preserved() {
        let stage = ChorusStage::with_wave(1.0, 0.25, 1.0, 2.0, ChorusWave::Triangle).unwrap();

        assert_eq!(stage.wave(), ChorusWave::Triangle);
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
        assert_eq!(
            Chorus::with_options(0.5, 1.0, [], ChorusInterpolation::None).unwrap_err(),
            EffectError::InvalidChorus
        );
    }

    #[test]
    fn rejects_zero_resolved_delay_at_processing_time() {
        let audio = mono_audio_buffer(1_000, &[1.0]);
        let chorus = Chorus::new(0.5, 1.0, ChorusStage::new(0.0, 0.5, 1.0, 0.0).unwrap()).unwrap();

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

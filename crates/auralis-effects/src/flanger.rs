use auralis_core::AudioBuffer;

use crate::{EffectError, Result};

const DEFAULT_DELAY_MS: f64 = 0.0;
const DEFAULT_DEPTH_MS: f64 = 2.0;
const DEFAULT_REGEN_PERCENT: f64 = 0.0;
const DEFAULT_WIDTH_PERCENT: f64 = 71.0;
const DEFAULT_SPEED_HZ: f64 = 0.5;
const DEFAULT_PHASE_PERCENT: f64 = 25.0;

/// SoX-ng flanger delay-line interpolation mode.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum FlangerInterpolation {
    /// Use the nearest integer swept delay offset.
    None,
    /// Linearly interpolate between adjacent delay-line samples.
    #[default]
    Linear,
    /// Use SoX-ng's quadratic interpolation across three delay-line samples.
    Quadratic,
}

/// SoX-ng flanger modulation waveform.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum FlangerWave {
    /// Sine modulation.
    #[default]
    Sine,
    /// Triangle modulation.
    Triangle,
}

/// SoX-ng-style scalar flanger.
///
/// `Flanger` mixes the input with a swept delay line, applies optional delayed
/// signal feedback, balances the dry/wet path to avoid ordinary mix clipping,
/// and preserves the input length. It does not emit the delayed tail; callers
/// that need the feedback tail should pad before applying this effect, matching
/// SoX-ng's command behavior.
///
/// # Examples
///
/// ```
/// use auralis_core::{
///     AudioBuffer, AudioSpec, ChannelCount, FrameCount, SampleFormat, SampleRate,
/// };
/// use auralis_effects::{Flanger, FlangerInterpolation, FlangerWave};
///
/// let spec = AudioSpec::new(
///     SampleRate::new(1_000)?,
///     ChannelCount::new(1)?,
///     SampleFormat::Float32,
/// );
/// let audio = AudioBuffer::from_planar_f32(spec, FrameCount::new(2), vec![1.0, 0.0])?;
/// let flanger = Flanger::new(
///     0.0,
///     0.0,
///     0.0,
///     100.0,
///     1.0,
///     FlangerWave::Sine,
///     0.0,
///     FlangerInterpolation::Linear,
/// )?;
///
/// let processed = flanger.process_buffer(&audio)?;
/// assert_eq!(processed.as_planar_f32(), &[1.0, 0.0]);
/// # Ok::<(), Box<dyn std::error::Error>>(())
/// ```
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Flanger {
    delay_ms: f64,
    depth_ms: f64,
    regen_percent: f64,
    width_percent: f64,
    speed_hz: f64,
    wave: FlangerWave,
    phase_percent: f64,
    interpolation: FlangerInterpolation,
}

impl Flanger {
    /// Creates a flanger from SoX-ng command parameters.
    ///
    /// # Errors
    ///
    /// Returns [`EffectError::InvalidFlanger`] when delay/depth are outside
    /// `0..=1000` ms, regeneration is outside `-100..=100` percent, width is
    /// negative or NaN, speed is outside `0..=192000` Hz, phase is outside
    /// `0..=100` percent, or any finite-required value is non-finite. A zero
    /// speed is accepted at construction but rejected at processing start to
    /// match SoX-ng's runtime validation style.
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        delay_ms: f64,
        depth_ms: f64,
        regen_percent: f64,
        width_percent: f64,
        speed_hz: f64,
        wave: FlangerWave,
        phase_percent: f64,
        interpolation: FlangerInterpolation,
    ) -> Result<Self> {
        if !delay_ms.is_finite()
            || !(0.0..=1000.0).contains(&delay_ms)
            || !depth_ms.is_finite()
            || !(0.0..=1000.0).contains(&depth_ms)
            || !regen_percent.is_finite()
            || !(-100.0..=100.0).contains(&regen_percent)
            || width_percent.is_nan()
            || width_percent < 0.0
            || !speed_hz.is_finite()
            || !(0.0..=192_000.0).contains(&speed_hz)
            || !phase_percent.is_finite()
            || !(0.0..=100.0).contains(&phase_percent)
        {
            return Err(EffectError::InvalidFlanger);
        }

        Ok(Self {
            delay_ms,
            depth_ms,
            regen_percent,
            width_percent,
            speed_hz,
            wave,
            phase_percent,
            interpolation,
        })
    }

    /// Creates a flanger using SoX-ng's scalar defaults.
    ///
    /// # Errors
    ///
    /// This constructor currently cannot fail because all defaults are valid.
    pub fn with_defaults() -> Result<Self> {
        Self::new(
            DEFAULT_DELAY_MS,
            DEFAULT_DEPTH_MS,
            DEFAULT_REGEN_PERCENT,
            DEFAULT_WIDTH_PERCENT,
            DEFAULT_SPEED_HZ,
            FlangerWave::Sine,
            DEFAULT_PHASE_PERCENT,
            FlangerInterpolation::Linear,
        )
    }

    /// Returns the base delay in milliseconds.
    #[must_use]
    pub const fn delay_ms(self) -> f64 {
        self.delay_ms
    }

    /// Returns the added swept delay depth in milliseconds.
    #[must_use]
    pub const fn depth_ms(self) -> f64 {
        self.depth_ms
    }

    /// Returns delayed-signal feedback in percent.
    #[must_use]
    pub const fn regen_percent(self) -> f64 {
        self.regen_percent
    }

    /// Returns delayed-signal mix width in percent.
    #[must_use]
    pub const fn width_percent(self) -> f64 {
        self.width_percent
    }

    /// Returns the sweep speed in hertz.
    #[must_use]
    pub const fn speed_hz(self) -> f64 {
        self.speed_hz
    }

    /// Returns the modulation waveform.
    #[must_use]
    pub const fn wave(self) -> FlangerWave {
        self.wave
    }

    /// Returns the per-channel LFO phase shift in percent.
    #[must_use]
    pub const fn phase_percent(self) -> f64 {
        self.phase_percent
    }

    /// Returns the delay-line interpolation mode.
    #[must_use]
    pub const fn interpolation(self) -> FlangerInterpolation {
        self.interpolation
    }

    /// Applies flanger and returns a length-preserving output buffer.
    ///
    /// # Errors
    ///
    /// Returns [`EffectError::InvalidFlanger`] when the sweep speed cannot be
    /// resolved at the input sample rate, or [`EffectError::FlangerLengthOverflow`]
    /// when the delay line allocation would exceed the platform limit.
    pub fn process_buffer(&self, audio: &AudioBuffer) -> Result<AudioBuffer> {
        let resolved = self.resolve(audio.spec().sample_rate().as_u32())?;
        let mut output = Vec::with_capacity(audio.as_planar_f32().len());

        for channel_index in 0..audio.channels().as_usize() {
            let channel = audio
                .channel(channel_index)
                .ok_or(EffectError::FlangerLengthOverflow)?;
            process_channel(channel, channel_index, resolved, &mut output);
        }

        Ok(AudioBuffer::from_planar_f32(
            audio.spec(),
            audio.frames(),
            output,
        )?)
    }

    fn resolve(self, sample_rate_hz: u32) -> Result<ResolvedFlanger> {
        let sample_rate = f64::from(sample_rate_hz);
        if self.speed_hz == 0.0 || self.speed_hz > sample_rate {
            return Err(EffectError::InvalidFlanger);
        }

        let delay = frames_from_ms(self.delay_ms, sample_rate)?;
        let depth = frames_from_ms(self.depth_ms, sample_rate)?;
        let delay_line_length = (delay + depth).ceil() + 2.0;
        let lfo_length = sample_rate / self.speed_hz + 0.5;
        if lfo_length < 1.0 {
            return Err(EffectError::InvalidFlanger);
        }

        let (gain_in, width) = balanced_mix(self.width_percent, self.regen_percent);

        #[allow(
            clippy::cast_possible_truncation,
            clippy::cast_sign_loss,
            reason = "resolved flanger delay and LFO lengths are finite and range-checked"
        )]
        Ok(ResolvedFlanger {
            delay,
            depth,
            regen: self.regen_percent / 100.0,
            width,
            gain_in,
            delay_line_length: delay_line_length as usize,
            lfo_length: (lfo_length as usize).max(1),
            wave: self.wave,
            phase: self.phase_percent / 100.0,
            interpolation: self.interpolation,
        })
    }
}

#[derive(Debug, Clone, Copy)]
struct ResolvedFlanger {
    delay: f64,
    depth: f64,
    regen: f64,
    width: f64,
    gain_in: f64,
    delay_line_length: usize,
    lfo_length: usize,
    wave: FlangerWave,
    phase: f64,
    interpolation: FlangerInterpolation,
}

struct FlangerChannelState {
    resolved: ResolvedFlanger,
    delay_line: Vec<f64>,
    delay_line_index: usize,
    delay_last: f64,
}

fn process_channel(
    input: &[f32],
    channel_index: usize,
    resolved: ResolvedFlanger,
    output: &mut Vec<f32>,
) {
    let mut state = FlangerChannelState::new(resolved);
    let delay_offsets = resolved.delay_offsets(channel_index);
    for (frame, sample) in input.iter().copied().enumerate() {
        let delay = delay_offsets[frame % delay_offsets.len()];
        output.push(state.process(f64::from(sample), delay));
    }
}

impl FlangerChannelState {
    fn new(resolved: ResolvedFlanger) -> Self {
        Self {
            resolved,
            delay_line: vec![0.0; resolved.delay_line_length],
            delay_line_index: 0,
            delay_last: 0.0,
        }
    }

    fn process(&mut self, input_sample: f64, delay: f64) -> f32 {
        self.delay_line_index = (self.delay_line_index + self.resolved.delay_line_length - 1)
            % self.resolved.delay_line_length;
        self.delay_line[self.delay_line_index] =
            input_sample + self.delay_last * self.resolved.regen;

        let delayed = match self.resolved.interpolation {
            FlangerInterpolation::None => self.process_none(delay),
            FlangerInterpolation::Linear => self.process_linear(delay),
            FlangerInterpolation::Quadratic => self.process_quadratic(delay),
        };

        self.delay_last = delayed;
        f64_to_f32_clamped(input_sample * self.resolved.gain_in + delayed * self.resolved.width)
    }

    fn process_none(&self, delay: f64) -> f64 {
        let delay_index = self.delay_index(integer_wave_offset(delay));
        self.delay_line[delay_index]
    }

    fn process_linear(&self, delay: f64) -> f64 {
        let delay_i = delay.trunc();
        let frac = delay - delay_i;
        let delay_index = self.delay_index(float_offset_to_usize(delay_i));
        let delayed_0 = self.delay_line[delay_index];
        let delayed_1 = self.delay_line[(delay_index + 1) % self.resolved.delay_line_length];

        delayed_0.mul_add(1.0 - frac, delayed_1 * frac)
    }

    fn process_quadratic(&self, delay: f64) -> f64 {
        let delay_i = delay.trunc();
        let frac = delay - delay_i;
        let delay_index = self.delay_index(float_offset_to_usize(delay_i));
        let delayed_0 = self.delay_line[delay_index];
        let delayed_1 = self.delay_line[(delay_index + 1) % self.resolved.delay_line_length];
        let delayed_2 = self.delay_line[(delay_index + 2) % self.resolved.delay_line_length];
        let adjusted_2 = delayed_2 - delayed_0;
        let adjusted_1 = delayed_1 - delayed_0;
        let a = adjusted_2 * 0.5 - adjusted_1;
        let b = adjusted_1 * 2.0 - adjusted_2 * 0.5;

        delayed_0 + (a * frac + b) * frac
    }

    fn delay_index(&self, offset: usize) -> usize {
        (self.delay_line_index + offset) % self.resolved.delay_line_length
    }
}

impl ResolvedFlanger {
    fn delay_offsets(self, channel_index: usize) -> Vec<f64> {
        (0..self.lfo_length)
            .map(|frame| self.delay + self.depth * self.wave_value(frame, channel_index))
            .collect()
    }

    #[allow(
        clippy::cast_precision_loss,
        clippy::cast_possible_truncation,
        reason = "flanger LFO math mirrors SoX-ng's bounded table index conversions"
    )]
    fn wave_value(self, frame: usize, channel_index: usize) -> f64 {
        if self.depth == 0.0 {
            return 0.0;
        }

        let phase = channel_index as f64 * self.lfo_length as f64 * self.phase + 0.5;
        #[allow(
            clippy::cast_possible_truncation,
            clippy::cast_sign_loss,
            reason = "LFO length is finite and indexes are bounded by modulo"
        )]
        let point = (frame + phase as usize) % self.lfo_length;
        let value = match self.wave {
            FlangerWave::Sine => f64::midpoint(
                (point as f64 / self.lfo_length as f64 * std::f64::consts::TAU
                    + 1.5 * std::f64::consts::PI)
                    .sin(),
                1.0,
            ),
            FlangerWave::Triangle => triangle_wave_value(point, self.lfo_length),
        };

        f64::from(value as f32)
    }

    #[cfg(test)]
    fn balanced_gains(self) -> (f64, f64) {
        (self.gain_in, self.width)
    }

    #[cfg(test)]
    fn line_lengths(self) -> (usize, usize) {
        (self.delay_line_length, self.lfo_length)
    }
}

#[allow(
    clippy::cast_precision_loss,
    reason = "flanger wave-table math mirrors SoX-ng's bounded table index conversions"
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

fn balanced_mix(width_percent: f64, regen_percent: f64) -> (f64, f64) {
    let regen = regen_percent / 100.0;
    if width_percent.is_infinite() {
        (0.0, 1.0 - regen.abs())
    } else {
        let width = width_percent / 100.0;
        let denominator = 1.0 + width;
        (1.0 / denominator, width / denominator * (1.0 - regen.abs()))
    }
}

fn integer_wave_offset(offset: f64) -> usize {
    float_offset_to_usize(offset + 0.5)
}

#[allow(
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    reason = "flanger offsets are non-negative and bounded by the allocated delay line"
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
    if frames.is_finite() && frames + 2.0 <= usize::MAX as f64 {
        Ok(frames)
    } else {
        Err(EffectError::FlangerLengthOverflow)
    }
}

fn f64_to_f32_clamped(sample: f64) -> f32 {
    #[allow(
        clippy::cast_possible_truncation,
        reason = "flanger output is explicitly clipped to normalized f32 full scale"
    )]
    {
        sample.clamp(-1.0, 1.0) as f32
    }
}

#[cfg(test)]
mod tests {
    use super::{Flanger, FlangerInterpolation, FlangerWave};
    use crate::EffectError;
    use auralis_core::{
        AudioBuffer, AudioSpec, ChannelCount, FrameCount, SampleFormat, SampleRate,
    };

    #[test]
    fn zero_delay_full_width_is_length_preserving_identity() {
        let audio = mono_audio_buffer(1_000, &[1.0, 0.0, -0.5]);
        let flanger = Flanger::new(
            0.0,
            0.0,
            0.0,
            100.0,
            1.0,
            FlangerWave::Sine,
            0.0,
            FlangerInterpolation::Linear,
        )
        .unwrap();

        let processed = flanger.process_buffer(&audio).unwrap();

        assert_eq!(processed.frames(), audio.frames());
        assert_eq!(processed.as_planar_f32(), audio.as_planar_f32());
    }

    #[test]
    fn swept_delay_preserves_length_and_produces_finite_samples() {
        let audio = mono_audio_buffer(1_000, &[1.0, 0.0, 0.25, -0.25]);
        let flanger = Flanger::new(
            1.0,
            2.0,
            25.0,
            71.0,
            1.0,
            FlangerWave::Triangle,
            25.0,
            FlangerInterpolation::Quadratic,
        )
        .unwrap();

        let processed = flanger.process_buffer(&audio).unwrap();

        assert_eq!(processed.frames(), audio.frames());
        assert!(
            processed
                .as_planar_f32()
                .iter()
                .all(|sample| sample.is_finite())
        );
    }

    #[test]
    fn balances_width_and_feedback_like_sox_ng() {
        let flanger = Flanger::new(
            0.0,
            0.0,
            25.0,
            100.0,
            1.0,
            FlangerWave::Sine,
            0.0,
            FlangerInterpolation::Linear,
        )
        .unwrap();

        let resolved = flanger.resolve(1_000).unwrap();

        assert_eq!(resolved.balanced_gains(), (0.5, 0.375));
    }

    #[test]
    fn resolves_zero_delay_depth_to_minimum_delay_line_with_interpolator_padding() {
        let flanger = Flanger::new(
            0.0,
            0.0,
            0.0,
            71.0,
            1.0,
            FlangerWave::Sine,
            0.0,
            FlangerInterpolation::None,
        )
        .unwrap();

        assert_eq!(flanger.resolve(1_000).unwrap().line_lengths(), (2, 1000));
    }

    #[test]
    fn rejects_invalid_configuration() {
        assert_eq!(
            Flanger::new(
                -1.0,
                0.0,
                0.0,
                71.0,
                1.0,
                FlangerWave::Sine,
                0.0,
                FlangerInterpolation::Linear,
            )
            .unwrap_err(),
            EffectError::InvalidFlanger
        );
        assert_eq!(
            Flanger::new(
                0.0,
                0.0,
                101.0,
                71.0,
                1.0,
                FlangerWave::Sine,
                0.0,
                FlangerInterpolation::Linear,
            )
            .unwrap_err(),
            EffectError::InvalidFlanger
        );
        assert_eq!(
            Flanger::new(
                0.0,
                0.0,
                0.0,
                71.0,
                2_000.0,
                FlangerWave::Sine,
                0.0,
                FlangerInterpolation::Linear,
            )
            .unwrap()
            .process_buffer(&mono_audio_buffer(1_000, &[1.0]))
            .unwrap_err(),
            EffectError::InvalidFlanger
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

use auralis_core::AudioBuffer;

use crate::{EffectError, Result};

const DEFAULT_GAIN_IN: f64 = 0.4;
const DEFAULT_GAIN_OUT: f64 = 0.74;
const DEFAULT_DELAY_MS: f64 = 3.0;
const DEFAULT_REGEN: f64 = 0.4;
const DEFAULT_SPEED_HZ: f64 = 0.5;

/// SoX-ng phaser delay-line interpolation mode.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum PhaserInterpolation {
    /// Use SoX-ng's rounded integer modulation table offsets.
    #[default]
    None,
    /// Linearly interpolate between adjacent delay-line samples.
    Linear,
    /// Use SoX-ng's quadratic interpolation across three delay-line samples.
    Quadratic,
}

/// SoX-ng phaser modulation waveform.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum PhaserWave {
    /// Sine modulation.
    #[default]
    Sine,
    /// Triangle modulation.
    Triangle,
}

/// SoX-ng-style scalar phaser.
///
/// SoX-ng's `phaser` is a length-preserving swept delay with feedback. The
/// processor applies input gain, feeds the mixed signal into independent
/// channel-local delay lines, clips after output gain, and does not emit a
/// delayed tail.
///
/// # Examples
///
/// ```
/// use auralis_core::{
///     AudioBuffer, AudioSpec, ChannelCount, FrameCount, SampleFormat, SampleRate,
/// };
/// use auralis_effects::Phaser;
///
/// let spec = AudioSpec::new(
///     SampleRate::new(1_000)?,
///     ChannelCount::new(1)?,
///     SampleFormat::Float32,
/// );
/// let audio = AudioBuffer::from_planar_f32(spec, FrameCount::new(3), vec![1.0, 0.0, 0.0])?;
/// let phaser = Phaser::with_defaults()?;
///
/// let processed = phaser.process_buffer(&audio)?;
/// assert_eq!(processed.frames(), audio.frames());
/// # Ok::<(), Box<dyn std::error::Error>>(())
/// ```
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Phaser {
    gain_in: f64,
    gain_out: f64,
    delay_ms: f64,
    regen: f64,
    speed_hz: f64,
    wave: PhaserWave,
    interpolation: PhaserInterpolation,
}

impl Phaser {
    /// Creates a phaser from SoX-ng command parameters.
    ///
    /// # Errors
    ///
    /// Returns [`EffectError::InvalidPhaser`] when gains or regeneration are
    /// outside `-1..=1`, delay is outside `0..=1000` ms, speed is outside
    /// `0..=192000` Hz, or any parameter is non-finite. A zero speed is accepted
    /// at construction but rejected at processing start to match SoX-ng's
    /// runtime validation style.
    pub fn new(
        gain_in: f64,
        gain_out: f64,
        delay_ms: f64,
        regen: f64,
        speed_hz: f64,
        wave: PhaserWave,
        interpolation: PhaserInterpolation,
    ) -> Result<Self> {
        if !gain_in.is_finite()
            || !(-1.0..=1.0).contains(&gain_in)
            || !gain_out.is_finite()
            || !(-1.0..=1.0).contains(&gain_out)
            || !delay_ms.is_finite()
            || !(0.0..=1000.0).contains(&delay_ms)
            || !regen.is_finite()
            || !(-1.0..=1.0).contains(&regen)
            || !speed_hz.is_finite()
            || !(0.0..=192_000.0).contains(&speed_hz)
        {
            return Err(EffectError::InvalidPhaser);
        }

        Ok(Self {
            gain_in,
            gain_out,
            delay_ms,
            regen,
            speed_hz,
            wave,
            interpolation,
        })
    }

    /// Creates a phaser using SoX-ng's scalar defaults.
    ///
    /// # Errors
    ///
    /// This constructor currently cannot fail because all defaults are valid.
    pub fn with_defaults() -> Result<Self> {
        Self::new(
            DEFAULT_GAIN_IN,
            DEFAULT_GAIN_OUT,
            DEFAULT_DELAY_MS,
            DEFAULT_REGEN,
            DEFAULT_SPEED_HZ,
            PhaserWave::Sine,
            PhaserInterpolation::None,
        )
    }

    /// Returns the proportion of input delivered to the delay and output path.
    #[must_use]
    pub const fn gain_in(self) -> f64 {
        self.gain_in
    }

    /// Returns the final output gain.
    #[must_use]
    pub const fn gain_out(self) -> f64 {
        self.gain_out
    }

    /// Returns the maximum delay in milliseconds.
    #[must_use]
    pub const fn delay_ms(self) -> f64 {
        self.delay_ms
    }

    /// Returns delay-line feedback.
    #[must_use]
    pub const fn regen(self) -> f64 {
        self.regen
    }

    /// Returns modulation speed in hertz.
    #[must_use]
    pub const fn speed_hz(self) -> f64 {
        self.speed_hz
    }

    /// Returns the modulation waveform.
    #[must_use]
    pub const fn wave(self) -> PhaserWave {
        self.wave
    }

    /// Returns the delay-line interpolation mode.
    #[must_use]
    pub const fn interpolation(self) -> PhaserInterpolation {
        self.interpolation
    }

    /// Applies phaser and returns a length-preserving output buffer.
    ///
    /// # Errors
    ///
    /// Returns [`EffectError::InvalidPhaser`] when the delay or modulation
    /// speed cannot be resolved at the input sample rate, or
    /// [`EffectError::PhaserLengthOverflow`] when delay-line allocation would
    /// exceed the platform limit.
    pub fn process_buffer(&self, audio: &AudioBuffer) -> Result<AudioBuffer> {
        let resolved = self.resolve(audio.spec().sample_rate().as_u32())?;
        let mut output = Vec::with_capacity(audio.as_planar_f32().len());

        for channel_index in 0..audio.channels().as_usize() {
            let channel = audio
                .channel(channel_index)
                .ok_or(EffectError::PhaserLengthOverflow)?;
            let mut state = PhaserState::new(resolved);
            for sample in channel.iter().copied() {
                output.push(state.process(f64::from(sample)));
            }
        }

        Ok(AudioBuffer::from_planar_f32(
            audio.spec(),
            audio.frames(),
            output,
        )?)
    }

    fn resolve(self, sample_rate_hz: u32) -> Result<ResolvedPhaser> {
        let sample_rate = f64::from(sample_rate_hz);
        if self.speed_hz == 0.0 || self.speed_hz > sample_rate {
            return Err(EffectError::InvalidPhaser);
        }

        let mut delay_line_length = frames_from_ms(self.delay_ms, sample_rate)?;
        if delay_line_length < 1 {
            return Err(EffectError::InvalidPhaser);
        }
        delay_line_length = delay_line_length
            .checked_add(match self.interpolation {
                PhaserInterpolation::None => 0,
                PhaserInterpolation::Linear => 1,
                PhaserInterpolation::Quadratic => 2,
            })
            .ok_or(EffectError::PhaserLengthOverflow)?;

        #[allow(
            clippy::cast_possible_truncation,
            clippy::cast_sign_loss,
            reason = "phaser LFO length is finite and range-checked"
        )]
        let modulation_length = (sample_rate / self.speed_hz) as usize;
        if modulation_length < 1 {
            return Err(EffectError::InvalidPhaser);
        }

        Ok(ResolvedPhaser {
            gain_in: self.gain_in,
            gain_out: self.gain_out,
            regen: self.regen,
            delay_line_length,
            modulation_length,
            wave: self.wave,
            interpolation: self.interpolation,
        })
    }
}

#[derive(Debug, Clone, Copy)]
struct ResolvedPhaser {
    gain_in: f64,
    gain_out: f64,
    regen: f64,
    delay_line_length: usize,
    modulation_length: usize,
    wave: PhaserWave,
    interpolation: PhaserInterpolation,
}

struct PhaserState {
    resolved: ResolvedPhaser,
    delay_line: Vec<f64>,
    delay_position: usize,
    modulation_position: usize,
}

impl PhaserState {
    fn new(resolved: ResolvedPhaser) -> Self {
        Self {
            resolved,
            delay_line: vec![0.0; resolved.delay_line_length],
            delay_position: 0,
            modulation_position: 0,
        }
    }

    fn process(&mut self, input_sample: f64) -> f32 {
        let offset = self.resolved.modulation_offset(self.modulation_position);
        let delayed = match self.resolved.interpolation {
            PhaserInterpolation::None => self.process_none(offset),
            PhaserInterpolation::Linear => self.process_linear(offset),
            PhaserInterpolation::Quadratic => self.process_quadratic(offset),
        };
        let delayed_mix = input_sample * self.resolved.gain_in + self.resolved.regen * delayed;

        self.modulation_position = (self.modulation_position + 1) % self.resolved.modulation_length;
        self.delay_position = (self.delay_position + 1) % self.resolved.delay_line_length;
        self.delay_line[self.delay_position] = delayed_mix;

        f64_to_f32_clamped(delayed_mix * self.resolved.gain_out)
    }

    fn process_none(&self, offset: f64) -> f64 {
        let delay_index = self.delay_index(integer_wave_offset(offset));
        self.delay_line[delay_index]
    }

    fn process_linear(&self, offset: f64) -> f64 {
        let offset_i = offset.trunc();
        let frac = offset - offset_i;
        let delay_index = self.delay_index(float_offset_to_usize(offset_i));
        let delayed_0 = self.delay_line[delay_index];
        let delayed_1 = self.delay_line[(delay_index + 1) % self.resolved.delay_line_length];

        delayed_0.mul_add(1.0 - frac, delayed_1 * frac)
    }

    fn process_quadratic(&self, offset: f64) -> f64 {
        let offset_i = offset.trunc();
        let frac = offset - offset_i;
        let delay_index = self.delay_index(float_offset_to_usize(offset_i));
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
        (self.delay_position + offset) % self.resolved.delay_line_length
    }
}

impl ResolvedPhaser {
    #[allow(
        clippy::cast_precision_loss,
        reason = "phaser wave-table range is bounded by the allocated delay line length"
    )]
    fn modulation_offset(self, position: usize) -> f64 {
        let value = match self.wave {
            PhaserWave::Sine => self.sine_wave_value(position),
            PhaserWave::Triangle => triangle_wave_value(position, self.modulation_length),
        };

        value * (self.delay_line_length as f64 - 1.0) + 1.0
    }

    #[allow(
        clippy::cast_precision_loss,
        reason = "phaser wave-table math mirrors SoX-ng's bounded table index conversions"
    )]
    fn sine_wave_value(self, position: usize) -> f64 {
        let phase_offset = quarter_cycle_phase_offset(self.modulation_length);
        let point = (position + phase_offset) % self.modulation_length;
        f64::midpoint(
            (point as f64 / self.modulation_length as f64 * std::f64::consts::TAU).sin(),
            1.0,
        )
    }

    #[cfg(test)]
    fn line_lengths(self) -> (usize, usize) {
        (self.delay_line_length, self.modulation_length)
    }
}

#[allow(
    clippy::cast_precision_loss,
    reason = "phaser wave-table math mirrors SoX-ng's bounded table index conversions"
)]
fn triangle_wave_value(position: usize, wave_length: usize) -> f64 {
    let phase_offset = quarter_cycle_phase_offset(wave_length);
    let point = (position + phase_offset) % wave_length;
    let d = point as f64 * 2.0 / wave_length as f64;
    match 4 * point / wave_length {
        0 => d + 0.5,
        1 | 2 => 1.5 - d,
        3 => d - 1.5,
        _ => 0.0,
    }
}

#[allow(
    clippy::cast_precision_loss,
    clippy::cast_sign_loss,
    clippy::cast_possible_truncation,
    reason = "phaser phase offset mirrors SoX-ng wave table initialization"
)]
fn quarter_cycle_phase_offset(wave_length: usize) -> usize {
    (wave_length as f64 * 0.25 + 0.5) as usize
}

fn integer_wave_offset(offset: f64) -> usize {
    float_offset_to_usize(offset + 0.5)
}

#[allow(
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    reason = "phaser offsets are non-negative and bounded by the allocated delay line"
)]
fn float_offset_to_usize(offset: f64) -> usize {
    offset as usize
}

#[allow(
    clippy::cast_precision_loss,
    reason = "this is a conservative preflight bound before converting frame counts to usize"
)]
fn frames_from_ms(milliseconds: f64, sample_rate: f64) -> Result<usize> {
    let frames = milliseconds * sample_rate / 1000.0;
    if frames.is_finite() && frames <= usize::MAX as f64 {
        #[allow(
            clippy::cast_possible_truncation,
            clippy::cast_sign_loss,
            reason = "phaser frame count is finite and range-checked"
        )]
        Ok(frames as usize)
    } else {
        Err(EffectError::PhaserLengthOverflow)
    }
}

fn f64_to_f32_clamped(sample: f64) -> f32 {
    #[allow(
        clippy::cast_possible_truncation,
        reason = "phaser output is explicitly clipped to normalized f32 full scale"
    )]
    {
        sample.clamp(-1.0, 1.0) as f32
    }
}

#[cfg(test)]
mod tests {
    use super::{Phaser, PhaserInterpolation, PhaserWave};
    use crate::EffectError;
    use auralis_core::{
        AudioBuffer, AudioSpec, ChannelCount, FrameCount, SampleFormat, SampleRate,
    };

    #[test]
    fn default_phaser_is_length_preserving_and_finite() {
        let audio = mono_audio_buffer(1_000, &[1.0, 0.0, 0.25, -0.25, 0.0]);
        let processed = Phaser::with_defaults()
            .unwrap()
            .process_buffer(&audio)
            .unwrap();

        assert_eq!(processed.frames(), audio.frames());
        assert!(
            processed
                .as_planar_f32()
                .iter()
                .all(|sample| sample.is_finite())
        );
    }

    #[test]
    fn stereo_processing_uses_channel_local_delay_lines() {
        let audio = stereo_audio_buffer(1_000, &[1.0, 0.0], &[0.0, 0.0]);
        let phaser = Phaser::new(
            1.0,
            1.0,
            1.0,
            1.0,
            1.0,
            PhaserWave::Sine,
            PhaserInterpolation::None,
        )
        .unwrap();

        let processed = phaser.process_buffer(&audio).unwrap();

        assert_eq!(
            processed.channel(1).unwrap()[1].to_bits(),
            0.0_f32.to_bits()
        );
    }

    #[test]
    fn resolves_delay_and_interpolator_lengths_like_sox_ng() {
        let phaser = Phaser::new(
            0.4,
            0.74,
            3.5,
            0.4,
            2.0,
            PhaserWave::Sine,
            PhaserInterpolation::Quadratic,
        )
        .unwrap();

        assert_eq!(phaser.resolve(1_000).unwrap().line_lengths(), (5, 500));
    }

    #[test]
    fn rejects_invalid_configuration() {
        assert_eq!(
            Phaser::new(
                1.1,
                0.74,
                3.0,
                0.4,
                0.5,
                PhaserWave::Sine,
                PhaserInterpolation::None,
            )
            .unwrap_err(),
            EffectError::InvalidPhaser
        );
        assert_eq!(
            Phaser::new(
                0.4,
                0.74,
                0.0,
                0.4,
                0.5,
                PhaserWave::Sine,
                PhaserInterpolation::None,
            )
            .unwrap()
            .process_buffer(&mono_audio_buffer(1_000, &[1.0]))
            .unwrap_err(),
            EffectError::InvalidPhaser
        );
        assert_eq!(
            Phaser::new(
                0.4,
                0.74,
                3.0,
                0.4,
                2_000.0,
                PhaserWave::Sine,
                PhaserInterpolation::None,
            )
            .unwrap()
            .process_buffer(&mono_audio_buffer(1_000, &[1.0]))
            .unwrap_err(),
            EffectError::InvalidPhaser
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

    fn stereo_audio_buffer(sample_rate_hz: u32, left: &[f32], right: &[f32]) -> AudioBuffer {
        assert_eq!(left.len(), right.len());
        let spec = AudioSpec::new(
            SampleRate::new(sample_rate_hz).unwrap(),
            ChannelCount::new(2).unwrap(),
            SampleFormat::Float32,
        );
        let mut samples = left.to_vec();
        samples.extend_from_slice(right);
        AudioBuffer::from_planar_f32(
            spec,
            FrameCount::new(u64::try_from(left.len()).unwrap()),
            samples,
        )
        .unwrap()
    }
}

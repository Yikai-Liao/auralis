use auralis_core::{AudioBuffer, AudioSpec, FrameCount, SampleRate};

use crate::{EffectError, Result};

const DEFAULT_FREQUENCY_HZ: f64 = 440.0;
const DEFAULT_SYNTH_RANDOM_SEED: i32 = 0;
const SOX_RANDOM_SCALE: f64 = 65536.0 * 32768.0;

/// Basic SoX-ng waveform and noise kinds supported by [`Synth`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SynthWaveform {
    /// Sinusoidal oscillator.
    Sine,
    /// Pulse wave, using `p1` as the high portion of the cycle.
    Square,
    /// Rising sawtooth oscillator.
    Sawtooth,
    /// Triangle oscillator, using `p1` as the peak position.
    Triangle,
    /// Trapezium oscillator, using `p1`, `p2`, and `p3` as segment boundaries.
    Trapezium,
    /// Exponential pulse, using `p1` as the peak position and `p2` as the range.
    Exp,
    /// Uniform white noise.
    WhiteNoise,
    /// Triangular probability density noise.
    TpdfNoise,
    /// Pink noise using SoX-ng's Paul Kellet filter shape.
    PinkNoise,
    /// Brown noise using SoX-ng's bounded random walk.
    BrownNoise,
}

/// SoX-ng-style synth frequency sweep family.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SynthSweep {
    /// `f1:f2`, linear frequency sweep.
    Linear,
    /// `f1+f2`, frequency proportional to time squared.
    Square,
    /// `f1/f2`, exponential sweep.
    Exponential,
    /// `f1-f2`, stepped exponential sweep starting each cycle at phase zero.
    ExponentialCycle,
}

/// Variable-delay parameters for `synth ... vdelay fixed[,extra[,mix]]`.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SynthVariableDelay {
    /// Fixed delay in milliseconds.
    pub fixed_ms: f64,
    /// Extra modulated delay depth in milliseconds.
    pub extra_ms: f64,
    /// Delayed-signal mix as a `0..=1` fraction.
    pub mix: f64,
}

impl SynthVariableDelay {
    /// Creates validated variable-delay parameters.
    ///
    /// # Errors
    ///
    /// Returns [`EffectError::InvalidSynth`] when any value is non-finite,
    /// either delay is negative, or `mix` is outside `0..=1`.
    pub fn new(fixed_ms: f64, extra_ms: f64, mix: f64) -> Result<Self> {
        if fixed_ms.is_finite()
            && fixed_ms >= 0.0
            && extra_ms.is_finite()
            && extra_ms >= 0.0
            && (0.0..=1.0).contains(&mix)
        {
            Ok(Self {
                fixed_ms,
                extra_ms,
                mix,
            })
        } else {
            Err(EffectError::InvalidSynth)
        }
    }
}

/// SoX-ng-style way to combine synthesized samples with input samples.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum SynthCombineMode {
    /// Replace input samples with synthesized samples.
    Create,
    /// Mix synthesized and input samples 50:50.
    Mix,
    /// Amplitude-modulate input by the synthesized wave mapped to `0..=1`.
    Amod,
    /// Frequency-style modulation: multiply input by the synthesized wave.
    Fmod,
    /// Use the synthesized wave to read from a variable delay line.
    Vdelay(SynthVariableDelay),
}

/// Optional SoX-ng-style `synth` output length.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum SynthLength {
    /// Length in seconds, resolved using the input sample rate.
    Seconds(f64),
    /// Length in decoded audio frames.
    Frames(FrameCount),
}

impl SynthLength {
    /// Creates a seconds-based synth length.
    ///
    /// # Errors
    ///
    /// Returns [`EffectError::InvalidSynth`] when `seconds` is non-finite or
    /// negative.
    pub fn seconds(seconds: f64) -> Result<Self> {
        if seconds.is_finite() && seconds >= 0.0 {
            Ok(Self::Seconds(seconds))
        } else {
            Err(EffectError::InvalidSynth)
        }
    }

    /// Creates a frame-count synth length.
    #[must_use]
    pub const fn frames(frames: FrameCount) -> Self {
        Self::Frames(frames)
    }

    #[allow(
        clippy::cast_precision_loss,
        clippy::cast_possible_truncation,
        clippy::cast_sign_loss,
        reason = "seconds are validated non-negative and finite before SoX-ng-style rounding"
    )]
    fn resolve(self, sample_rate: SampleRate) -> Result<FrameCount> {
        match self {
            Self::Seconds(seconds) => {
                let frames = seconds
                    .mul_add(f64::from(sample_rate.as_u32()), 0.5)
                    .floor();
                if !frames.is_finite() || frames > u64::MAX as f64 {
                    return Err(EffectError::InvalidSynth);
                }
                Ok(FrameCount::new(frames as u64))
            }
            Self::Frames(frames) => Ok(frames),
        }
    }
}

/// One channel specification for a basic SoX-ng-style synth oscillator.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SynthChannel {
    /// Waveform kind.
    pub waveform: SynthWaveform,
    /// Oscillator frequency in hertz.
    pub frequency_hz: f64,
    /// Optional ending frequency in hertz for sweep commands.
    pub frequency2_hz: Option<f64>,
    /// Optional frequency sweep family.
    pub sweep: Option<SynthSweep>,
    /// How this channel combines generated samples with input samples.
    pub combine: SynthCombineMode,
    /// DC offset in normalized full-scale units.
    pub offset: f64,
    /// Initial phase as a `0..=1` cycle fraction.
    pub phase: f64,
    /// Waveform-specific first shape parameter as a `0..=1` fraction.
    pub p1: f64,
    /// Waveform-specific second shape parameter as a `0..=1` fraction.
    pub p2: f64,
    /// Waveform-specific third shape parameter as a `0..=1` fraction.
    pub p3: f64,
}

impl SynthChannel {
    /// Creates a channel with SoX-ng defaults for the selected waveform.
    ///
    /// # Errors
    ///
    /// Returns [`EffectError::InvalidSynth`] when `frequency_hz` is non-finite
    /// or negative.
    pub fn new(waveform: SynthWaveform, frequency_hz: f64) -> Result<Self> {
        Self::with_parameters(waveform, frequency_hz, 0.0, 0.0, None, None, None)
    }

    /// Creates a channel with explicit normalized shape parameters.
    ///
    /// `offset` is in `-1..=1`; `phase`, `p1`, `p2`, and `p3` are fractions in
    /// `0..=1`.
    ///
    /// # Errors
    ///
    /// Returns [`EffectError::InvalidSynth`] when a value is outside the
    /// supported range or would make the waveform shape degenerate.
    pub fn with_parameters(
        waveform: SynthWaveform,
        frequency_hz: f64,
        offset: f64,
        phase: f64,
        p1: Option<f64>,
        p2: Option<f64>,
        p3: Option<f64>,
    ) -> Result<Self> {
        if !frequency_hz.is_finite()
            || frequency_hz < 0.0
            || !(-1.0..=1.0).contains(&offset)
            || !(0.0..=1.0).contains(&phase)
        {
            return Err(EffectError::InvalidSynth);
        }

        let (p1, p2, p3) = resolve_shape_parameters(waveform, p1, p2, p3)?;
        Ok(Self {
            waveform,
            frequency_hz,
            frequency2_hz: None,
            sweep: None,
            combine: SynthCombineMode::Create,
            offset,
            phase,
            p1,
            p2,
            p3,
        })
    }

    /// Returns this channel with a SoX-ng-style frequency sweep.
    ///
    /// # Errors
    ///
    /// Returns [`EffectError::InvalidSynth`] when the waveform cannot sweep or
    /// the ending frequency is invalid for the sweep family.
    pub fn with_sweep(mut self, sweep: SynthSweep, frequency2_hz: f64) -> Result<Self> {
        if !self.waveform.is_tonal()
            || !frequency2_hz.is_finite()
            || frequency2_hz < 0.0
            || matches!(
                sweep,
                SynthSweep::Exponential | SynthSweep::ExponentialCycle
            ) && (self.frequency_hz == 0.0 || frequency2_hz == 0.0)
        {
            return Err(EffectError::InvalidSynth);
        }
        self.sweep = Some(sweep);
        self.frequency2_hz = Some(frequency2_hz);
        Ok(self)
    }

    /// Returns this channel with a SoX-ng-style combine mode.
    #[must_use]
    pub const fn with_combine(mut self, combine: SynthCombineMode) -> Self {
        self.combine = combine;
        self
    }

    #[allow(
        clippy::cast_precision_loss,
        clippy::cast_possible_truncation,
        reason = "the public effect sample format is f32; oscillator math is evaluated in f64 then rounded once"
    )]
    #[cfg(test)]
    fn sample(self, frame: u64, sample_rate: SampleRate) -> f32 {
        let mut runtime = SynthChannelRuntime::default();
        self.generated_sample(
            frame,
            FrameCount::new(frame.saturating_add(1)),
            sample_rate,
            &mut SynthRandom::new(DEFAULT_SYNTH_RANDOM_SEED),
            &mut runtime,
        )
    }

    #[allow(
        clippy::cast_precision_loss,
        clippy::cast_possible_truncation,
        reason = "the public effect sample format is f32; oscillator math is evaluated in f64 then rounded once"
    )]
    fn generated_sample(
        self,
        frame: u64,
        total_frames: FrameCount,
        sample_rate: SampleRate,
        random: &mut SynthRandom,
        runtime: &mut SynthChannelRuntime,
    ) -> f32 {
        let sample = if self.waveform.is_tonal() {
            let phase = self
                .phase_at(frame, total_frames, sample_rate, runtime)
                .rem_euclid(1.0);
            self.tonal_sample(phase)
        } else {
            self.noise_sample(random, runtime)
        };

        (sample * (1.0 - self.offset.abs()) + self.offset) as f32
    }

    fn tonal_sample(self, phase: f64) -> f64 {
        match self.waveform {
            SynthWaveform::Sine => (std::f64::consts::TAU * phase).sin(),
            SynthWaveform::Square => -1.0 + 2.0 * f64::from(phase < self.p1),
            SynthWaveform::Sawtooth => -1.0 + 2.0 * phase,
            SynthWaveform::Triangle | SynthWaveform::Trapezium if phase < self.p1 => {
                -1.0 + 2.0 * phase / self.p1
            }
            SynthWaveform::Triangle => 1.0 - 2.0 * (phase - self.p1) / (1.0 - self.p1),
            SynthWaveform::Trapezium if phase < self.p2 => 1.0,
            SynthWaveform::Trapezium if phase < self.p3 => {
                1.0 - 2.0 * (phase - self.p2) / (self.p3 - self.p2)
            }
            SynthWaveform::Trapezium => -1.0,
            SynthWaveform::Exp => exponential_sample(phase, self.p1, self.p2),
            SynthWaveform::WhiteNoise
            | SynthWaveform::TpdfNoise
            | SynthWaveform::PinkNoise
            | SynthWaveform::BrownNoise => 0.0,
        }
    }

    #[allow(
        clippy::cast_precision_loss,
        reason = "SoX-ng-compatible sweep math is defined in floating-point sample counters"
    )]
    fn phase_at(
        self,
        frame: u64,
        total_frames: FrameCount,
        sample_rate: SampleRate,
        runtime: &mut SynthChannelRuntime,
    ) -> f64 {
        let sample_rate = f64::from(sample_rate.as_u32());
        let frame = frame as f64;
        let elapsed_time_s = frame / sample_rate;
        let total_frames = total_frames.as_u64() as f64;
        let Some(sweep) = self.sweep else {
            return self.frequency_hz * elapsed_time_s + self.phase;
        };
        let frequency2 = self.frequency2_hz.unwrap_or(self.frequency_hz);
        if total_frames == 0.0 {
            return self.frequency_hz * elapsed_time_s + self.phase;
        }
        let phase = match sweep {
            SynthSweep::Linear => {
                let mult = (frequency2 - self.frequency_hz) / total_frames / 2.0;
                (self.frequency_hz + frame * mult) * elapsed_time_s
            }
            SynthSweep::Square => {
                let mut mult =
                    (frequency2 - self.frequency_hz).abs().sqrt() / total_frames / 3.0_f64.sqrt();
                if self.frequency_hz > frequency2 {
                    mult = -mult;
                }
                (self.frequency_hz + mult.signum() * (frame * mult).powi(2)) * elapsed_time_s
            }
            SynthSweep::Exponential => {
                let mult = (frequency2 / self.frequency_hz).ln() / total_frames * sample_rate;
                if mult == 0.0 {
                    self.frequency_hz * elapsed_time_s
                } else {
                    (self.frequency_hz / mult) * (mult * elapsed_time_s).exp()
                }
            }
            SynthSweep::ExponentialCycle => {
                let mult = (frequency2.ln() - self.frequency_hz.ln()) / total_frames;
                let frequency = self.frequency_hz * (frame * mult).exp();
                let mut cycle_elapsed_time_s = elapsed_time_s - runtime.cycle_start_time_s;
                if frequency * cycle_elapsed_time_s >= 1.0 {
                    runtime.cycle_start_time_s += 1.0 / frequency;
                    cycle_elapsed_time_s = elapsed_time_s - runtime.cycle_start_time_s;
                }
                frequency * cycle_elapsed_time_s
            }
        };
        phase + self.phase
    }

    fn noise_sample(self, random: &mut SynthRandom, runtime: &mut SynthChannelRuntime) -> f64 {
        match self.waveform {
            SynthWaveform::WhiteNoise => random.next_f64(),
            SynthWaveform::TpdfNoise => 0.5 * (random.next_f64() + random.next_f64()),
            SynthWaveform::PinkNoise => {
                let random_sample = f64::from(random.next_i32());
                let scale = 0.125 / SOX_RANDOM_SCALE;
                runtime.c0 = 0.99886_f64.mul_add(runtime.c0, random_sample * 0.055_517_9 * scale);
                runtime.c1 = 0.99332_f64.mul_add(runtime.c1, random_sample * 0.075_075_9 * scale);
                runtime.c2 = 0.96900_f64.mul_add(runtime.c2, random_sample * 0.153_852_0 * scale);
                runtime.c3 = 0.86650_f64.mul_add(runtime.c3, random_sample * 0.310_485_6 * scale);
                runtime.c4 = 0.55000_f64.mul_add(runtime.c4, random_sample * 0.532_952_2 * scale);
                runtime.c5 =
                    (-0.7616_f64).mul_add(runtime.c5, -random_sample * 0.016_898_0 * scale);
                let sample = runtime.c0
                    + runtime.c1
                    + runtime.c2
                    + runtime.c3
                    + runtime.c4
                    + runtime.c5
                    + runtime.c6
                    + random_sample * 0.5362 * scale;
                runtime.c6 = random_sample * 0.115_926 * scale;
                sample
            }
            SynthWaveform::BrownNoise => loop {
                let sample = runtime.lp_last_out + random.next_f64() * (1.0 / 16.0);
                if sample.abs() <= 1.0 {
                    runtime.lp_last_out = sample;
                    break sample;
                }
            },
            SynthWaveform::Sine
            | SynthWaveform::Square
            | SynthWaveform::Sawtooth
            | SynthWaveform::Triangle
            | SynthWaveform::Trapezium
            | SynthWaveform::Exp => 0.0,
        }
    }
}

impl SynthWaveform {
    const fn is_tonal(self) -> bool {
        matches!(
            self,
            Self::Sine
                | Self::Square
                | Self::Sawtooth
                | Self::Triangle
                | Self::Trapezium
                | Self::Exp
        )
    }
}

/// SoX-ng-style synth processor.
///
/// `Synth` implements deterministic tonal waveform and noise generation over
/// existing decoded buffers. It preserves the input sample rate, channel count,
/// and sample format. If no explicit length is configured, output length
/// follows the input buffer.
#[derive(Debug, Clone, PartialEq)]
pub struct Synth {
    length: Option<SynthLength>,
    channels: Vec<SynthChannel>,
    no_headroom: bool,
}

impl Synth {
    /// Creates a default one-channel 440 Hz sine synth.
    ///
    /// Auralis repeats channel specifications modulo the input channel count,
    /// matching SoX-ng's channel assignment model.
    ///
    /// # Errors
    ///
    /// Returns [`EffectError::InvalidSynth`] only if default parameters become
    /// invalid.
    pub fn new() -> Result<Self> {
        Self::with_channels(
            None,
            [SynthChannel::new(
                SynthWaveform::Sine,
                DEFAULT_FREQUENCY_HZ,
            )?],
        )
    }

    /// Creates a synth from channel specifications.
    ///
    /// # Errors
    ///
    /// Returns [`EffectError::InvalidSynth`] when no channel specification is
    /// supplied.
    pub fn with_channels<I>(length: Option<SynthLength>, channels: I) -> Result<Self>
    where
        I: IntoIterator<Item = SynthChannel>,
    {
        let channels = channels.into_iter().collect::<Vec<_>>();
        if channels.is_empty() {
            return Err(EffectError::InvalidSynth);
        }
        if length.is_none() && channels.iter().any(|channel| channel.sweep.is_some()) {
            return Err(EffectError::InvalidSynth);
        }

        Ok(Self {
            length,
            channels,
            no_headroom: false,
        })
    }

    /// Returns a copy with SoX-ng's `-n` no-headroom flag recorded.
    #[must_use]
    pub const fn without_headroom(mut self) -> Self {
        self.no_headroom = true;
        self
    }

    /// Returns the optional output length.
    #[must_use]
    pub const fn length(&self) -> Option<SynthLength> {
        self.length
    }

    /// Returns the configured channel specifications.
    #[must_use]
    pub fn channel_specs(&self) -> &[SynthChannel] {
        &self.channels
    }

    /// Returns whether the SoX-ng `-n` no-headroom flag was supplied.
    #[must_use]
    pub const fn no_headroom(&self) -> bool {
        self.no_headroom
    }

    /// Generates synth output over an audio buffer's shape.
    ///
    /// # Errors
    ///
    /// Returns [`EffectError::InvalidSynth`] when the configured output length
    /// cannot be represented by the current platform or audio buffer shape.
    pub fn process_buffer(&self, audio: &AudioBuffer) -> Result<AudioBuffer> {
        let frames = match self.length {
            Some(length) => length.resolve(audio.spec().sample_rate())?,
            None => audio.frames(),
        };
        let frame_count =
            usize::try_from(frames.as_u64()).map_err(|_| EffectError::InvalidSynth)?;
        let channels = audio.channels().as_usize();
        let capacity = channels
            .checked_mul(frame_count)
            .ok_or(EffectError::InvalidSynth)?;
        let mut output = vec![0.0; capacity];
        let mut random = SynthRandom::new(DEFAULT_SYNTH_RANDOM_SEED);
        let sample_rate = audio.spec().sample_rate();
        let input_frames =
            usize::try_from(audio.frames().as_u64()).map_err(|_| EffectError::InvalidSynth)?;
        let input = audio.as_planar_f32();
        let channel_specs = (0..channels)
            .map(|channel_index| self.channels[channel_index % self.channels.len()])
            .collect::<Vec<_>>();
        let mut runtimes = self
            .channels
            .iter()
            .cycle()
            .take(channels)
            .map(|channel| SynthChannelRuntime::new(*channel, sample_rate))
            .collect::<Result<Vec<_>>>()?;

        for frame_index in 0..frame_count {
            let frame = u64::try_from(frame_index).map_err(|_| EffectError::InvalidSynth)?;
            for channel_index in 0..channels {
                let spec = channel_specs[channel_index];
                let generated = spec.generated_sample(
                    frame,
                    frames,
                    sample_rate,
                    &mut random,
                    &mut runtimes[channel_index],
                );
                let input_sample = if frame_index < input_frames {
                    input[channel_index * input_frames + frame_index]
                } else {
                    0.0
                };
                let output_sample = runtimes[channel_index].combine(
                    spec.combine,
                    generated,
                    input_sample,
                    sample_rate,
                );
                output[channel_index * frame_count + frame_index] = output_sample.clamp(-1.0, 1.0);
            }
        }

        let spec = AudioSpec::new(
            audio.spec().sample_rate(),
            audio.channels(),
            audio.spec().sample_format(),
        );
        Ok(AudioBuffer::from_planar_f32(spec, frames, output)?)
    }
}

fn resolve_shape_parameters(
    waveform: SynthWaveform,
    p1: Option<f64>,
    p2: Option<f64>,
    p3: Option<f64>,
) -> Result<(f64, f64, f64)> {
    let (p1, p2, p3) = match waveform {
        SynthWaveform::Sine
        | SynthWaveform::Sawtooth
        | SynthWaveform::WhiteNoise
        | SynthWaveform::TpdfNoise
        | SynthWaveform::PinkNoise
        | SynthWaveform::BrownNoise => (0.0, 0.0, 0.0),
        SynthWaveform::Square | SynthWaveform::Triangle => (p1.unwrap_or(0.5), 0.0, 0.0),
        SynthWaveform::Trapezium => match (p1, p2, p3) {
            (None, _, _) => (0.1, 0.5, 0.6),
            (Some(rise), None, _) if rise <= 0.5 => {
                let high_start = (1.0 - 2.0 * rise) / 2.0;
                (rise, high_start, high_start + rise)
            }
            (Some(rise), None, _) => (rise, rise, 1.0),
            (Some(rise), Some(fall_start), None) => (rise, fall_start, 1.0),
            (Some(rise), Some(fall_start), Some(low_start)) => (rise, fall_start, low_start),
        },
        SynthWaveform::Exp => (p1.unwrap_or(0.5), p2.unwrap_or(0.5), 0.0),
    };

    if [p1, p2, p3].iter().any(|value| !value.is_finite()) {
        return Err(EffectError::InvalidSynth);
    }
    if matches!(
        waveform,
        SynthWaveform::Triangle | SynthWaveform::Trapezium | SynthWaveform::Exp
    ) && (!(0.0..1.0).contains(&p1) || p1 == 0.0)
    {
        return Err(EffectError::InvalidSynth);
    }

    match waveform {
        SynthWaveform::Square if !(0.0..=1.0).contains(&p1) => Err(EffectError::InvalidSynth),
        SynthWaveform::Trapezium if p2 < p1 || p2 > 1.0 || p3 <= p2 || p3 > 1.0 => {
            Err(EffectError::InvalidSynth)
        }
        SynthWaveform::Exp if !(0.0..=1.0).contains(&p2) => Err(EffectError::InvalidSynth),
        _ => Ok((p1, p2, p3)),
    }
}

#[derive(Debug, Clone)]
struct SynthChannelRuntime {
    cycle_start_time_s: f64,
    lp_last_out: f64,
    c0: f64,
    c1: f64,
    c2: f64,
    c3: f64,
    c4: f64,
    c5: f64,
    c6: f64,
    vdelay_buffer: Vec<f32>,
    vdelay_pos: usize,
}

impl SynthChannelRuntime {
    fn new(channel: SynthChannel, sample_rate: SampleRate) -> Result<Self> {
        let mut runtime = Self::default();
        if let SynthCombineMode::Vdelay(delay) = channel.combine {
            runtime.vdelay_buffer =
                vec![0.0; vdelay_buffer_len(delay, sample_rate).ok_or(EffectError::InvalidSynth)?];
        }
        Ok(runtime)
    }

    fn combine(
        &mut self,
        mode: SynthCombineMode,
        generated: f32,
        input: f32,
        sample_rate: SampleRate,
    ) -> f32 {
        match mode {
            SynthCombineMode::Create => generated,
            SynthCombineMode::Mix => (generated + input) * 0.5,
            SynthCombineMode::Amod => (generated + 1.0) * input * 0.5,
            SynthCombineMode::Fmod => generated * input,
            SynthCombineMode::Vdelay(delay) => {
                self.combine_vdelay(delay, generated, input, sample_rate)
            }
        }
    }

    #[allow(
        clippy::cast_possible_truncation,
        reason = "SoX-ng vdelay interpolation is evaluated in f64 then rounded to Auralis' f32 sample format"
    )]
    fn combine_vdelay(
        &mut self,
        delay: SynthVariableDelay,
        generated: f32,
        input: f32,
        sample_rate: SampleRate,
    ) -> f32 {
        let len = self.vdelay_buffer.len();
        if len == 0 {
            return input;
        }
        self.vdelay_buffer[self.vdelay_pos] = input;
        let offset = ((delay.fixed_ms + delay.extra_ms * f64::from((generated + 1.0) * 0.5))
            / 1000.0)
            * f64::from(sample_rate.as_u32());
        let left_index = wrapped_delay_index(self.vdelay_pos, offset.ceil(), len);
        let right_index = wrapped_delay_index(self.vdelay_pos, offset.trunc(), len);
        let delayed = if left_index == right_index {
            self.vdelay_buffer[left_index]
        } else {
            let fraction = 1.0 - (offset - offset.trunc());
            let left = f64::from(self.vdelay_buffer[left_index]);
            let right = f64::from(self.vdelay_buffer[right_index]);
            (left.mul_add(1.0 - fraction, right * fraction)) as f32
        };
        self.vdelay_pos = (self.vdelay_pos + 1) % len;
        input.mul_add(1.0 - delay.mix as f32, delayed * delay.mix as f32)
    }
}

impl Default for SynthChannelRuntime {
    fn default() -> Self {
        Self {
            cycle_start_time_s: 0.0,
            lp_last_out: 0.0,
            c0: 0.0,
            c1: 0.0,
            c2: 0.0,
            c3: 0.0,
            c4: 0.0,
            c5: 0.0,
            c6: 0.0,
            vdelay_buffer: Vec::new(),
            vdelay_pos: 0,
        }
    }
}

#[allow(
    clippy::cast_possible_truncation,
    clippy::cast_precision_loss,
    clippy::cast_sign_loss,
    reason = "validated millisecond delays are converted to SoX-ng-style delay-line lengths"
)]
fn vdelay_buffer_len(delay: SynthVariableDelay, sample_rate: SampleRate) -> Option<usize> {
    let len =
        ((delay.fixed_ms + delay.extra_ms) / 1000.0).mul_add(f64::from(sample_rate.as_u32()), 2.0);
    if len.is_finite() && len >= 0.0 && len <= usize::MAX as f64 {
        Some(len as usize)
    } else {
        None
    }
}

#[allow(
    clippy::cast_possible_truncation,
    clippy::cast_possible_wrap,
    reason = "delay offsets are bounded by the allocated ring buffer length"
)]
fn wrapped_delay_index(position: usize, offset: f64, len: usize) -> usize {
    let len = len as isize;
    let index = position as isize - offset as isize;
    index.rem_euclid(len) as usize
}

#[derive(Debug, Clone, Copy)]
struct SynthRandom {
    state: i32,
}

impl SynthRandom {
    const fn new(seed: i32) -> Self {
        Self { state: seed }
    }

    fn next_i32(&mut self) -> i32 {
        self.state = self
            .state
            .wrapping_mul(1_664_525)
            .wrapping_add(1_013_904_223);
        self.state
    }

    fn next_f64(&mut self) -> f64 {
        f64::from(self.next_i32()) / SOX_RANDOM_SCALE
    }
}

fn exponential_sample(phase: f64, peak: f64, range: f64) -> f64 {
    let floor = 10.0_f64.powf(range * -10.0);
    let shaped = if phase < peak {
        floor * (phase * (1.0 / floor).ln() / peak).exp()
    } else {
        floor * ((1.0 - phase) * (1.0 / floor).ln() / (1.0 - peak)).exp()
    };
    shaped * 2.0 - 1.0
}

#[cfg(test)]
mod tests {
    use super::{
        Synth, SynthChannel, SynthCombineMode, SynthLength, SynthSweep, SynthVariableDelay,
        SynthWaveform,
    };
    use crate::EffectError;
    use auralis_core::{
        AudioBuffer, AudioSpec, ChannelCount, FrameCount, SampleFormat, SampleRate,
    };

    #[test]
    fn default_sine_replaces_input_using_input_length() {
        let audio = audio_buffer(4, 1, vec![0.25, 0.25, 0.25, 0.25]);

        let synthesized =
            Synth::with_channels(None, [SynthChannel::new(SynthWaveform::Sine, 1.0).unwrap()])
                .unwrap()
                .process_buffer(&audio)
                .unwrap();

        assert_samples_close(synthesized.as_planar_f32(), &[0.0, 1.0, 0.0, -1.0]);
    }

    #[test]
    fn explicit_frame_length_and_channel_repetition_are_supported() {
        let audio = audio_buffer(4, 2, vec![0.0, 0.0, 0.0, 0.0]);
        let square = SynthChannel::new(SynthWaveform::Square, 1.0).unwrap();

        let synthesized =
            Synth::with_channels(Some(SynthLength::frames(FrameCount::new(2))), [square])
                .unwrap()
                .process_buffer(&audio)
                .unwrap();

        assert_eq!(synthesized.frames(), FrameCount::new(2));
        assert_samples_close(synthesized.as_planar_f32(), &[1.0, 1.0, 1.0, 1.0]);
    }

    #[test]
    fn shape_parameters_match_basic_waveform_formulas() {
        let saw = SynthChannel::new(SynthWaveform::Sawtooth, 1.0).unwrap();
        let triangle = SynthChannel::with_parameters(
            SynthWaveform::Triangle,
            1.0,
            0.0,
            0.0,
            Some(0.25),
            None,
            None,
        )
        .unwrap();
        assert_eq!(
            saw.sample(1, SampleRate::new(4).unwrap()).to_bits(),
            (-0.5_f32).to_bits()
        );
        assert_eq!(
            triangle.sample(1, SampleRate::new(4).unwrap()).to_bits(),
            1.0_f32.to_bits()
        );
    }

    #[test]
    fn noise_generation_uses_repeatable_sox_style_prng() {
        let audio = audio_buffer(4, 1, vec![0.0, 0.0]);
        let synthesized = Synth::with_channels(
            None,
            [SynthChannel::new(SynthWaveform::WhiteNoise, 440.0).unwrap()],
        )
        .unwrap()
        .process_buffer(&audio)
        .unwrap();

        assert_samples_close(synthesized.as_planar_f32(), &[0.472_135_93, 0.557_133_8]);
    }

    #[test]
    fn sweeps_and_combine_modes_are_supported() {
        let audio = audio_buffer(4, 1, vec![0.25, 0.25, 0.25, 0.25]);
        let channel = SynthChannel::new(SynthWaveform::Sine, 1.0)
            .unwrap()
            .with_sweep(SynthSweep::Linear, 2.0)
            .unwrap()
            .with_combine(SynthCombineMode::Fmod);

        let synthesized =
            Synth::with_channels(Some(SynthLength::frames(FrameCount::new(4))), [channel])
                .unwrap()
                .process_buffer(&audio)
                .unwrap();

        assert_eq!(synthesized.frames(), FrameCount::new(4));
        assert!(
            synthesized
                .as_planar_f32()
                .iter()
                .all(|sample| sample.is_finite())
        );
    }

    #[test]
    fn variable_delay_combines_input_with_delayed_samples() {
        let audio = audio_buffer(4, 1, vec![0.25, 0.5, 0.75, 1.0]);
        let delay = SynthVariableDelay::new(250.0, 0.0, 1.0).unwrap();
        let channel = SynthChannel::new(SynthWaveform::Sine, 1.0)
            .unwrap()
            .with_combine(SynthCombineMode::Vdelay(delay));

        let synthesized = Synth::with_channels(None, [channel])
            .unwrap()
            .process_buffer(&audio)
            .unwrap();

        assert_samples_close(synthesized.as_planar_f32(), &[0.0, 0.25, 0.5, 0.75]);
    }

    #[test]
    fn invalid_values_are_rejected() {
        assert_eq!(
            SynthChannel::new(SynthWaveform::Sine, f64::NAN).unwrap_err(),
            EffectError::InvalidSynth
        );
        assert_eq!(
            SynthChannel::with_parameters(
                SynthWaveform::Triangle,
                440.0,
                0.0,
                0.0,
                Some(0.0),
                None,
                None,
            )
            .unwrap_err(),
            EffectError::InvalidSynth
        );
        assert_eq!(
            Synth::with_channels(None, []).unwrap_err(),
            EffectError::InvalidSynth
        );
        assert_eq!(
            SynthLength::seconds(f64::INFINITY).unwrap_err(),
            EffectError::InvalidSynth
        );
        assert_eq!(
            SynthChannel::new(SynthWaveform::WhiteNoise, 440.0)
                .unwrap()
                .with_sweep(SynthSweep::Linear, 880.0)
                .unwrap_err(),
            EffectError::InvalidSynth
        );
        assert_eq!(
            Synth::with_channels(
                None,
                [SynthChannel::new(SynthWaveform::Sine, 440.0)
                    .unwrap()
                    .with_sweep(SynthSweep::Linear, 880.0)
                    .unwrap()]
            )
            .unwrap_err(),
            EffectError::InvalidSynth
        );
    }

    fn audio_buffer(sample_rate: u32, channels: u16, samples: Vec<f32>) -> AudioBuffer {
        let spec = AudioSpec::new(
            SampleRate::new(sample_rate).unwrap(),
            ChannelCount::new(channels).unwrap(),
            SampleFormat::Float32,
        );
        AudioBuffer::from_planar_f32(
            spec,
            FrameCount::new(u64::try_from(samples.len() / usize::from(channels)).unwrap()),
            samples,
        )
        .unwrap()
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

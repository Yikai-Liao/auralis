use auralis_core::{AudioBuffer, AudioSpec, FrameCount, SampleRate};

use crate::{EffectError, Result};

const DEFAULT_FREQUENCY_HZ: f64 = 440.0;

/// Basic SoX-ng tonal waveform kinds supported by [`Synth`].
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
            offset,
            phase,
            p1,
            p2,
            p3,
        })
    }

    #[allow(
        clippy::cast_precision_loss,
        clippy::cast_possible_truncation,
        reason = "the public effect sample format is f32; oscillator math is evaluated in f64 then rounded once"
    )]
    fn sample(self, frame: u64, sample_rate: SampleRate) -> f32 {
        let sample_rate = f64::from(sample_rate.as_u32());
        let phase = (self.frequency_hz * frame as f64 / sample_rate + self.phase).rem_euclid(1.0);
        let sample = match self.waveform {
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
        };

        (sample * (1.0 - self.offset.abs()) + self.offset) as f32
    }
}

/// SoX-ng-style basic synth create-mode processor.
///
/// `Synth` currently implements deterministic tonal waveform creation for
/// existing decoded buffers. It replaces input samples with generated output,
/// preserving the input sample rate, channel count, and sample format. If no
/// explicit length is configured, output length follows the input buffer.
/// Noise, sweeps, and input-combine modes are intentionally left to the later
/// `synth` feature.
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
        let mut output = Vec::with_capacity(capacity);

        for channel_index in 0..channels {
            let spec = self.channels[channel_index % self.channels.len()];
            for frame in 0..frames.as_u64() {
                output.push(spec.sample(frame, audio.spec().sample_rate()));
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
        SynthWaveform::Sine | SynthWaveform::Sawtooth => (0.0, 0.0, 0.0),
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
    use super::{Synth, SynthChannel, SynthLength, SynthWaveform};
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

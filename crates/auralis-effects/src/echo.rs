use auralis_core::{AudioBuffer, FrameCount};

use crate::{EffectError, Result};

/// One SoX-ng-style echo delay tap.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct EchoTap {
    delay_ms: f64,
    decay: f64,
}

impl EchoTap {
    /// Creates an echo tap from a delay in milliseconds and a decay multiplier.
    ///
    /// # Errors
    ///
    /// Returns [`EffectError::InvalidEcho`] when the delay is negative or any
    /// value is non-finite.
    pub fn new(delay_ms: f64, decay: f64) -> Result<Self> {
        if delay_ms.is_finite() && delay_ms >= 0.0 && decay.is_finite() {
            Ok(Self { delay_ms, decay })
        } else {
            Err(EffectError::InvalidEcho)
        }
    }

    /// Returns the delay in milliseconds.
    #[must_use]
    pub const fn delay_ms(self) -> f64 {
        self.delay_ms
    }

    /// Returns the delayed-signal decay multiplier.
    #[must_use]
    pub const fn decay(self) -> f64 {
        self.decay
    }

    #[allow(
        clippy::cast_precision_loss,
        reason = "the comparison only rejects values outside the representable delay range"
    )]
    fn resolved_frames(self, sample_rate_hz: u32) -> Result<FrameCount> {
        let frames = self.delay_ms * f64::from(sample_rate_hz) / 1000.0;
        if !frames.is_finite() || frames > u64::MAX as f64 {
            return Err(EffectError::EchoLengthOverflow);
        }

        #[allow(
            clippy::cast_possible_truncation,
            clippy::cast_sign_loss,
            reason = "delay was validated non-negative, finite, and in u64 range"
        )]
        Ok(FrameCount::new(frames as u64))
    }
}

/// SoX-ng-style parallel echo delay line.
///
/// `Echo` applies a clean input gain, adds one or more delayed copies of the
/// input scaled by each tap decay, applies a final output gain, clips to the
/// normalized full-scale range, and extends the output by the largest resolved
/// delay. Delay values are milliseconds and resolve against the input sample
/// rate at processing time.
///
/// # Errors
///
/// [`Self::process_buffer`] returns [`EffectError::EchoLengthOverflow`] when a
/// delay or output shape cannot be represented.
///
/// # Examples
///
/// ```
/// use auralis_core::{
///     AudioBuffer, AudioSpec, ChannelCount, FrameCount, SampleFormat, SampleRate,
/// };
/// use auralis_effects::{Echo, EchoTap};
///
/// let spec = AudioSpec::new(
///     SampleRate::new(1_000)?,
///     ChannelCount::new(1)?,
///     SampleFormat::Float32,
/// );
/// let audio = AudioBuffer::from_planar_f32(spec, FrameCount::new(2), vec![1.0, 0.0])?;
///
/// let echoed = Echo::new(1.0, 1.0, [EchoTap::new(1.0, 0.5)?])?.process_buffer(&audio)?;
///
/// assert_eq!(echoed.as_planar_f32(), &[1.0, 0.5, 0.0]);
/// # Ok::<(), Box<dyn std::error::Error>>(())
/// ```
#[derive(Debug, Clone, PartialEq)]
pub struct Echo {
    gain_in: f64,
    gain_out: f64,
    taps: Vec<EchoTap>,
}

impl Echo {
    /// Creates an echo processor.
    ///
    /// # Errors
    ///
    /// Returns [`EffectError::InvalidEcho`] when either gain is non-finite or
    /// when no delay taps are supplied.
    pub fn new<I>(gain_in: f64, gain_out: f64, taps: I) -> Result<Self>
    where
        I: IntoIterator<Item = EchoTap>,
    {
        let taps = taps.into_iter().collect::<Vec<_>>();
        if gain_in.is_finite() && gain_out.is_finite() && !taps.is_empty() {
            Ok(Self {
                gain_in,
                gain_out,
                taps,
            })
        } else {
            Err(EffectError::InvalidEcho)
        }
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

    /// Returns the configured echo taps.
    #[must_use]
    pub fn taps(&self) -> &[EchoTap] {
        &self.taps
    }

    /// Applies the echo and returns an output buffer extended by the tail.
    ///
    /// # Errors
    ///
    /// Returns [`EffectError::EchoLengthOverflow`] when delay resolution or
    /// output allocation would overflow.
    pub fn process_buffer(&self, audio: &AudioBuffer) -> Result<AudioBuffer> {
        let resolved_taps = self.resolved_taps(audio.spec().sample_rate().as_u32())?;
        let max_delay = resolved_taps
            .iter()
            .map(|(delay, _decay)| *delay)
            .max()
            .unwrap_or(0);
        let input_frames = usize::try_from(audio.frames().as_u64())
            .map_err(|_| EffectError::EchoLengthOverflow)?;
        let output_frames = input_frames
            .checked_add(max_delay)
            .ok_or(EffectError::EchoLengthOverflow)?;
        let capacity = audio
            .channels()
            .as_usize()
            .checked_mul(output_frames)
            .ok_or(EffectError::EchoLengthOverflow)?;
        let mut output = Vec::with_capacity(capacity);

        for channel_index in 0..audio.channels().as_usize() {
            let channel = audio
                .channel(channel_index)
                .ok_or(EffectError::EchoLengthOverflow)?;
            process_channel(
                channel,
                output_frames,
                max_delay,
                self.gain_in,
                self.gain_out,
                &resolved_taps,
                &mut output,
            );
        }

        Ok(AudioBuffer::from_planar_f32(
            audio.spec(),
            FrameCount::new(
                u64::try_from(output_frames).map_err(|_| EffectError::EchoLengthOverflow)?,
            ),
            output,
        )?)
    }

    #[allow(
        clippy::cast_possible_truncation,
        reason = "echo delay processing stores decay coefficients as f32 to match f32 audio buffers"
    )]
    fn resolved_taps(&self, sample_rate_hz: u32) -> Result<Vec<(usize, f32)>> {
        self.taps
            .iter()
            .map(|tap| {
                let frames = tap.resolved_frames(sample_rate_hz)?;
                usize::try_from(frames.as_u64())
                    .map(|frames| (frames, tap.decay() as f32))
                    .map_err(|_| EffectError::EchoLengthOverflow)
            })
            .collect()
    }
}

fn process_channel(
    input: &[f32],
    output_frames: usize,
    max_delay: usize,
    gain_in: f64,
    gain_out: f64,
    taps: &[(usize, f32)],
    output: &mut Vec<f32>,
) {
    #[allow(
        clippy::cast_possible_truncation,
        reason = "echo delay processing stores gains as f32 to match f32 audio buffers"
    )]
    let gain_in = gain_in as f32;
    #[allow(
        clippy::cast_possible_truncation,
        reason = "echo delay processing stores gains as f32 to match f32 audio buffers"
    )]
    let gain_out = gain_out as f32;
    let mut delay_buffer = vec![0.0_f32; max_delay];
    let mut counter = 0;

    for frame in 0..output_frames {
        let input_sample = input.get(frame).copied().unwrap_or(0.0);
        let mut output_sample = input_sample * gain_in;

        if max_delay == 0 {
            for (_delay, decay) in taps {
                output_sample += input_sample * decay;
            }
        } else {
            for (delay, decay) in taps {
                let delayed_index = (counter + max_delay - delay) % max_delay;
                output_sample += delay_buffer[delayed_index] * decay;
            }
            delay_buffer[counter] = input_sample;
            counter = (counter + 1) % max_delay;
        }

        output.push((output_sample * gain_out).clamp(-1.0, 1.0));
    }
}

#[cfg(test)]
mod tests {
    use super::{Echo, EchoTap};
    use crate::{EffectError, test_support::stereo_audio_buffer};
    use auralis_core::{
        AudioBuffer, AudioSpec, ChannelCount, FrameCount, SampleFormat, SampleRate,
    };

    #[test]
    fn parallel_echo_extends_by_maximum_delay() {
        let audio = mono_audio_buffer(1_000, &[1.0, 0.0, 0.25]);
        let echo = Echo::new(
            0.5,
            1.0,
            [
                EchoTap::new(1.0, 0.25).unwrap(),
                EchoTap::new(2.0, 0.125).unwrap(),
            ],
        )
        .unwrap();

        let echoed = echo.process_buffer(&audio).unwrap();

        assert_eq!(echoed.frames(), FrameCount::new(5));
        assert_eq!(echoed.as_planar_f32(), &[0.5, 0.25, 0.25, 0.0625, 0.03125]);
    }

    #[test]
    fn zero_delay_tap_is_immediate_when_all_taps_are_zero_delay() {
        let audio = stereo_audio_buffer(vec![0.25, -0.5, 0.5, -0.25]);
        let echo = Echo::new(0.5, 0.5, [EchoTap::new(0.0, 0.5).unwrap()]).unwrap();

        let echoed = echo.process_buffer(&audio).unwrap();

        assert_eq!(echoed.frames(), audio.frames());
        assert_eq!(echoed.as_planar_f32(), &[0.125, -0.25, 0.25, -0.125]);
    }

    #[test]
    fn clips_inside_effect() {
        let audio = mono_audio_buffer(1_000, &[1.0]);
        let echo = Echo::new(1.0, 1.0, [EchoTap::new(0.0, 1.0).unwrap()]).unwrap();

        let echoed = echo.process_buffer(&audio).unwrap();

        assert_eq!(echoed.as_planar_f32(), &[1.0]);
    }

    #[test]
    fn rejects_invalid_config() {
        assert_eq!(
            EchoTap::new(-1.0, 0.5).unwrap_err(),
            EffectError::InvalidEcho
        );
        assert_eq!(
            Echo::new(f64::NAN, 1.0, [EchoTap::new(1.0, 0.5).unwrap()]).unwrap_err(),
            EffectError::InvalidEcho
        );
        assert_eq!(
            Echo::new(1.0, 1.0, []).unwrap_err(),
            EffectError::InvalidEcho
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

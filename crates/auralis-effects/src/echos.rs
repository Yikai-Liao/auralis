use auralis_core::{AudioBuffer, FrameCount};

use crate::{EffectError, Result};

/// One SoX-ng-style cascaded echos delay tap.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct EchosTap {
    delay_ms: f64,
    decay: f64,
}

impl EchosTap {
    /// Creates an echos tap from a delay in milliseconds and a decay multiplier.
    ///
    /// # Errors
    ///
    /// Returns [`EffectError::InvalidEchos`] when the delay is negative,
    /// either value is non-finite, or decay is outside `0..=1`.
    pub fn new(delay_ms: f64, decay: f64) -> Result<Self> {
        if delay_ms.is_finite()
            && delay_ms >= 0.0
            && decay.is_finite()
            && (0.0..=1.0).contains(&decay)
        {
            Ok(Self { delay_ms, decay })
        } else {
            Err(EffectError::InvalidEchos)
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
            return Err(EffectError::EchosLengthOverflow);
        }

        #[allow(
            clippy::cast_possible_truncation,
            clippy::cast_sign_loss,
            reason = "delay was validated non-negative, finite, and in u64 range"
        )]
        let frames = frames as u64;
        if frames == 0 {
            return Err(EffectError::InvalidEchos);
        }

        Ok(FrameCount::new(frames))
    }
}

/// SoX-ng-style cascaded echo delay line.
///
/// `Echos` applies a clean input gain, sums the current output of each
/// cascaded delay line, applies a final output gain, clips to normalized full
/// scale, and extends the output by the sum of all resolved tap delays. Unlike
/// [`crate::Echo`], later delay lines are fed by earlier delay lines plus the
/// current input, so multi-tap echoes include echoes of earlier echoes.
#[derive(Debug, Clone, PartialEq)]
pub struct Echos {
    gain_in: f64,
    gain_out: f64,
    taps: Vec<EchosTap>,
}

impl Echos {
    /// Creates an echos processor.
    ///
    /// # Errors
    ///
    /// Returns [`EffectError::InvalidEchos`] when either gain is non-finite or
    /// when no delay taps are supplied.
    pub fn new<I>(gain_in: f64, gain_out: f64, taps: I) -> Result<Self>
    where
        I: IntoIterator<Item = EchosTap>,
    {
        let taps = taps.into_iter().collect::<Vec<_>>();
        if gain_in.is_finite() && gain_out.is_finite() && !taps.is_empty() {
            Ok(Self {
                gain_in,
                gain_out,
                taps,
            })
        } else {
            Err(EffectError::InvalidEchos)
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

    /// Returns the configured cascaded echo taps.
    #[must_use]
    pub fn taps(&self) -> &[EchosTap] {
        &self.taps
    }

    /// Applies the cascaded echoes and returns an output buffer extended by the tail.
    ///
    /// # Errors
    ///
    /// Returns [`EffectError::InvalidEchos`] when a configured delay resolves
    /// to less than one frame, or [`EffectError::EchosLengthOverflow`] when the
    /// output shape cannot be represented.
    pub fn process_buffer(&self, audio: &AudioBuffer) -> Result<AudioBuffer> {
        let resolved_taps = self.resolved_taps(audio.spec().sample_rate().as_u32())?;
        let input_frames = usize::try_from(audio.frames().as_u64())
            .map_err(|_| EffectError::EchosLengthOverflow)?;
        let tail_frames = resolved_taps
            .iter()
            .try_fold(0_usize, |sum, (delay, _decay)| {
                sum.checked_add(*delay)
                    .ok_or(EffectError::EchosLengthOverflow)
            })?;
        let output_frames = input_frames
            .checked_add(tail_frames)
            .ok_or(EffectError::EchosLengthOverflow)?;
        let capacity = audio
            .channels()
            .as_usize()
            .checked_mul(output_frames)
            .ok_or(EffectError::EchosLengthOverflow)?;
        let mut output = Vec::with_capacity(capacity);

        for channel_index in 0..audio.channels().as_usize() {
            let channel = audio
                .channel(channel_index)
                .ok_or(EffectError::EchosLengthOverflow)?;
            process_channel(
                channel,
                output_frames,
                self.gain_in,
                self.gain_out,
                &resolved_taps,
                &mut output,
            );
        }

        Ok(AudioBuffer::from_planar_f32(
            audio.spec(),
            FrameCount::new(
                u64::try_from(output_frames).map_err(|_| EffectError::EchosLengthOverflow)?,
            ),
            output,
        )?)
    }

    fn resolved_taps(&self, sample_rate_hz: u32) -> Result<Vec<(usize, f64)>> {
        self.taps
            .iter()
            .map(|tap| {
                let frames = tap.resolved_frames(sample_rate_hz)?;
                usize::try_from(frames.as_u64())
                    .map(|frames| (frames, tap.decay()))
                    .map_err(|_| EffectError::EchosLengthOverflow)
            })
            .collect()
    }
}

fn process_channel(
    input: &[f32],
    output_frames: usize,
    gain_in: f64,
    gain_out: f64,
    taps: &[(usize, f64)],
    output: &mut Vec<f32>,
) {
    let mut delay_buffers = taps
        .iter()
        .map(|(delay, _decay)| vec![0.0_f64; *delay])
        .collect::<Vec<_>>();
    let mut counters = vec![0_usize; taps.len()];

    for frame in 0..output_frames {
        let input_sample = input.get(frame).copied().map_or(0.0, f64::from);
        let mut output_sample = input_sample * gain_in;

        for (index, (_delay, decay)) in taps.iter().enumerate() {
            output_sample += delay_buffers[index][counters[index]] * decay;
        }

        output.push(f64_to_f32_clamped(output_sample * gain_out));

        for index in (1..taps.len()).rev() {
            delay_buffers[index][counters[index]] =
                delay_buffers[index - 1][counters[index - 1]] + input_sample;
        }
        delay_buffers[0][counters[0]] = input_sample;

        for (counter, (delay, _decay)) in counters.iter_mut().zip(taps.iter()) {
            *counter = (*counter + 1) % *delay;
        }
    }
}

fn f64_to_f32_clamped(sample: f64) -> f32 {
    #[allow(
        clippy::cast_possible_truncation,
        reason = "echos output is explicitly clipped to normalized f32 full scale"
    )]
    {
        sample.clamp(-1.0, 1.0) as f32
    }
}

#[cfg(test)]
mod tests {
    use super::{Echos, EchosTap};
    use crate::EffectError;
    use auralis_core::{
        AudioBuffer, AudioSpec, ChannelCount, FrameCount, SampleFormat, SampleRate,
    };

    #[test]
    fn cascaded_echos_extends_by_sum_of_delays() {
        let audio = mono_audio_buffer(1_000, &[1.0, 0.0, 0.25]);
        let echos = Echos::new(
            0.5,
            1.0,
            [
                EchosTap::new(1.0, 0.25).unwrap(),
                EchosTap::new(2.0, 0.125).unwrap(),
            ],
        )
        .unwrap();

        let echoed = echos.process_buffer(&audio).unwrap();

        assert_eq!(echoed.frames(), FrameCount::new(6));
        assert_eq!(
            echoed.as_planar_f32(),
            &[0.5, 0.25, 0.25, 0.1875, 0.03125, 0.03125]
        );
    }

    #[test]
    fn one_tap_matches_parallel_echo_shape() {
        let audio = mono_audio_buffer(1_000, &[0.25, -0.5]);
        let echos = Echos::new(0.5, 0.5, [EchosTap::new(1.0, 0.5).unwrap()]).unwrap();

        let echoed = echos.process_buffer(&audio).unwrap();

        assert_eq!(echoed.frames(), FrameCount::new(3));
        assert_eq!(echoed.as_planar_f32(), &[0.0625, -0.0625, -0.125]);
    }

    #[test]
    fn rejects_invalid_config_and_sub_frame_delays() {
        assert_eq!(
            EchosTap::new(1.0, 1.5).unwrap_err(),
            EffectError::InvalidEchos
        );
        assert_eq!(
            Echos::new(f64::NAN, 1.0, [EchosTap::new(1.0, 0.5).unwrap()]).unwrap_err(),
            EffectError::InvalidEchos
        );
        assert_eq!(
            Echos::new(1.0, 1.0, []).unwrap_err(),
            EffectError::InvalidEchos
        );

        let audio = mono_audio_buffer(1_000, &[1.0]);
        let error = Echos::new(1.0, 1.0, [EchosTap::new(0.0, 0.5).unwrap()])
            .unwrap()
            .process_buffer(&audio)
            .unwrap_err();
        assert_eq!(error, EffectError::InvalidEchos);
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

use auralis_core::{AudioBuffer, SampleRate};

use crate::{EffectError, Fir, FirCoefficients, Result};

const DEFAULT_CUTOFF_HZ: f64 = 76.5;
const BLACKMAN_ALPHA: f64 = 0.16;
const MIN_TAPS: u32 = 3;
const MAX_TAPS: u32 = 1_073_741_823;

/// SoX-ng-style Hilbert transform FIR filter.
///
/// `hilbert [-n taps]` builds an odd-length, Blackman-windowed FIR Hilbert
/// transformer. When the tap count is omitted, it is derived from the input
/// sample rate using SoX-ng's cutoff heuristic of roughly 75 Hz. Processing is
/// deterministic, scalar, and length-preserving through the shared FIR executor.
///
/// # Examples
///
/// ```
/// use auralis_core::{AudioBuffer, AudioSpec, ChannelCount, FrameCount, SampleFormat, SampleRate};
/// use auralis_effects::Hilbert;
///
/// let spec = AudioSpec::new(
///     SampleRate::new(48_000)?,
///     ChannelCount::new(1)?,
///     SampleFormat::Float32,
/// );
/// let audio = AudioBuffer::from_planar_f32(spec, FrameCount::new(3), vec![0.25, 0.0, -0.25])?;
/// let shifted = Hilbert::with_taps(5)?.process_buffer(&audio)?;
///
/// assert_eq!(shifted.frames(), audio.frames());
/// # Ok::<(), Box<dyn std::error::Error>>(())
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Hilbert {
    taps: Option<u32>,
}

impl Hilbert {
    /// Creates a Hilbert filter with the sample-rate-derived default tap count.
    #[must_use]
    pub const fn default_taps() -> Self {
        Self { taps: None }
    }

    /// Creates a Hilbert filter with an explicit odd tap count.
    ///
    /// # Errors
    ///
    /// Returns [`EffectError::InvalidHilbert`] when `taps` is even or outside
    /// SoX-ng's accepted `3..=1073741823` range.
    pub fn with_taps(taps: u32) -> Result<Self> {
        validate_taps(taps)?;
        Ok(Self { taps: Some(taps) })
    }

    /// Returns the explicit tap count, if one was configured.
    #[must_use]
    pub const fn taps(self) -> Option<u32> {
        self.taps
    }

    /// Resolves the tap count for a concrete sample rate.
    ///
    /// # Errors
    ///
    /// Returns [`EffectError::InvalidHilbert`] if the derived value is outside
    /// the supported range.
    #[allow(
        clippy::cast_possible_truncation,
        clippy::cast_sign_loss,
        reason = "SoX-ng truncates the positive floating-point default tap heuristic to int"
    )]
    pub fn taps_for_sample_rate(self, sample_rate: SampleRate) -> Result<u32> {
        if let Some(taps) = self.taps {
            return Ok(taps);
        }

        let mut taps = (f64::from(sample_rate.as_u32()) / DEFAULT_CUTOFF_HZ + 2.0) as u32;
        if taps.is_multiple_of(2) {
            taps = taps.checked_add(1).ok_or(EffectError::InvalidHilbert)?;
        }
        validate_taps(taps)?;
        Ok(taps)
    }

    /// Generates the Blackman-windowed FIR coefficients for a sample rate.
    ///
    /// The sample rate is used only to derive the default tap count; explicit
    /// tap counts produce the same coefficients at every rate.
    ///
    /// # Errors
    ///
    /// Returns [`EffectError::InvalidHilbert`] when the resolved tap count is
    /// not representable by the current scalar implementation.
    #[allow(
        clippy::cast_precision_loss,
        reason = "Hilbert tap indices are bounded by SoX-ng's int range and require f64 DSP math"
    )]
    pub fn coefficients_for_sample_rate(self, sample_rate: SampleRate) -> Result<FirCoefficients> {
        let taps = self.taps_for_sample_rate(sample_rate)?;
        let taps_usize = usize::try_from(taps).map_err(|_| EffectError::InvalidHilbert)?;
        let center = i64::from(taps / 2);
        let last = f64::from(taps - 1);
        let mut coefficients = Vec::with_capacity(taps_usize);

        for index in 0..taps {
            let k = i64::from(index) - center;
            let value = if k % 2 == 0 {
                0.0
            } else {
                let pi_k = std::f64::consts::PI * k as f64;
                (1.0 - pi_k.cos()) / pi_k
            };
            let x = 2.0 * std::f64::consts::PI * f64::from(index) / last;
            let window = (1.0 - BLACKMAN_ALPHA) * 0.5 - 0.5 * x.cos()
                + BLACKMAN_ALPHA * 0.5 * (2.0 * x).cos();
            coefficients.push(value * window);
        }

        FirCoefficients::new(coefficients).map_err(|_| EffectError::InvalidHilbert)
    }

    /// Applies the Hilbert transformer to a decoded audio buffer.
    ///
    /// # Errors
    ///
    /// Returns [`EffectError::InvalidHilbert`] when coefficient generation
    /// fails, or FIR execution errors if the output shape cannot be represented.
    pub fn process_buffer(self, audio: &AudioBuffer) -> Result<AudioBuffer> {
        let coefficients = self.coefficients_for_sample_rate(audio.spec().sample_rate())?;
        if coefficients.len() == 5 {
            return process_five_tap_hilbert(audio, coefficients.as_slice()[3]);
        }

        Fir::from_coefficients(coefficients).process_buffer(audio)
    }
}

#[allow(
    clippy::cast_possible_truncation,
    reason = "Hilbert output is rounded to the public f32 sample format after f64 coefficient math"
)]
fn process_five_tap_hilbert(audio: &AudioBuffer, coefficient: f64) -> Result<AudioBuffer> {
    let frames =
        usize::try_from(audio.frames().as_u64()).map_err(|_| EffectError::FirLengthOverflow)?;
    let mut output = vec![0.0; audio.as_planar_f32().len()];

    for channel_index in 0..audio.channels().as_usize() {
        let channel = audio
            .channel(channel_index)
            .ok_or(EffectError::FirLengthOverflow)?;
        let start = channel_index
            .checked_mul(frames)
            .ok_or(EffectError::FirLengthOverflow)?;
        let out = output
            .get_mut(start..start + frames)
            .ok_or(EffectError::FirLengthOverflow)?;

        for frame in 0..frames {
            let previous = if frame == 0 {
                0.0
            } else {
                f64::from(channel[frame - 1])
            };
            let next = channel.get(frame + 1).copied().map_or(0.0, f64::from);
            out[frame] = ((previous - next) * coefficient) as f32;
        }
    }

    AudioBuffer::from_planar_f32(audio.spec(), audio.frames(), output)
        .map_err(|_| EffectError::FirLengthOverflow)
}

fn validate_taps(taps: u32) -> Result<()> {
    if (MIN_TAPS..=MAX_TAPS).contains(&taps) && !taps.is_multiple_of(2) {
        Ok(())
    } else {
        Err(EffectError::InvalidHilbert)
    }
}

#[cfg(test)]
mod tests {
    use super::Hilbert;
    use crate::EffectError;
    use auralis_core::{
        AudioBuffer, AudioSpec, ChannelCount, FrameCount, SampleFormat, SampleRate,
    };

    #[test]
    fn validates_sox_ng_tap_range_and_oddness() {
        assert_eq!(
            Hilbert::with_taps(1).unwrap_err(),
            EffectError::InvalidHilbert
        );
        assert_eq!(
            Hilbert::with_taps(4).unwrap_err(),
            EffectError::InvalidHilbert
        );
        assert!(Hilbert::with_taps(5).is_ok());
    }

    #[test]
    fn derives_default_taps_from_sample_rate() {
        assert_eq!(
            Hilbert::default_taps()
                .taps_for_sample_rate(SampleRate::new(48_000).unwrap())
                .unwrap(),
            629
        );
    }

    #[test]
    fn generates_odd_antisymmetric_blackman_windowed_coefficients() {
        let coefficients = Hilbert::with_taps(5)
            .unwrap()
            .coefficients_for_sample_rate(SampleRate::new(48_000).unwrap())
            .unwrap();

        assert!(coefficients.as_slice()[0].abs() < f64::EPSILON);
        assert!(coefficients.as_slice()[2].abs() < f64::EPSILON);
        assert!(coefficients.as_slice()[4].abs() < f64::EPSILON);
        assert!((coefficients.as_slice()[1] + 0.216_450_72).abs() < 1.0e-8);
        assert!((coefficients.as_slice()[3] - 0.216_450_72).abs() < 1.0e-8);
    }

    #[test]
    fn processes_audio_with_shared_fir_alignment() {
        let audio = mono_audio_buffer(vec![1.0, 0.0, 0.0, 0.0]);

        let shifted = Hilbert::with_taps(5)
            .unwrap()
            .process_buffer(&audio)
            .unwrap();

        assert_samples_close(shifted.as_planar_f32(), &[0.0, 0.216_450_72, 0.0, 0.0]);
    }

    fn mono_audio_buffer(samples: Vec<f32>) -> AudioBuffer {
        let spec = AudioSpec::new(
            SampleRate::new(48_000).unwrap(),
            ChannelCount::new(1).unwrap(),
            SampleFormat::Float32,
        );
        AudioBuffer::from_planar_f32(
            spec,
            FrameCount::new(u64::try_from(samples.len()).unwrap()),
            samples,
        )
        .unwrap()
    }

    fn assert_samples_close(actual: &[f32], expected: &[f32]) {
        assert_eq!(actual.len(), expected.len());
        for (index, (&actual, &expected)) in actual.iter().zip(expected).enumerate() {
            let difference = (actual - expected).abs();
            assert!(
                difference <= 1.0e-6,
                "sample {index} differed by {difference}: {actual} != {expected}"
            );
        }
    }
}

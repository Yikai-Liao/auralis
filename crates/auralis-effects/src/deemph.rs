use auralis_core::{AudioBuffer, SampleRate};

use crate::{Biquad, BiquadCoefficients, BiquadWidth, EffectError, Result};

/// SoX-ng-style CD/DAT de-emphasis filter.
///
/// `deemph` applies SoX-ng's fixed de-emphasis high-shelf presets. SoX-ng only
/// accepts 44.1 kHz audio-CD input and 48 kHz DAT input for this effect; other
/// sample rates return a design error.
///
/// Processing is deterministic and streaming-safe when callers preserve a
/// [`crate::BiquadState`] per channel across chunks.
///
/// # Examples
///
/// ```
/// use auralis_effects::Deemph;
///
/// let mut audio = auralis_core::AudioBuffer::from_planar_f32(
///     auralis_core::AudioSpec::new(
///         auralis_core::SampleRate::new(48_000)?,
///         auralis_core::ChannelCount::new(1)?,
///         auralis_core::SampleFormat::Float32,
///     ),
///     auralis_core::FrameCount::new(3),
///     vec![0.25, 0.0, -0.25],
/// )?;
/// Deemph::new().process_buffer(&mut audio)?;
///
/// assert!(audio.as_planar_f32().iter().all(|sample| sample.is_finite()));
/// # Ok::<(), Box<dyn std::error::Error>>(())
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Deemph;

impl Deemph {
    /// SoX-ng's 44.1 kHz de-emphasis high-shelf frequency.
    pub const AUDIO_CD_FREQUENCY_HZ: f64 = 5283.0;

    /// SoX-ng's 44.1 kHz de-emphasis shelf slope.
    pub const AUDIO_CD_WIDTH: BiquadWidth = BiquadWidth::slope(0.4845);

    /// SoX-ng's 44.1 kHz de-emphasis shelf gain.
    pub const AUDIO_CD_GAIN_DB: f64 = -9.477;

    /// SoX-ng's 48 kHz de-emphasis high-shelf frequency.
    pub const DAT_FREQUENCY_HZ: f64 = 5356.0;

    /// SoX-ng's 48 kHz de-emphasis shelf slope.
    pub const DAT_WIDTH: BiquadWidth = BiquadWidth::slope(0.479);

    /// SoX-ng's 48 kHz de-emphasis shelf gain.
    pub const DAT_GAIN_DB: f64 = -9.62;

    /// Creates a de-emphasis filter.
    #[must_use]
    pub const fn new() -> Self {
        Self
    }

    /// Returns normalized coefficients for SoX-ng's sample-rate-specific preset.
    ///
    /// # Errors
    ///
    /// Returns [`EffectError::InvalidBiquadDesign`] when the sample rate is not
    /// 44.1 kHz or 48 kHz.
    pub fn coefficients(self, sample_rate: SampleRate) -> Result<BiquadCoefficients> {
        let (frequency_hz, width, gain_db) = match sample_rate.as_u32() {
            44_100 => (
                Self::AUDIO_CD_FREQUENCY_HZ,
                Self::AUDIO_CD_WIDTH,
                Self::AUDIO_CD_GAIN_DB,
            ),
            48_000 => (Self::DAT_FREQUENCY_HZ, Self::DAT_WIDTH, Self::DAT_GAIN_DB),
            _ => return Err(EffectError::InvalidBiquadDesign),
        };

        Ok(BiquadCoefficients::rbj_high_shelf(
            f64::from(sample_rate.as_u32()),
            frequency_hz,
            width,
            gain_db,
        )?)
    }

    /// Applies the de-emphasis filter independently to every channel.
    ///
    /// # Errors
    ///
    /// Returns [`EffectError::InvalidBiquadDesign`] when the buffer sample rate
    /// is not 44.1 kHz or 48 kHz.
    pub fn process_buffer(self, audio: &mut AudioBuffer) -> Result<()> {
        let coefficients = self.coefficients(audio.spec().sample_rate())?;
        Biquad::new(coefficients).process_buffer(audio);
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::Deemph;
    use crate::{BiquadCoefficients, BiquadWidth, EffectError};
    use auralis_core::SampleRate;

    #[test]
    fn uses_sox_ng_audio_cd_and_dat_presets() {
        assert_eq!(
            Deemph::new()
                .coefficients(SampleRate::new(44_100).unwrap())
                .unwrap(),
            BiquadCoefficients::rbj_high_shelf(
                44_100.0,
                5283.0,
                BiquadWidth::slope(0.4845),
                -9.477,
            )
            .unwrap()
        );
        assert_eq!(
            Deemph::new()
                .coefficients(SampleRate::new(48_000).unwrap())
                .unwrap(),
            BiquadCoefficients::rbj_high_shelf(48_000.0, 5356.0, BiquadWidth::slope(0.479), -9.62,)
                .unwrap()
        );
    }

    #[test]
    fn rejects_sample_rates_without_sox_ng_presets() {
        assert_eq!(
            Deemph::new()
                .coefficients(SampleRate::new(32_000).unwrap())
                .unwrap_err(),
            EffectError::InvalidBiquadDesign
        );
    }
}

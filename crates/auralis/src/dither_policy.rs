use crate::AudioBuffer;
use auralis_effects::{DEFAULT_DITHER_SEED, Dither};
use thiserror::Error;

/// Output-boundary dither configuration for PCM16 writing.
///
/// Auralis never inserts dither implicitly. Callers opt in with
/// [`OutputDitherPolicy::Automatic`], which applies deterministic TPDF dither
/// after output rate, channel, and level policies and before PCM16 encoding.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct OutputDitherConfig {
    mode: OutputDitherMode,
    seed: u32,
}

impl OutputDitherConfig {
    /// Creates deterministic TPDF dither for PCM16 output.
    #[must_use]
    pub const fn new() -> Self {
        Self {
            mode: OutputDitherMode::Tpdf,
            seed: DEFAULT_DITHER_SEED,
        }
    }

    /// Creates deterministic sloped TPDF dither for PCM16 output.
    #[must_use]
    pub const fn sloped_tpdf() -> Self {
        Self {
            mode: OutputDitherMode::SlopedTpdf,
            seed: DEFAULT_DITHER_SEED,
        }
    }

    /// Returns this dither configuration with an explicit deterministic seed.
    #[must_use]
    pub const fn with_seed(mut self, seed: u32) -> Self {
        self.seed = seed;
        self
    }

    /// Returns the configured dither mode.
    #[must_use]
    pub const fn mode(self) -> OutputDitherMode {
        self.mode
    }

    /// Returns the deterministic PRNG seed.
    #[must_use]
    pub const fn seed(self) -> u32 {
        self.seed
    }

    fn into_dither(self) -> Dither {
        match self.mode {
            OutputDitherMode::Tpdf => Dither::new(),
            OutputDitherMode::SlopedTpdf => Dither::sloped_tpdf(),
        }
        .with_seed(self.seed)
    }
}

impl Default for OutputDitherConfig {
    fn default() -> Self {
        Self::new()
    }
}

/// Base output dither distribution.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum OutputDitherMode {
    /// Plain triangular probability density dither.
    Tpdf,
    /// SoX-ng `-S` sloped triangular dither without noise shaping.
    SlopedTpdf,
}

/// Explicit output-boundary dither policy.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum OutputDitherPolicy {
    /// Do not add output-boundary dither.
    Disabled,

    /// Apply deterministic PCM16 dither before encoding.
    Automatic(OutputDitherConfig),
}

impl OutputDitherPolicy {
    /// Returns the default policy: no output-boundary dither.
    #[must_use]
    pub const fn disabled() -> Self {
        Self::Disabled
    }

    /// Returns deterministic TPDF dither for PCM16 output.
    #[must_use]
    pub const fn automatic() -> Self {
        Self::Automatic(OutputDitherConfig::new())
    }

    /// Returns deterministic dither with the caller-provided configuration.
    #[must_use]
    pub const fn automatic_with_config(config: OutputDitherConfig) -> Self {
        Self::Automatic(config)
    }
}

/// Errors produced by explicit output dither insertion.
#[derive(Debug, Clone, PartialEq, Error)]
#[non_exhaustive]
pub enum OutputDitherError {
    /// A non-finite sample prevented deterministic dither insertion.
    #[error(
        "output dither encountered non-finite sample at channel {channel_index}, frame {frame_index}"
    )]
    NonFiniteSample {
        /// Zero-based channel index containing the non-finite sample.
        channel_index: usize,

        /// Zero-based frame index containing the non-finite sample.
        frame_index: u64,
    },
}

/// Applies deterministic PCM16 output dither to a decoded planar buffer.
///
/// This helper clones the input, validates that every sample is finite, applies
/// the configured dither, and returns the adjusted buffer. It is the library
/// counterpart to `auralis run --dither`.
///
/// # Errors
///
/// Returns [`OutputDitherError::NonFiniteSample`] when a sample is NaN or
/// infinite.
pub fn dither_audio_for_pcm16(
    audio: &AudioBuffer,
    config: OutputDitherConfig,
) -> std::result::Result<AudioBuffer, OutputDitherError> {
    validate_finite_samples(audio)?;
    let mut output = audio.clone();
    config.into_dither().process_buffer(&mut output);
    Ok(output)
}

pub(crate) fn apply_output_dither_policy(
    audio: AudioBuffer,
    policy: OutputDitherPolicy,
) -> std::result::Result<AudioBuffer, OutputDitherError> {
    match policy {
        OutputDitherPolicy::Disabled => Ok(audio),
        OutputDitherPolicy::Automatic(config) => dither_audio_for_pcm16(&audio, config),
    }
}

fn validate_finite_samples(audio: &AudioBuffer) -> std::result::Result<(), OutputDitherError> {
    for channel_index in 0..audio.channels().as_usize() {
        let channel = audio
            .channel(channel_index)
            .expect("validated AudioBuffer must expose each declared channel");
        for (frame_index, &sample) in channel.iter().enumerate() {
            if !sample.is_finite() {
                return Err(OutputDitherError::NonFiniteSample {
                    channel_index,
                    frame_index: u64::try_from(frame_index)
                        .expect("frame index from a slice must fit in u64"),
                });
            }
        }
    }

    Ok(())
}

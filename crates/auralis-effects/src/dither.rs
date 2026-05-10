//! Effect-boundary wrappers for deterministic dither primitives.

use auralis_core::AudioBuffer;
use auralis_dsp::{Dither as DspDither, DitherState as DspDitherState};

use crate::{EffectError, Result};

pub use auralis_dsp::{DEFAULT_DITHER_SEED, DitherMode, DitherNoiseShape};

/// SoX-ng-style deterministic dither configuration.
///
/// Command parsing and [`AudioBuffer`] processing live in `auralis-effects`,
/// while reusable sample-slice processing is owned by `auralis-dsp`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Dither {
    inner: DspDither,
}

impl Dither {
    /// Creates default TPDF dither for 16-bit output with a deterministic seed.
    #[must_use]
    pub const fn new() -> Self {
        Self {
            inner: DspDither::new(),
        }
    }

    /// Creates sloped TPDF dither for 16-bit output with a deterministic seed.
    #[must_use]
    pub const fn sloped_tpdf() -> Self {
        Self {
            inner: DspDither::sloped_tpdf(),
        }
    }

    /// Creates default Shibata noise-shaped TPDF dither for 16-bit output.
    #[must_use]
    pub const fn shibata() -> Self {
        Self {
            inner: DspDither::shibata(),
        }
    }

    /// Returns this dither configuration with an explicit target precision.
    ///
    /// # Errors
    ///
    /// Returns [`EffectError::InvalidDither`] when `precision_bits` is outside
    /// the implemented SoX-ng-compatible range.
    pub const fn with_precision(self, precision_bits: u8) -> Result<Self> {
        match self.inner.with_precision(precision_bits) {
            Ok(inner) => Ok(Self { inner }),
            Err(_) => Err(EffectError::InvalidDither),
        }
    }

    /// Returns this dither configuration with an explicit deterministic seed.
    #[must_use]
    pub const fn with_seed(self, seed: u32) -> Self {
        Self {
            inner: self.inner.with_seed(seed),
        }
    }

    /// Returns this dither configuration with explicit noise shaping.
    #[must_use]
    pub const fn with_noise_shape(self, noise_shape: DitherNoiseShape) -> Self {
        Self {
            inner: self.inner.with_noise_shape(noise_shape),
        }
    }

    /// Returns the configured dither mode.
    #[must_use]
    pub const fn mode(self) -> DitherMode {
        self.inner.mode()
    }

    /// Returns the configured noise-shaping filter, if any.
    #[must_use]
    pub const fn noise_shape(self) -> Option<DitherNoiseShape> {
        self.inner.noise_shape()
    }

    /// Returns the target precision in bits.
    #[must_use]
    pub const fn precision_bits(self) -> u8 {
        self.inner.precision_bits()
    }

    /// Returns the deterministic PRNG seed.
    #[must_use]
    pub const fn seed(self) -> u32 {
        self.inner.seed()
    }

    /// Applies dither to a decoded planar audio buffer in place.
    pub fn process_buffer(self, audio: &mut AudioBuffer) {
        self.process_samples(audio.as_planar_f32_mut());
    }

    /// Applies dither to a planar sample slice in place.
    pub fn process_samples(self, samples: &mut [f32]) {
        self.inner.process_samples(samples);
    }

    pub(crate) const fn into_dsp(self) -> DspDither {
        self.inner
    }
}

impl Default for Dither {
    fn default() -> Self {
        Self::new()
    }
}

impl From<DspDither> for Dither {
    fn from(inner: DspDither) -> Self {
        Self { inner }
    }
}

impl From<Dither> for DspDither {
    fn from(dither: Dither) -> Self {
        dither.into_dsp()
    }
}

/// Stateful deterministic dither processor for chunked effect callers.
#[derive(Debug, Clone)]
pub struct DitherState {
    inner: DspDitherState,
}

impl DitherState {
    /// Creates a stateful dither processor from a validated configuration.
    #[must_use]
    pub const fn new(dither: Dither) -> Self {
        Self {
            inner: DspDitherState::new(dither.into_dsp()),
        }
    }

    /// Applies dither to the next chunk of samples.
    pub fn process_samples(&mut self, samples: &mut [f32]) {
        self.inner.process_samples(samples);
    }
}

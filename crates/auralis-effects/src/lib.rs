//! Typed audio effects for Auralis.
//!
//! Effects in this crate own validated configuration and delegate numerical
//! work to deterministic DSP kernels. They operate on Auralis' planar `f32`
//! audio buffers and are chunk-invariant unless their documentation says
//! otherwise. The crate also exposes a typed command parser for the currently
//! implemented SoX-ng-style effect command subset; successful parses return
//! [`EffectCommand`] variants that wrap the same typed processors used by the
//! direct API.
//!
//! # Examples
//!
//! ```
//! use auralis_core::Decibels;
//! use auralis_effects::{DcShift, Gain};
//!
//! let gain = Gain::new(Decibels::new(-3.0)?);
//! let mut samples = [0.25, -0.5, 1.0];
//! gain.process_samples(&mut samples);
//! DcShift::new(-0.25)?.process_samples(&mut samples);
//!
//! assert!(samples[2] > 0.45 && samples[2] < 0.46);
//! # Ok::<(), Box<dyn std::error::Error>>(())
//! ```

mod chain;
mod chain_gain;
mod command;
mod command_contrast;
mod command_fade;
mod command_gain;
mod command_norm;
mod command_pad;
mod command_trim;
mod command_vol;
mod contrast;
mod dcshift;
mod effects_file;
mod error;
mod fade;
mod gain;
mod norm;
mod pad;
mod registry;
mod reverse;
#[cfg(test)]
mod test_support;
mod trim;
mod vol;

pub use chain::{
    ChainParseResult, ChainResult, EffectChain, EffectChainBoundary, EffectChainError,
    EffectChainParseError, parse_effect_chain,
};
pub use command::{CommandResult, EffectCommand, EffectCommandParseError, parse_effect_command};
pub use contrast::Contrast;
pub use dcshift::DcShift;
pub use effects_file::{
    EffectsFileParseError, EffectsFileParseResult, EffectsFileReadError, EffectsFileReadResult,
    parse_effects_file, parse_effects_file_str,
};
pub use error::{EffectError, Result};
pub use fade::{Fade, FadeCurve};
pub use gain::{Gain, GainChannelMode, GainHeadroom};
pub use norm::Norm;
pub use pad::{Pad, PositionedPad};
pub use registry::{
    EffectDescriptor, EffectKind, EffectNameError, EffectRegistry, KNOWN_SOX_NG_EFFECTS,
    SUPPORTED_EFFECTS, resolve_effect_name,
};
pub use reverse::Reverse;
pub use trim::{Trim, TrimPosition};
pub use vol::{Vol, VolGainType};

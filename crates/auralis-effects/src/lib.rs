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

mod allpass;
mod band;
mod bandpass;
mod bandreject;
mod bass;
mod biquad;
mod biquad_design;
mod centercut;
mod chain;
mod chain_dispatch;
mod chain_gain;
mod channels;
mod command;
mod command_allpass;
mod command_band;
mod command_bandpass;
mod command_bandreject;
mod command_bass;
mod command_biquad;
mod command_centercut;
mod command_channels;
mod command_contrast;
mod command_dcshift;
mod command_equalizer;
mod command_fade;
mod command_filter;
mod command_gain;
mod command_norm;
mod command_oops;
mod command_overdrive;
mod command_pad;
mod command_remix;
mod command_repeat;
mod command_reverse;
mod command_saturation;
mod command_softvol;
mod command_swap;
mod command_treble;
mod command_tremolo;
mod command_trim;
mod command_vol;
mod contrast;
mod dcshift;
mod effects_file;
mod equalizer;
mod error;
mod fade;
mod gain;
mod norm;
mod oops;
mod overdrive;
mod pad;
mod registry;
mod remix;
mod repeat;
mod reverse;
mod saturation;
mod softvol;
mod swap;
#[cfg(test)]
mod test_support;
mod treble;
mod tremolo;
mod trim;
mod vol;

pub use allpass::{AllPass, AllPassMode};
pub use band::{Band, BandMode};
pub use bandpass::{BandPass, BandPassMode};
pub use bandreject::BandReject;
pub use bass::Bass;
pub use biquad::{Biquad, BiquadCoefficients, BiquadState};
pub use biquad_design::BiquadWidth;
pub use centercut::Centercut;
pub use chain::{
    ChainParseResult, ChainResult, EffectChain, EffectChainBoundary, EffectChainError,
    EffectChainParseError, parse_effect_chain,
};
pub use channels::Channels;
pub use command::{CommandResult, EffectCommand, EffectCommandParseError, parse_effect_command};
pub use contrast::Contrast;
pub use dcshift::DcShift;
pub use effects_file::{
    EffectsFileParseError, EffectsFileParseResult, EffectsFileReadError, EffectsFileReadResult,
    parse_effects_file, parse_effects_file_str,
};
pub use equalizer::Equalizer;
pub use error::{EffectError, Result};
pub use fade::{Fade, FadeCurve};
pub use gain::{Gain, GainChannelMode, GainHeadroom};
pub use norm::Norm;
pub use oops::Oops;
pub use overdrive::{Overdrive, OverdriveState};
pub use pad::{Pad, PositionedPad};
pub use registry::{
    EffectDescriptor, EffectKind, EffectNameError, EffectRegistry, KNOWN_SOX_NG_EFFECTS,
    SUPPORTED_EFFECTS, resolve_effect_name,
};
pub use remix::{Remix, RemixGain, RemixLevelMode, RemixOutputSpec, RemixSource};
pub use repeat::Repeat;
pub use reverse::Reverse;
pub use saturation::{Saturation, SaturationType};
pub use softvol::SoftVol;
pub use swap::Swap;
pub use treble::Treble;
pub use tremolo::Tremolo;
pub use trim::{Trim, TrimPosition};
pub use vol::{Vol, VolGainType};

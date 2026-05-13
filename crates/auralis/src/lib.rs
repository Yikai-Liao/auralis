//! High-level Auralis library API.
//!
//! This crate provides a small facade over the lower-level codec and effect
//! crates. It is intended for applications that want to open supported audio
//! files, build a typed processing chain, and write the result without wiring
//! the codec and effect crates manually.
//!
//! # Examples
//!
//! ```no_run
//! use auralis::AudioFile;
//!
//! AudioFile::open_wav("input.wav")?
//!     .into_pipeline()
//!     .gain_db(-3.0)
//!     .dc_shift(0.125)
//!     .fade_frames(1_000, 2_000)
//!     .reverse()
//!     .pad_frames(0, 48_000)
//!     .write_wav("output.wav")?;
//!
//! # Ok::<(), auralis::Error>(())
//! ```

mod audio_file;
mod channel_policy;
mod combine;
mod dither_policy;
mod errors;
mod level_policy;
mod pipeline;
mod rate_policy;

pub use auralis_au::AuError;
pub use auralis_codec::{
    AiffContainer, AiffEncodeOptions, AiffSampleFormat, AuEncodeOptions, AuSampleFormat,
    CodecCapabilities, CodecError, CodecKind, EncodeSummary, FlacEncodeOptions, OutputFormat,
    RawPcmBitOrder, RawPcmByteOrder, RawPcmEncodeOptions, RawPcmNibbleOrder, RawPcmSampleFormat,
    WavEncodeOptions, WavSampleFormat,
};
pub use auralis_core::{
    AudioBuffer, AudioSpec, ChannelCount, Decibels, FrameCount, SampleFormat, SampleRate,
    TimeSeconds,
};
pub use auralis_effects::{
    EffectChain, EffectChainBoundary, EffectChainError, EffectChainParseError, EffectCommand,
    EffectsFileParseError, EffectsFileReadError, parse_effect_chain, parse_effects_file,
    parse_effects_file_str,
};
pub use auralis_flac::FlacError;
pub use auralis_simd::BackendKind;

pub use audio_file::AudioFile;
pub use channel_policy::{
    ChannelConversionError, ChannelConversionPolicy, convert_audio_channels,
    convert_audio_channels_with_backend,
};
pub use combine::{
    CombineMethod, InputCombineError, concatenate_audio_buffers, merge_audio_buffers,
    mix_audio_buffers, mix_audio_buffers_with_backend, mix_power_audio_buffers,
    mix_power_audio_buffers_with_backend, multiply_audio_buffers,
    multiply_audio_buffers_with_backend, sequence_audio_buffers,
};
pub use dither_policy::{
    OutputDitherConfig, OutputDitherError, OutputDitherMode, OutputDitherPolicy,
    dither_audio_for_pcm16,
};
pub use errors::Error;
pub use level_policy::{
    OutputLevelError, OutputLevelPolicy, guard_audio_level, guard_audio_level_with_backend,
    normalize_audio_level, normalize_audio_level_with_backend,
};
pub use pipeline::Pipeline;
pub use rate_policy::{
    SampleRateConversionError, SampleRateConversionPolicy, convert_audio_sample_rate,
};

/// Crate-local result type using [`Error`].
pub type Result<T> = std::result::Result<T, Error>;

//! WAV codec support for Auralis.
//!
//! This crate currently implements deterministic PCM8, PCM16, PCM24, and PCM32 WAV
//! decoding and encoding between RIFF/WAVE files and Auralis' internal planar `f32`
//! [`auralis_core::AudioBuffer`].
//!
//! # Examples
//!
//! ```
//! use std::io::Cursor;
//!
//! use auralis_wav::decode_pcm16;
//!
//! let mut wav_bytes = Vec::new();
//! wav_bytes.extend_from_slice(b"RIFF");
//! wav_bytes.extend_from_slice(&38_u32.to_le_bytes());
//! wav_bytes.extend_from_slice(b"WAVEfmt ");
//! wav_bytes.extend_from_slice(&16_u32.to_le_bytes());
//! wav_bytes.extend_from_slice(&1_u16.to_le_bytes());
//! wav_bytes.extend_from_slice(&1_u16.to_le_bytes());
//! wav_bytes.extend_from_slice(&48_000_u32.to_le_bytes());
//! wav_bytes.extend_from_slice(&96_000_u32.to_le_bytes());
//! wav_bytes.extend_from_slice(&2_u16.to_le_bytes());
//! wav_bytes.extend_from_slice(&16_u16.to_le_bytes());
//! wav_bytes.extend_from_slice(b"data");
//! wav_bytes.extend_from_slice(&2_u32.to_le_bytes());
//! wav_bytes.extend_from_slice(&0_i16.to_le_bytes());
//!
//! let audio = decode_pcm16(Cursor::new(wav_bytes))?;
//! assert_eq!(audio.frames().as_u64(), 1);
//! assert_eq!(audio.sample(0, 0), Some(0.0));
//! # Ok::<(), auralis_wav::WavError>(())
//! ```

mod error;
mod format;
mod reader;
mod sample_conversion;
mod writer;

pub use error::{Result, WavError};
pub use format::WavSampleEncoding;
pub use reader::{
    AnyPcmWavReader, Pcm16WavReader, decode_pcm8, decode_pcm8_path, decode_pcm8_path_with_backend,
    decode_pcm8_with_backend, decode_pcm16, decode_pcm16_path, decode_pcm16_path_with_backend,
    decode_pcm16_with_backend, decode_pcm24, decode_pcm24_path, decode_pcm24_path_with_backend,
    decode_pcm24_with_backend, decode_pcm32, decode_pcm32_path, decode_pcm32_path_with_backend,
    decode_pcm32_with_backend, decode_wav, decode_wav_path, decode_wav_path_with_backend,
    decode_wav_with_backend,
};
pub use writer::{
    Pcm8WavEncoder, Pcm8WavWriter, Pcm16WavEncoder, Pcm16WavWriter, Pcm24WavEncoder,
    Pcm24WavWriter, Pcm32WavEncoder, Pcm32WavWriter, encode_pcm8, encode_pcm8_path,
    encode_pcm8_path_with_backend, encode_pcm8_with_backend, encode_pcm16, encode_pcm16_path,
    encode_pcm16_path_with_backend, encode_pcm16_with_backend, encode_pcm24, encode_pcm24_path,
    encode_pcm24_path_with_backend, encode_pcm24_with_backend, encode_pcm32, encode_pcm32_path,
    encode_pcm32_path_with_backend, encode_pcm32_with_backend,
};

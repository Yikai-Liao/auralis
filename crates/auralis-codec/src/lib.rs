//! Codec boundaries for Auralis.
//!
//! This crate defines the stable traits and error vocabulary used by concrete
//! codecs. The initial project scope is WAV only, but this crate deliberately
//! contains no decoding or encoding implementation. Concrete crates adapt file
//! formats into [`auralis_core::AudioBuffer`] and report their capabilities
//! through [`CodecCapabilities`].
//!
//! # Examples
//!
//! ```
//! use auralis_codec::{AudioReader, CodecError, CodecKind, UnsupportedReader};
//!
//! let mut reader = UnsupportedReader::new(CodecKind::Flac);
//! let error = reader.read_audio().unwrap_err();
//!
//! assert!(matches!(error, CodecError::UnsupportedFormat(_)));
//! ```

use std::fmt;

use auralis_core::AudioBuffer;
use thiserror::Error;

/// Crate-local result type using [`CodecError`].
pub type Result<T> = std::result::Result<T, CodecError>;

/// Errors produced by codec boundary implementations.
#[derive(Debug, Clone, PartialEq, Error)]
#[non_exhaustive]
pub enum CodecError {
    /// The requested container or sample format is not supported by the active
    /// codec set.
    #[error("{0}")]
    UnsupportedFormat(UnsupportedFormat),

    /// A concrete decoder failed to read the requested stream.
    #[error("{kind} decode failed: {message}")]
    DecodeFailed {
        /// Codec kind that failed while decoding.
        kind: CodecKind,
        /// Human-readable failure detail from the concrete codec.
        message: String,
    },
}

/// Description of an unsupported codec request.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct UnsupportedFormat {
    kind: CodecKind,
}

impl UnsupportedFormat {
    /// Creates an unsupported-format error payload for `kind`.
    #[must_use]
    pub const fn new(kind: CodecKind) -> Self {
        Self { kind }
    }

    /// Returns the codec kind that could not be handled.
    #[must_use]
    pub const fn kind(self) -> CodecKind {
        self.kind
    }
}

impl fmt::Display for UnsupportedFormat {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "{} codec is not supported", self.kind)
    }
}

/// Container or codec family understood by the Auralis boundary layer.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum CodecKind {
    /// RIFF/WAVE audio.
    Wav,

    /// FLAC audio, represented for explicit unsupported-format reporting.
    Flac,

    /// MP3 audio, represented for explicit unsupported-format reporting.
    Mp3,
}

impl fmt::Display for CodecKind {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Wav => formatter.write_str("wav"),
            Self::Flac => formatter.write_str("flac"),
            Self::Mp3 => formatter.write_str("mp3"),
        }
    }
}

/// Declared read/write support for a codec kind in the active build.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CodecCapabilities {
    kind: CodecKind,
    can_read: bool,
    can_write: bool,
}

impl CodecCapabilities {
    /// Returns the active-build capabilities for `kind`.
    ///
    /// WAV is reported as readable and writable only when the `auralis-wav`
    /// feature is enabled. Placeholder formats remain unsupported so callers
    /// can surface typed errors before a concrete codec exists.
    #[must_use]
    pub const fn for_kind(kind: CodecKind) -> Self {
        let supported = matches!(kind, CodecKind::Wav) && cfg!(feature = "auralis-wav");

        Self {
            kind,
            can_read: supported,
            can_write: supported,
        }
    }

    /// Returns the described codec kind.
    #[must_use]
    pub const fn kind(self) -> CodecKind {
        self.kind
    }

    /// Returns whether this build can decode the codec kind.
    #[must_use]
    pub const fn can_read(self) -> bool {
        self.can_read
    }

    /// Returns whether this build can encode the codec kind.
    #[must_use]
    pub const fn can_write(self) -> bool {
        self.can_write
    }
}

/// Boundary implemented by audio decoders.
///
/// Implementations are deterministic for a fixed input byte stream and active
/// codec build. They return planar `f32` buffers and keep container-specific
/// details behind codec-specific types.
pub trait AudioReader {
    /// Returns the codec kind backing this reader.
    fn codec_kind(&self) -> CodecKind;

    /// Reads the entire stream into Auralis' internal planar `f32` buffer.
    ///
    /// # Errors
    ///
    /// Returns [`CodecError::UnsupportedFormat`] when the reader is a
    /// placeholder for a codec kind that is not available in the active build.
    /// Concrete readers may return additional variants in future releases.
    fn read_audio(&mut self) -> Result<AudioBuffer>;
}

/// Boundary implemented by audio encoders.
///
/// Implementations are deterministic for a fixed input buffer, parameters, and
/// active codec build. They accept Auralis' internal planar `f32` buffers and
/// keep container serialization details behind codec-specific types.
pub trait AudioWriter {
    /// Returns the codec kind backing this writer.
    fn codec_kind(&self) -> CodecKind;

    /// Writes an entire internal planar `f32` buffer.
    ///
    /// # Errors
    ///
    /// Returns [`CodecError::UnsupportedFormat`] when the writer is a
    /// placeholder for a codec kind that is not available in the active build.
    /// Concrete writers may return additional variants in future releases.
    fn write_audio(&mut self, audio: &AudioBuffer) -> Result<()>;
}

/// Reader implementation that always reports an unsupported codec kind.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct UnsupportedReader {
    kind: CodecKind,
}

impl UnsupportedReader {
    /// Creates a reader placeholder for `kind`.
    #[must_use]
    pub const fn new(kind: CodecKind) -> Self {
        Self { kind }
    }
}

impl AudioReader for UnsupportedReader {
    fn codec_kind(&self) -> CodecKind {
        self.kind
    }

    fn read_audio(&mut self) -> Result<AudioBuffer> {
        Err(CodecError::UnsupportedFormat(UnsupportedFormat::new(
            self.kind,
        )))
    }
}

/// Writer implementation that always reports an unsupported codec kind.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct UnsupportedWriter {
    kind: CodecKind,
}

impl UnsupportedWriter {
    /// Creates a writer placeholder for `kind`.
    #[must_use]
    pub const fn new(kind: CodecKind) -> Self {
        Self { kind }
    }
}

impl AudioWriter for UnsupportedWriter {
    fn codec_kind(&self) -> CodecKind {
        self.kind
    }

    fn write_audio(&mut self, _audio: &AudioBuffer) -> Result<()> {
        Err(CodecError::UnsupportedFormat(UnsupportedFormat::new(
            self.kind,
        )))
    }
}

#[cfg(test)]
mod tests {
    use auralis_core::{
        AudioBuffer, AudioSpec, ChannelCount, FrameCount, SampleFormat, SampleRate,
    };

    use super::{
        AudioReader, AudioWriter, CodecCapabilities, CodecError, CodecKind, UnsupportedFormat,
        UnsupportedReader, UnsupportedWriter,
    };

    fn mono_buffer() -> AudioBuffer {
        let spec = AudioSpec::new(
            SampleRate::new(48_000).unwrap(),
            ChannelCount::new(1).unwrap(),
            SampleFormat::Float32,
        );

        AudioBuffer::zeroed(spec, FrameCount::new(1)).unwrap()
    }

    #[test]
    fn unsupported_reader_returns_typed_error() {
        let mut reader = UnsupportedReader::new(CodecKind::Flac);
        let error = reader.read_audio().unwrap_err();

        assert_eq!(reader.codec_kind(), CodecKind::Flac);
        assert_eq!(
            error,
            CodecError::UnsupportedFormat(UnsupportedFormat::new(CodecKind::Flac))
        );
        assert_eq!(error.to_string(), "flac codec is not supported");
    }

    #[test]
    fn unsupported_writer_returns_typed_error() {
        let mut writer = UnsupportedWriter::new(CodecKind::Mp3);
        let audio = mono_buffer();
        let error = writer.write_audio(&audio).unwrap_err();

        assert_eq!(writer.codec_kind(), CodecKind::Mp3);
        assert_eq!(
            error,
            CodecError::UnsupportedFormat(UnsupportedFormat::new(CodecKind::Mp3))
        );
    }

    #[test]
    fn placeholder_formats_are_not_supported() {
        for kind in [CodecKind::Flac, CodecKind::Mp3] {
            let capabilities = CodecCapabilities::for_kind(kind);

            assert_eq!(capabilities.kind(), kind);
            assert!(!capabilities.can_read());
            assert!(!capabilities.can_write());
        }
    }

    #[test]
    fn wav_capability_tracks_feature_flag() {
        let capabilities = CodecCapabilities::for_kind(CodecKind::Wav);

        assert_eq!(capabilities.kind(), CodecKind::Wav);
        assert_eq!(capabilities.can_read(), cfg!(feature = "auralis-wav"));
        assert_eq!(capabilities.can_write(), cfg!(feature = "auralis-wav"));
    }
}

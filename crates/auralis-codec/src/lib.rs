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

use std::{
    fmt,
    io::{Seek, Write},
};

use auralis_core::{AudioBuffer, AudioSpec, FrameCount};
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

    /// A concrete encoder failed to write the requested stream.
    #[error("{kind} encode failed: {message}")]
    EncodeFailed {
        /// Codec kind that failed while encoding.
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

    /// Headerless raw PCM audio.
    RawPcm,

    /// AIFF or AIFC audio.
    Aiff,

    /// FLAC audio, represented for explicit unsupported-format reporting.
    Flac,

    /// MP3 audio, represented for explicit unsupported-format reporting.
    Mp3,
}

impl fmt::Display for CodecKind {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Wav => formatter.write_str("wav"),
            Self::RawPcm => formatter.write_str("raw-pcm"),
            Self::Aiff => formatter.write_str("aiff"),
            Self::Flac => formatter.write_str("flac"),
            Self::Mp3 => formatter.write_str("mp3"),
        }
    }
}

/// Sample format for WAV export.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum WavSampleFormat {
    /// Unsigned 8-bit PCM samples.
    Pcm8,

    /// Signed 16-bit PCM samples.
    #[default]
    Pcm16,

    /// Signed 24-bit PCM samples stored in 32-bit containers.
    Pcm24,

    /// Signed 32-bit PCM samples.
    Pcm32,

    /// 32-bit IEEE floating-point samples.
    Float32,

    /// 64-bit IEEE floating-point samples.
    Float64,

    /// 8-bit G.711 u-law companded samples.
    ULaw,

    /// 8-bit G.711 A-law companded samples.
    ALaw,
}

/// Container byte order for WAV export.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum WavContainer {
    /// Little-endian RIFF/WAVE.
    #[default]
    Riff,

    /// Big-endian RIFX/WAVE.
    Rifx,
}

/// Auralis-owned options for WAV export.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct WavEncodeOptions {
    sample_format: WavSampleFormat,
    container: WavContainer,
}

impl WavEncodeOptions {
    /// Creates WAV encode options for `sample_format`.
    #[must_use]
    pub const fn new(sample_format: WavSampleFormat) -> Self {
        Self {
            sample_format,
            container: WavContainer::Riff,
        }
    }

    /// Returns options with the requested container byte order.
    #[must_use]
    pub const fn with_container(mut self, container: WavContainer) -> Self {
        self.container = container;
        self
    }

    /// Creates WAV encode options for unsigned 8-bit PCM.
    #[must_use]
    pub const fn pcm8() -> Self {
        Self::new(WavSampleFormat::Pcm8)
    }

    /// Creates WAV encode options for signed 16-bit PCM.
    #[must_use]
    pub const fn pcm16() -> Self {
        Self::new(WavSampleFormat::Pcm16)
    }

    /// Creates WAV encode options for signed 24-bit PCM.
    #[must_use]
    pub const fn pcm24() -> Self {
        Self::new(WavSampleFormat::Pcm24)
    }

    /// Creates WAV encode options for signed 32-bit PCM.
    #[must_use]
    pub const fn pcm32() -> Self {
        Self::new(WavSampleFormat::Pcm32)
    }

    /// Creates WAV encode options for 32-bit IEEE floating-point samples.
    #[must_use]
    pub const fn float32() -> Self {
        Self::new(WavSampleFormat::Float32)
    }

    /// Creates WAV encode options for 64-bit IEEE floating-point samples.
    #[must_use]
    pub const fn float64() -> Self {
        Self::new(WavSampleFormat::Float64)
    }

    /// Creates WAV encode options for 8-bit G.711 u-law samples.
    #[must_use]
    pub const fn ulaw() -> Self {
        Self::new(WavSampleFormat::ULaw)
    }

    /// Creates WAV encode options for 8-bit G.711 A-law samples.
    #[must_use]
    pub const fn alaw() -> Self {
        Self::new(WavSampleFormat::ALaw)
    }

    /// Returns the configured WAV sample format.
    #[must_use]
    pub const fn sample_format(self) -> WavSampleFormat {
        self.sample_format
    }

    /// Returns the configured WAV container byte order.
    #[must_use]
    pub const fn container(self) -> WavContainer {
        self.container
    }
}

impl Default for WavEncodeOptions {
    fn default() -> Self {
        Self::pcm16()
    }
}

/// Sample format for headerless raw PCM export.
///
/// Multi-byte integer and floating-point samples are little-endian until
/// explicit raw endian options land in the format roadmap.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum RawPcmSampleFormat {
    /// Signed 8-bit PCM samples.
    Signed8,

    /// Unsigned 8-bit PCM samples.
    Unsigned8,

    /// Signed 16-bit PCM samples.
    #[default]
    Signed16,

    /// Unsigned 16-bit PCM samples.
    Unsigned16,

    /// Signed 24-bit PCM samples.
    Signed24,

    /// Unsigned 24-bit PCM samples.
    Unsigned24,

    /// Signed 32-bit PCM samples.
    Signed32,

    /// Unsigned 32-bit PCM samples.
    Unsigned32,

    /// IEEE 754 little-endian 32-bit floating-point samples.
    Float32,

    /// IEEE 754 little-endian 64-bit floating-point samples.
    Float64,
}

/// Auralis-owned options for raw PCM export.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RawPcmEncodeOptions {
    sample_format: RawPcmSampleFormat,
}

impl RawPcmEncodeOptions {
    /// Creates raw PCM encode options for `sample_format`.
    #[must_use]
    pub const fn new(sample_format: RawPcmSampleFormat) -> Self {
        Self { sample_format }
    }

    /// Creates options for signed 8-bit raw PCM.
    #[must_use]
    pub const fn signed8() -> Self {
        Self::new(RawPcmSampleFormat::Signed8)
    }

    /// Creates options for unsigned 8-bit raw PCM.
    #[must_use]
    pub const fn unsigned8() -> Self {
        Self::new(RawPcmSampleFormat::Unsigned8)
    }

    /// Creates options for signed little-endian 16-bit raw PCM.
    #[must_use]
    pub const fn signed16() -> Self {
        Self::new(RawPcmSampleFormat::Signed16)
    }

    /// Creates options for unsigned little-endian 16-bit raw PCM.
    #[must_use]
    pub const fn unsigned16() -> Self {
        Self::new(RawPcmSampleFormat::Unsigned16)
    }

    /// Creates options for signed little-endian 24-bit raw PCM.
    #[must_use]
    pub const fn signed24() -> Self {
        Self::new(RawPcmSampleFormat::Signed24)
    }

    /// Creates options for unsigned little-endian 24-bit raw PCM.
    #[must_use]
    pub const fn unsigned24() -> Self {
        Self::new(RawPcmSampleFormat::Unsigned24)
    }

    /// Creates options for signed little-endian 32-bit raw PCM.
    #[must_use]
    pub const fn signed32() -> Self {
        Self::new(RawPcmSampleFormat::Signed32)
    }

    /// Creates options for unsigned little-endian 32-bit raw PCM.
    #[must_use]
    pub const fn unsigned32() -> Self {
        Self::new(RawPcmSampleFormat::Unsigned32)
    }

    /// Creates options for IEEE 754 little-endian 32-bit floating-point raw PCM.
    #[must_use]
    pub const fn float32() -> Self {
        Self::new(RawPcmSampleFormat::Float32)
    }

    /// Creates options for IEEE 754 little-endian 64-bit floating-point raw PCM.
    #[must_use]
    pub const fn float64() -> Self {
        Self::new(RawPcmSampleFormat::Float64)
    }

    /// Returns the configured raw PCM sample format.
    #[must_use]
    pub const fn sample_format(self) -> RawPcmSampleFormat {
        self.sample_format
    }
}

impl Default for RawPcmEncodeOptions {
    fn default() -> Self {
        Self::signed16()
    }
}

/// Auralis-owned options for AIFF/AIFC export.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub struct AiffEncodeOptions;

/// Auralis-owned options for FLAC export.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub struct FlacEncodeOptions;

/// High-level format selection for audio export.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum OutputFormat {
    /// RIFF/WAVE export using the built-in WAV adapter path.
    Wav(WavEncodeOptions),

    /// Headerless raw PCM export.
    RawPcm(RawPcmEncodeOptions),

    /// AIFF or AIFC export.
    Aiff(AiffEncodeOptions),

    /// FLAC export.
    Flac(FlacEncodeOptions),
}

impl OutputFormat {
    /// Returns the codec family selected by this output format.
    #[must_use]
    pub const fn codec_kind(self) -> CodecKind {
        match self {
            Self::Wav(_) => CodecKind::Wav,
            Self::RawPcm(_) => CodecKind::RawPcm,
            Self::Aiff(_) => CodecKind::Aiff,
            Self::Flac(_) => CodecKind::Flac,
        }
    }
}

impl Default for OutputFormat {
    fn default() -> Self {
        Self::Wav(WavEncodeOptions::default())
    }
}

/// Deterministic metadata returned after one encode request completes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct EncodeSummary {
    kind: CodecKind,
    spec: AudioSpec,
    frames: FrameCount,
}

impl EncodeSummary {
    /// Builds an encode summary for a completed output.
    #[must_use]
    pub const fn new(kind: CodecKind, spec: AudioSpec, frames: FrameCount) -> Self {
        Self { kind, spec, frames }
    }

    /// Returns the output codec family.
    #[must_use]
    pub const fn codec_kind(self) -> CodecKind {
        self.kind
    }

    /// Returns the encoded audio spec.
    #[must_use]
    pub const fn spec(self) -> AudioSpec {
        self.spec
    }

    /// Returns the encoded frame count.
    #[must_use]
    pub const fn frames(self) -> FrameCount {
        self.frames
    }
}

/// Seekable byte sink used by codec encoders.
pub trait AudioOutput: Write + Seek {}

impl<T> AudioOutput for T where T: Write + Seek + ?Sized {}

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

/// Boundary implemented by high-level audio encoders.
///
/// Implementations are deterministic for a fixed input buffer, selected
/// options, and active codec build. They write to a seekable byte sink so the
/// current WAV backend can finalize its header without leaking backend details
/// into the public API.
pub trait AudioEncoder {
    /// Returns the codec kind produced by this encoder.
    fn codec_kind(&self) -> CodecKind;

    /// Encodes an entire internal planar `f32` buffer into `output`.
    ///
    /// # Errors
    ///
    /// Returns [`CodecError::UnsupportedFormat`] when the encoder is a
    /// placeholder for a codec kind that is not available in the active build.
    fn encode(&self, input: &AudioBuffer, output: &mut dyn AudioOutput) -> Result<EncodeSummary>;
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

/// Encoder implementation that always reports an unsupported codec kind.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct UnsupportedEncoder {
    kind: CodecKind,
}

impl UnsupportedEncoder {
    /// Creates an encoder placeholder for `kind`.
    #[must_use]
    pub const fn new(kind: CodecKind) -> Self {
        Self { kind }
    }
}

impl AudioEncoder for UnsupportedEncoder {
    fn codec_kind(&self) -> CodecKind {
        self.kind
    }

    fn encode(&self, _input: &AudioBuffer, _output: &mut dyn AudioOutput) -> Result<EncodeSummary> {
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
        AiffEncodeOptions, AudioEncoder, AudioReader, AudioWriter, CodecCapabilities, CodecError,
        CodecKind, EncodeSummary, FlacEncodeOptions, OutputFormat, RawPcmEncodeOptions,
        RawPcmSampleFormat, UnsupportedEncoder, UnsupportedFormat, UnsupportedReader,
        UnsupportedWriter,
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

    #[test]
    fn output_format_maps_to_codec_kind() {
        assert_eq!(OutputFormat::default().codec_kind(), CodecKind::Wav);
        assert_eq!(
            OutputFormat::RawPcm(RawPcmEncodeOptions::default()).codec_kind(),
            CodecKind::RawPcm
        );
        assert_eq!(
            RawPcmEncodeOptions::unsigned24().sample_format(),
            RawPcmSampleFormat::Unsigned24
        );
        assert_eq!(
            RawPcmEncodeOptions::float64().sample_format(),
            RawPcmSampleFormat::Float64
        );
        assert_eq!(
            OutputFormat::Aiff(AiffEncodeOptions).codec_kind(),
            CodecKind::Aiff
        );
        assert_eq!(
            OutputFormat::Flac(FlacEncodeOptions).codec_kind(),
            CodecKind::Flac
        );
    }

    #[test]
    fn encode_summary_reports_kind_spec_and_frames() {
        let audio = mono_buffer();
        let summary = EncodeSummary::new(CodecKind::Wav, audio.spec(), audio.frames());

        assert_eq!(summary.codec_kind(), CodecKind::Wav);
        assert_eq!(summary.spec(), audio.spec());
        assert_eq!(summary.frames(), audio.frames());
    }

    #[test]
    fn unsupported_encoder_returns_typed_error() {
        let audio = mono_buffer();
        let encoder = UnsupportedEncoder::new(CodecKind::Aiff);
        let mut output = std::io::Cursor::new(Vec::new());
        let error = encoder.encode(&audio, &mut output).unwrap_err();

        assert_eq!(encoder.codec_kind(), CodecKind::Aiff);
        assert_eq!(
            error,
            CodecError::UnsupportedFormat(UnsupportedFormat::new(CodecKind::Aiff))
        );
    }
}

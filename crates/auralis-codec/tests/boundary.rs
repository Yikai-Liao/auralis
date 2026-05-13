#![allow(missing_docs)]

use auralis_codec::{
    AiffContainer, AiffEncodeOptions, AiffSampleFormat, AuEncodeOptions, AuSampleFormat,
    AudioEncoder, AudioReader, AudioWriter, CodecCapabilities, CodecError, CodecKind,
    EncodeSummary, FlacEncodeOptions, OutputFormat, RawPcmBitOrder, RawPcmByteOrder,
    RawPcmEncodeOptions, RawPcmNibbleOrder, RawPcmSampleFormat, UnsupportedEncoder,
    UnsupportedFormat, UnsupportedReader, UnsupportedWriter,
};
use auralis_core::{AudioBuffer, AudioSpec, ChannelCount, FrameCount, SampleFormat, SampleRate};

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
    for kind in [CodecKind::RawPcm, CodecKind::Aiff, CodecKind::Mp3] {
        let capabilities = CodecCapabilities::for_kind(kind);

        assert!(!capabilities.can_read());
        assert!(!capabilities.can_write());
    }
}

#[test]
fn wav_capability_tracks_feature_flag() {
    let capabilities = CodecCapabilities::for_kind(CodecKind::Wav);

    assert_eq!(capabilities.can_read(), cfg!(feature = "auralis-wav"));
    assert_eq!(capabilities.can_write(), cfg!(feature = "auralis-wav"));
}

#[test]
fn flac_capability_tracks_decode_feature_flag() {
    let capabilities = CodecCapabilities::for_kind(CodecKind::Flac);
    assert_eq!(capabilities.can_read(), cfg!(feature = "auralis-flac"));
    assert_eq!(capabilities.can_write(), cfg!(feature = "auralis-flac"));
}

#[test]
fn au_capability_is_built_in() {
    let capabilities = CodecCapabilities::for_kind(CodecKind::Au);
    assert!(capabilities.can_read());
    assert!(capabilities.can_write());
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
    let raw_options = RawPcmEncodeOptions::signed16()
        .with_byte_order(RawPcmByteOrder::BigEndian)
        .with_bit_order(RawPcmBitOrder::LeastSignificantBitFirst)
        .with_nibble_order(RawPcmNibbleOrder::LowNibbleFirst);
    assert_eq!(raw_options.byte_order(), RawPcmByteOrder::BigEndian);
    assert_eq!(
        raw_options.bit_order(),
        RawPcmBitOrder::LeastSignificantBitFirst
    );
    assert_eq!(
        raw_options.nibble_order(),
        RawPcmNibbleOrder::LowNibbleFirst
    );
    assert_eq!(
        AiffEncodeOptions::signed24().sample_format(),
        AiffSampleFormat::Signed24
    );
    assert_eq!(
        AiffEncodeOptions::aifc_ulaw().container(),
        AiffContainer::Aifc
    );
    assert_eq!(
        AuEncodeOptions::float64().sample_format(),
        AuSampleFormat::Float64
    );
    assert_eq!(
        OutputFormat::Aiff(AiffEncodeOptions::default()).codec_kind(),
        CodecKind::Aiff
    );
    assert_eq!(
        OutputFormat::Au(AuEncodeOptions::default()).codec_kind(),
        CodecKind::Au
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

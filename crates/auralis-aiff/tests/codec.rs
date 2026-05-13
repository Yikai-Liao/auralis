//! AIFF PCM codec-boundary tests.
#![allow(missing_docs)]

use std::io::Cursor;

use aifc::{AifcReader, FileFormat, Sample, SampleFormat};
use auralis_aiff::{AiffError, AiffPcmEncoder, decode_aiff, encode_aiff};
use auralis_codec::{AiffEncodeOptions, AudioEncoder, CodecKind};
use auralis_core::{
    AudioBuffer, AudioSpec, ChannelCount, FrameCount, SampleFormat as CoreFormat, SampleRate,
};

fn audio_buffer(channels: u16, frames: u64, data: &[f32]) -> AudioBuffer {
    let spec = AudioSpec::new(
        SampleRate::new(48_000).unwrap(),
        ChannelCount::new(channels).unwrap(),
        CoreFormat::Float32,
    );
    AudioBuffer::from_planar_f32(spec, FrameCount::new(frames), data.to_vec()).unwrap()
}

#[test]
fn encodes_plain_aiff_signed_integer_pcm() {
    let audio = audio_buffer(1, 3, &[-1.0, 0.0, 1.0]);

    let mut bytes = Cursor::new(Vec::new());
    encode_aiff(&mut bytes, &audio, AiffEncodeOptions::signed16()).unwrap();

    let mut reader = AifcReader::new(Cursor::new(bytes.into_inner())).unwrap();
    let info = reader.info();
    assert_eq!(info.file_format, FileFormat::Aiff);
    assert_eq!(info.channels, 1);
    assert!((info.sample_rate - 48_000.0).abs() < f64::EPSILON);
    assert_eq!(info.sample_format, SampleFormat::I16);
    assert_eq!(reader.read_sample().unwrap(), Some(Sample::I16(i16::MIN)));
    assert_eq!(reader.read_sample().unwrap(), Some(Sample::I16(0)));
    assert_eq!(reader.read_sample().unwrap(), Some(Sample::I16(i16::MAX)));
}

#[test]
fn encodes_supported_aiff_pcm_widths() {
    let audio = audio_buffer(1, 2, &[-1.0, 1.0]);

    for (options, expected_format) in [
        (AiffEncodeOptions::signed8(), SampleFormat::I8),
        (AiffEncodeOptions::signed16(), SampleFormat::I16),
        (AiffEncodeOptions::signed24(), SampleFormat::I24),
        (AiffEncodeOptions::signed32(), SampleFormat::I32),
    ] {
        let mut bytes = Cursor::new(Vec::new());
        encode_aiff(&mut bytes, &audio, options).unwrap();
        let reader = AifcReader::new(Cursor::new(bytes.into_inner())).unwrap();
        assert_eq!(reader.info().sample_format, expected_format);
    }
}

#[test]
fn decodes_plain_aiff_pcm_into_planar_f32() {
    let source = audio_buffer(2, 2, &[0.0, 0.5, -0.5, 1.0]);
    let mut bytes = Cursor::new(Vec::new());
    encode_aiff(&mut bytes, &source, AiffEncodeOptions::signed16()).unwrap();

    let decoded = decode_aiff(Cursor::new(bytes.into_inner())).unwrap();

    assert_eq!(decoded.spec(), source.spec());
    assert_eq!(decoded.frames(), source.frames());
    assert_eq!(decoded.as_planar_f32(), &[0.0, 0.5, -0.5, 0.999_969_5]);
}

#[test]
fn rejects_aifc_until_compressed_leaf_lands() {
    let mut bytes = Cursor::new(Vec::new());
    let info = aifc::AifcWriteInfo {
        file_format: FileFormat::Aifc,
        channels: 1,
        sample_rate: 48_000.0,
        sample_format: SampleFormat::I16,
    };
    let mut writer = aifc::AifcWriter::new(&mut bytes, &info).unwrap();
    writer.write_samples_i16(&[0]).unwrap();
    writer.finalize().unwrap();

    let error = decode_aiff(Cursor::new(bytes.into_inner())).unwrap_err();

    assert!(matches!(error, AiffError::UnsupportedContainer { .. }));
}

#[test]
fn rejects_non_finite_samples_with_position() {
    let audio = audio_buffer(2, 2, &[0.0, f32::NAN, 0.0, 1.0]);
    let error = encode_aiff(
        Cursor::new(Vec::new()),
        &audio,
        AiffEncodeOptions::signed16(),
    )
    .unwrap_err();

    assert!(matches!(
        error,
        AiffError::NonFiniteSample {
            channel_index: 0,
            frame_index: 1
        }
    ));
}

#[test]
fn implements_codec_encoder_boundary() {
    let audio = audio_buffer(1, 2, &[-1.0, 1.0]);
    let encoder = AiffPcmEncoder::new(AiffEncodeOptions::signed8());
    let mut output = Cursor::new(Vec::new());

    let summary = encoder.encode(&audio, &mut output).unwrap();

    assert_eq!(encoder.codec_kind(), CodecKind::Aiff);
    assert_eq!(summary.codec_kind(), CodecKind::Aiff);
    assert_eq!(summary.spec(), audio.spec());
    assert_eq!(summary.frames(), audio.frames());
    let mut reader = AifcReader::new(Cursor::new(output.into_inner())).unwrap();
    assert_eq!(reader.read_sample().unwrap(), Some(Sample::I8(i8::MIN)));
    assert_eq!(reader.read_sample().unwrap(), Some(Sample::I8(i8::MAX)));
}

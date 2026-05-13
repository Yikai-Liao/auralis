#![allow(missing_docs)]

use std::io::Cursor;

use auralis_au::{AuEncoder, AuError, decode_au, decode_au_path, encode_au, encode_au_path};
use auralis_codec::{AuEncodeOptions, AuSampleFormat, AudioEncoder, CodecKind};
use auralis_core::{AudioBuffer, AudioSpec, ChannelCount, FrameCount, SampleFormat, SampleRate};

fn stereo_buffer() -> AudioBuffer {
    let spec = AudioSpec::new(
        SampleRate::new(8_000).unwrap(),
        ChannelCount::new(2).unwrap(),
        SampleFormat::Float32,
    );
    AudioBuffer::from_planar_f32(
        spec,
        FrameCount::new(3),
        vec![-1.0, 0.0, 1.0, 0.5, -0.5, 0.25],
    )
    .unwrap()
}

#[test]
fn signed16_round_trip_decodes_planar_samples() {
    let audio = stereo_buffer();
    let mut bytes = Vec::new();
    encode_au(&mut bytes, &audio, AuEncodeOptions::signed16()).unwrap();

    assert_eq!(&bytes[..4], b".snd");
    assert_eq!(u32::from_be_bytes(bytes[12..16].try_into().unwrap()), 3);
    assert_eq!(u32::from_be_bytes(bytes[16..20].try_into().unwrap()), 8_000);
    assert_eq!(u32::from_be_bytes(bytes[20..24].try_into().unwrap()), 2);

    let decoded = decode_au(Cursor::new(bytes)).unwrap();
    assert_eq!(decoded.spec(), audio.spec());
    assert_eq!(decoded.frames(), audio.frames());
    assert_eq!(
        decoded.as_planar_f32(),
        &[-1.0, 0.0, 32767.0 / 32768.0, 0.5, -0.5, 0.25]
    );
}

#[test]
fn all_supported_options_emit_the_expected_encoding_codes() {
    let audio = stereo_buffer();
    let cases = [
        (AuEncodeOptions::ulaw(), AuSampleFormat::ULaw, 1),
        (AuEncodeOptions::signed8(), AuSampleFormat::Signed8, 2),
        (AuEncodeOptions::signed16(), AuSampleFormat::Signed16, 3),
        (AuEncodeOptions::signed24(), AuSampleFormat::Signed24, 4),
        (AuEncodeOptions::signed32(), AuSampleFormat::Signed32, 5),
        (AuEncodeOptions::float32(), AuSampleFormat::Float32, 6),
        (AuEncodeOptions::float64(), AuSampleFormat::Float64, 7),
        (AuEncodeOptions::alaw(), AuSampleFormat::ALaw, 27),
    ];

    for (options, sample_format, encoding) in cases {
        let mut bytes = Vec::new();
        assert_eq!(options.sample_format(), sample_format);
        encode_au(&mut bytes, &audio, options).unwrap();
        assert_eq!(
            u32::from_be_bytes(bytes[12..16].try_into().unwrap()),
            encoding
        );
        let decoded = decode_au(Cursor::new(bytes)).unwrap();
        assert_eq!(decoded.spec().sample_rate().as_u32(), 8_000);
        assert_eq!(decoded.channels().as_u16(), 2);
        assert_eq!(decoded.frames().as_u64(), 3);
    }
}

#[test]
fn codec_boundary_encoder_reports_summary() {
    let audio = stereo_buffer();
    let encoder = AuEncoder::new(AuEncodeOptions::float32());
    let mut output = Cursor::new(Vec::new());
    let summary = encoder.encode(&audio, &mut output).unwrap();

    assert_eq!(encoder.codec_kind(), CodecKind::Au);
    assert_eq!(summary.codec_kind(), CodecKind::Au);
    assert_eq!(summary.spec(), audio.spec());
    assert_eq!(summary.frames(), audio.frames());
    assert_eq!(
        u32::from_be_bytes(output.into_inner()[12..16].try_into().unwrap()),
        6
    );
}

#[test]
fn path_helpers_round_trip() {
    let audio = stereo_buffer();
    let path = std::env::temp_dir().join(format!(
        "auralis-au-{}-{}.au",
        std::process::id(),
        "path-roundtrip"
    ));
    encode_au_path(&path, &audio, AuEncodeOptions::signed24()).unwrap();
    let decoded = decode_au_path(&path).unwrap();
    std::fs::remove_file(path).unwrap();

    assert_eq!(decoded.frames(), audio.frames());
    assert_eq!(decoded.channels(), audio.channels());
}

#[test]
fn malformed_and_unsupported_inputs_return_typed_errors() {
    assert_eq!(
        decode_au(Cursor::new([0_u8; 24])),
        Err(AuError::MissingMagic)
    );

    let mut bytes = Vec::new();
    bytes.extend_from_slice(b".snd");
    for value in [24_u32, 0_u32, 99_u32, 8_000_u32, 1_u32] {
        bytes.extend_from_slice(&value.to_be_bytes());
    }
    assert_eq!(
        decode_au(Cursor::new(bytes)),
        Err(AuError::UnsupportedEncoding { encoding: 99 })
    );
}

#[test]
fn non_finite_encode_fails() {
    let spec = AudioSpec::new(
        SampleRate::new(8_000).unwrap(),
        ChannelCount::new(1).unwrap(),
        SampleFormat::Float32,
    );
    let audio = AudioBuffer::from_planar_f32(spec, FrameCount::new(1), vec![f32::NAN]).unwrap();
    let error = encode_au(Vec::new(), &audio, AuEncodeOptions::signed16()).unwrap_err();

    assert_eq!(
        error,
        AuError::NonFiniteSample {
            channel_index: 0,
            frame_index: 0
        }
    );
}

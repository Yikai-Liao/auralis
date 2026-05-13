//! Raw PCM encode and codec-boundary tests.
#![allow(missing_docs)]

use std::io::Cursor;

use auralis_codec::{
    AudioEncoder, CodecKind, RawPcmBitOrder, RawPcmByteOrder, RawPcmEncodeOptions,
    RawPcmNibbleOrder,
};
use auralis_core::{AudioBuffer, AudioSpec, ChannelCount, FrameCount, SampleFormat, SampleRate};
use auralis_raw::{RawPcmEncoder, RawPcmError, encode_raw_pcm};

fn audio_buffer(channels: u16, frames: u64, data: &[f32]) -> AudioBuffer {
    let spec = AudioSpec::new(
        SampleRate::new(48_000).unwrap(),
        ChannelCount::new(channels).unwrap(),
        SampleFormat::Float32,
    );
    AudioBuffer::from_planar_f32(spec, FrameCount::new(frames), data.to_vec()).unwrap()
}

#[test]
fn encodes_signed_and_unsigned_integer_raw_pcm() {
    let audio = audio_buffer(1, 3, &[-1.0, 0.0, 1.0]);

    let mut signed8 = Vec::new();
    encode_raw_pcm(&mut signed8, &audio, RawPcmEncodeOptions::signed8()).unwrap();
    assert_eq!(signed8, vec![0x80, 0x00, 0x7f]);

    let mut unsigned8 = Vec::new();
    encode_raw_pcm(&mut unsigned8, &audio, RawPcmEncodeOptions::unsigned8()).unwrap();
    assert_eq!(unsigned8, vec![0x00, 0x80, 0xff]);

    let mut signed16 = Vec::new();
    encode_raw_pcm(&mut signed16, &audio, RawPcmEncodeOptions::signed16()).unwrap();
    assert_eq!(signed16, vec![0x00, 0x80, 0x00, 0x00, 0xff, 0x7f]);

    let mut unsigned16 = Vec::new();
    encode_raw_pcm(&mut unsigned16, &audio, RawPcmEncodeOptions::unsigned16()).unwrap();
    assert_eq!(unsigned16, vec![0x00, 0x00, 0x00, 0x80, 0xff, 0xff]);
}

#[test]
fn interleaves_planar_channels_by_frame() {
    let audio = audio_buffer(2, 2, &[0.0, 0.5, -0.5, 1.0]);
    let mut output = Vec::new();

    encode_raw_pcm(&mut output, &audio, RawPcmEncodeOptions::signed8()).unwrap();

    assert_eq!(output, vec![0x00, 0xc0, 0x40, 0x7f]);
}

#[test]
fn encodes_24_and_32_bit_samples_little_endian() {
    let audio = audio_buffer(1, 2, &[-1.0, 1.0]);

    let mut signed24 = Vec::new();
    encode_raw_pcm(&mut signed24, &audio, RawPcmEncodeOptions::signed24()).unwrap();
    assert_eq!(signed24, vec![0x00, 0x00, 0x80, 0xff, 0xff, 0x7f]);

    let mut unsigned24 = Vec::new();
    encode_raw_pcm(&mut unsigned24, &audio, RawPcmEncodeOptions::unsigned24()).unwrap();
    assert_eq!(unsigned24, vec![0x00, 0x00, 0x00, 0xff, 0xff, 0xff]);

    let mut signed32 = Vec::new();
    encode_raw_pcm(&mut signed32, &audio, RawPcmEncodeOptions::signed32()).unwrap();
    assert_eq!(
        signed32,
        vec![0x00, 0x00, 0x00, 0x80, 0xff, 0xff, 0xff, 0x7f]
    );

    let mut unsigned32 = Vec::new();
    encode_raw_pcm(&mut unsigned32, &audio, RawPcmEncodeOptions::unsigned32()).unwrap();
    assert_eq!(
        unsigned32,
        vec![0x00, 0x00, 0x00, 0x00, 0xff, 0xff, 0xff, 0xff]
    );
}

#[test]
fn encodes_float_samples_little_endian() {
    let audio = audio_buffer(1, 3, &[-1.0, 0.5, 1.0]);

    let mut float32 = Vec::new();
    encode_raw_pcm(&mut float32, &audio, RawPcmEncodeOptions::float32()).unwrap();
    assert_eq!(
        float32,
        [
            (-1.0_f32).to_le_bytes(),
            0.5_f32.to_le_bytes(),
            1.0_f32.to_le_bytes(),
        ]
        .concat()
    );

    let mut float64 = Vec::new();
    encode_raw_pcm(&mut float64, &audio, RawPcmEncodeOptions::float64()).unwrap();
    assert_eq!(
        float64,
        [
            (-1.0_f64).to_le_bytes(),
            0.5_f64.to_le_bytes(),
            1.0_f64.to_le_bytes(),
        ]
        .concat()
    );
}

#[test]
fn encodes_multi_byte_samples_big_endian() {
    let audio = audio_buffer(1, 2, &[-1.0, 1.0]);

    let mut signed16 = Vec::new();
    encode_raw_pcm(
        &mut signed16,
        &audio,
        RawPcmEncodeOptions::signed16().with_byte_order(RawPcmByteOrder::BigEndian),
    )
    .unwrap();
    assert_eq!(signed16, vec![0x80, 0x00, 0x7f, 0xff]);

    let mut signed24 = Vec::new();
    encode_raw_pcm(
        &mut signed24,
        &audio,
        RawPcmEncodeOptions::signed24().with_byte_order(RawPcmByteOrder::BigEndian),
    )
    .unwrap();
    assert_eq!(signed24, vec![0x80, 0x00, 0x00, 0x7f, 0xff, 0xff]);

    let mut float32 = Vec::new();
    encode_raw_pcm(
        &mut float32,
        &audio,
        RawPcmEncodeOptions::float32().with_byte_order(RawPcmByteOrder::BigEndian),
    )
    .unwrap();
    assert_eq!(
        float32,
        [(-1.0_f32).to_be_bytes(), 1.0_f32.to_be_bytes(),].concat()
    );
}

#[test]
fn applies_raw_bit_and_nibble_order_per_byte() {
    let audio = audio_buffer(1, 1, &[0.5]);

    let mut nibble_swapped = Vec::new();
    encode_raw_pcm(
        &mut nibble_swapped,
        &audio,
        RawPcmEncodeOptions::unsigned8().with_nibble_order(RawPcmNibbleOrder::LowNibbleFirst),
    )
    .unwrap();
    assert_eq!(nibble_swapped, vec![0xfb]);

    let mut bit_reversed = Vec::new();
    encode_raw_pcm(
        &mut bit_reversed,
        &audio,
        RawPcmEncodeOptions::unsigned8().with_bit_order(RawPcmBitOrder::LeastSignificantBitFirst),
    )
    .unwrap();
    assert_eq!(bit_reversed, vec![0xfd]);
}

#[test]
fn rejects_non_finite_samples_with_position() {
    let audio = audio_buffer(2, 2, &[0.0, f32::NAN, 0.0, 1.0]);
    let error = encode_raw_pcm(Vec::new(), &audio, RawPcmEncodeOptions::signed16()).unwrap_err();

    assert!(matches!(
        error,
        RawPcmError::NonFiniteSample {
            channel_index: 0,
            frame_index: 1
        }
    ));
}

#[test]
fn implements_codec_encoder_boundary() {
    let audio = audio_buffer(1, 2, &[-1.0, 1.0]);
    let encoder = RawPcmEncoder::new(RawPcmEncodeOptions::unsigned8());
    let mut output = Cursor::new(Vec::new());

    let summary = encoder.encode(&audio, &mut output).unwrap();

    assert_eq!(encoder.codec_kind(), CodecKind::RawPcm);
    assert_eq!(summary.codec_kind(), CodecKind::RawPcm);
    assert_eq!(summary.spec(), audio.spec());
    assert_eq!(summary.frames(), audio.frames());
    assert_eq!(output.into_inner(), vec![0x00, 0xff]);
}

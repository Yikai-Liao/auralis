#![allow(dead_code)]

use std::{
    env, fs,
    path::{Path, PathBuf},
    sync::atomic::{AtomicUsize, Ordering},
};

use auralis_core::{AudioBuffer, AudioSpec, ChannelCount, FrameCount, SampleFormat, SampleRate};
use auralis_simd::BackendKind;
use auralis_wav::{
    decode_alaw_path, decode_float32_path, decode_float64_path, decode_pcm8_path,
    decode_pcm16_path, decode_pcm24_path, decode_pcm32_path, decode_ulaw_path, encode_alaw_path,
    encode_float32_path, encode_float64_path, encode_pcm8_path, encode_pcm16_path,
    encode_pcm16_path_with_backend, encode_pcm24_path, encode_pcm32_path, encode_ulaw_path,
};

pub(crate) fn wav_bytes(channels: u16, samples: &[i16]) -> Vec<u8> {
    let mut bytes = riff_header(channels, 16, 1, u32::try_from(samples.len() * 2).unwrap());
    for sample in samples {
        bytes.extend_from_slice(&sample.to_le_bytes());
    }
    bytes
}

pub(crate) fn wav_bytes_pcm8(channels: u16, samples: &[i8]) -> Vec<u8> {
    let mut bytes = riff_header(channels, 8, 1, u32::try_from(samples.len()).unwrap());
    for sample in samples {
        bytes.push(u8::try_from(i16::from(*sample) + 128).unwrap());
    }
    bytes
}

pub(crate) fn wav_bytes_pcm24(channels: u16, samples: &[i32]) -> Vec<u8> {
    let mut bytes = riff_header(channels, 24, 1, u32::try_from(samples.len() * 3).unwrap());
    for sample in samples {
        let le_bytes = sample.to_le_bytes();
        bytes.extend_from_slice(&le_bytes[..3]);
    }
    bytes
}

pub(crate) fn wav_bytes_pcm32(channels: u16, samples: &[i32]) -> Vec<u8> {
    let mut bytes = riff_header(channels, 32, 1, u32::try_from(samples.len() * 4).unwrap());
    for sample in samples {
        bytes.extend_from_slice(&sample.to_le_bytes());
    }
    bytes
}

pub(crate) fn rifx_bytes_pcm16(channels: u16, samples: &[i16]) -> Vec<u8> {
    let mut bytes = rifx_header(channels, 16, 1, u32::try_from(samples.len() * 2).unwrap());
    for sample in samples {
        bytes.extend_from_slice(&sample.to_be_bytes());
    }
    bytes
}

pub(crate) fn wav_bytes_float32(channels: u16, samples: &[f32]) -> Vec<u8> {
    let mut bytes = riff_header(channels, 32, 3, u32::try_from(samples.len() * 4).unwrap());
    for sample in samples {
        bytes.extend_from_slice(&sample.to_le_bytes());
    }
    bytes
}

pub(crate) fn rifx_bytes_float32(channels: u16, samples: &[f32]) -> Vec<u8> {
    let mut bytes = rifx_header(channels, 32, 3, u32::try_from(samples.len() * 4).unwrap());
    for sample in samples {
        bytes.extend_from_slice(&sample.to_be_bytes());
    }
    bytes
}

pub(crate) fn wav_bytes_float64(channels: u16, samples: &[f64]) -> Vec<u8> {
    let mut bytes = riff_header(channels, 64, 3, u32::try_from(samples.len() * 8).unwrap());
    for sample in samples {
        bytes.extend_from_slice(&sample.to_le_bytes());
    }
    bytes
}

pub(crate) fn wav_bytes_ulaw(channels: u16, samples: &[u8]) -> Vec<u8> {
    wav_bytes_g711(channels, 0x0007, samples)
}

pub(crate) fn wav_bytes_alaw(channels: u16, samples: &[u8]) -> Vec<u8> {
    wav_bytes_g711(channels, 0x0006, samples)
}

fn wav_bytes_g711(channels: u16, format_tag: u16, samples: &[u8]) -> Vec<u8> {
    let mut bytes = riff_header(
        channels,
        8,
        format_tag,
        u32::try_from(samples.len()).unwrap(),
    );
    bytes.extend_from_slice(samples);
    bytes
}

pub(crate) fn wav_bytes_with_bits(channels: u16, bits_per_sample: u16, payload: &[u8]) -> Vec<u8> {
    let mut bytes = riff_header(
        channels,
        bits_per_sample,
        1,
        u32::try_from(payload.len()).unwrap(),
    );
    bytes.extend_from_slice(payload);
    bytes
}

pub(crate) fn riff_header(
    channels: u16,
    bits_per_sample: u16,
    format_tag: u16,
    data_bytes: u32,
) -> Vec<u8> {
    let bytes_per_sample = u32::from(bits_per_sample) / 8;
    let byte_rate = 48_000 * u32::from(channels) * bytes_per_sample;
    let block_align = channels * (bits_per_sample / 8);
    let riff_size = 36 + data_bytes;
    let mut bytes = Vec::new();

    bytes.extend_from_slice(b"RIFF");
    bytes.extend_from_slice(&riff_size.to_le_bytes());
    bytes.extend_from_slice(b"WAVE");
    bytes.extend_from_slice(b"fmt ");
    bytes.extend_from_slice(&16_u32.to_le_bytes());
    bytes.extend_from_slice(&format_tag.to_le_bytes());
    bytes.extend_from_slice(&channels.to_le_bytes());
    bytes.extend_from_slice(&48_000_u32.to_le_bytes());
    bytes.extend_from_slice(&byte_rate.to_le_bytes());
    bytes.extend_from_slice(&block_align.to_le_bytes());
    bytes.extend_from_slice(&bits_per_sample.to_le_bytes());
    bytes.extend_from_slice(b"data");
    bytes.extend_from_slice(&data_bytes.to_le_bytes());
    bytes
}

pub(crate) fn rifx_header(
    channels: u16,
    bits_per_sample: u16,
    format_tag: u16,
    data_bytes: u32,
) -> Vec<u8> {
    let bytes_per_sample = u32::from(bits_per_sample) / 8;
    let byte_rate = 48_000 * u32::from(channels) * bytes_per_sample;
    let block_align = channels * (bits_per_sample / 8);
    let riff_size = 36 + data_bytes;
    let mut bytes = Vec::new();

    bytes.extend_from_slice(b"RIFX");
    bytes.extend_from_slice(&riff_size.to_be_bytes());
    bytes.extend_from_slice(b"WAVE");
    bytes.extend_from_slice(b"fmt ");
    bytes.extend_from_slice(&16_u32.to_be_bytes());
    bytes.extend_from_slice(&format_tag.to_be_bytes());
    bytes.extend_from_slice(&channels.to_be_bytes());
    bytes.extend_from_slice(&48_000_u32.to_be_bytes());
    bytes.extend_from_slice(&byte_rate.to_be_bytes());
    bytes.extend_from_slice(&block_align.to_be_bytes());
    bytes.extend_from_slice(&bits_per_sample.to_be_bytes());
    bytes.extend_from_slice(b"data");
    bytes.extend_from_slice(&data_bytes.to_be_bytes());
    bytes
}

pub(crate) fn write_temp_wav(
    prefix: &str,
    channels: u16,
    samples: &[i16],
) -> std::io::Result<PathBuf> {
    let path = temp_path(prefix, "wav");
    fs::write(&path, wav_bytes(channels, samples))?;
    Ok(path)
}

pub(crate) fn temp_path(prefix: &str, extension: &str) -> PathBuf {
    static NEXT_ID: AtomicUsize = AtomicUsize::new(0);

    let id = NEXT_ID.fetch_add(1, Ordering::Relaxed);
    env::temp_dir().join(format!("{prefix}-{}-{id}.{extension}", std::process::id()))
}

pub(crate) fn read_le_f32(path: &Path) -> std::io::Result<Vec<f32>> {
    let bytes = fs::read(path)?;
    Ok(bytes
        .chunks_exact(4)
        .map(|chunk| f32::from_le_bytes(chunk.try_into().unwrap()))
        .collect())
}

pub(crate) fn write_le_f32(path: &Path, samples: &[f32]) -> std::io::Result<()> {
    let mut bytes = Vec::with_capacity(samples.len() * 4);
    for sample in samples {
        bytes.extend_from_slice(&sample.to_le_bytes());
    }

    fs::write(path, bytes)
}

pub(crate) fn interleave_planar(planar: &[f32], channels: u16) -> Vec<f32> {
    let channel_count = usize::from(channels);
    let frames = planar.len() / channel_count;
    let mut interleaved = Vec::with_capacity(planar.len());

    for frame in 0..frames {
        for channel in 0..channel_count {
            interleaved.push(planar[channel * frames + frame]);
        }
    }

    interleaved
}

pub(crate) fn audio_buffer(channels: u16, frames: u64, planar: &[f32]) -> AudioBuffer {
    let spec = AudioSpec::new(
        SampleRate::new(48_000).unwrap(),
        ChannelCount::new(channels).unwrap(),
        SampleFormat::Float32,
    );

    AudioBuffer::from_planar_f32(spec, FrameCount::new(frames), planar.to_vec()).unwrap()
}

pub(crate) fn encode_temp_wav(prefix: &str, audio: &AudioBuffer) -> PathBuf {
    let path = temp_path(prefix, "wav");
    encode_pcm16_path(&path, audio).unwrap();
    path
}

pub(crate) fn encode_temp_wav_pcm8(prefix: &str, audio: &AudioBuffer) -> PathBuf {
    let path = temp_path(prefix, "wav");
    encode_pcm8_path(&path, audio).unwrap();
    path
}

pub(crate) fn encode_temp_wav_pcm24(prefix: &str, audio: &AudioBuffer) -> PathBuf {
    let path = temp_path(prefix, "wav");
    encode_pcm24_path(&path, audio).unwrap();
    path
}

pub(crate) fn encode_temp_wav_pcm32(prefix: &str, audio: &AudioBuffer) -> PathBuf {
    let path = temp_path(prefix, "wav");
    encode_pcm32_path(&path, audio).unwrap();
    path
}

pub(crate) fn encode_temp_wav_float32(prefix: &str, audio: &AudioBuffer) -> PathBuf {
    let path = temp_path(prefix, "wav");
    encode_float32_path(&path, audio).unwrap();
    path
}

pub(crate) fn encode_temp_wav_float64(prefix: &str, audio: &AudioBuffer) -> PathBuf {
    let path = temp_path(prefix, "wav");
    encode_float64_path(&path, audio).unwrap();
    path
}

pub(crate) fn encode_temp_wav_ulaw(prefix: &str, audio: &AudioBuffer) -> PathBuf {
    let path = temp_path(prefix, "wav");
    encode_ulaw_path(&path, audio).unwrap();
    path
}

pub(crate) fn encode_temp_wav_alaw(prefix: &str, audio: &AudioBuffer) -> PathBuf {
    let path = temp_path(prefix, "wav");
    encode_alaw_path(&path, audio).unwrap();
    path
}

pub(crate) fn encode_temp_wav_with_backend(
    prefix: &str,
    audio: &AudioBuffer,
    backend: BackendKind,
) -> PathBuf {
    let path = temp_path(prefix, "wav");
    encode_pcm16_path_with_backend(&path, audio, backend).unwrap();
    path
}

pub(crate) fn assert_audio_bits_eq(actual: &AudioBuffer, expected: &AudioBuffer) {
    assert_eq!(actual.spec(), expected.spec());
    assert_eq!(actual.frames(), expected.frames());

    for channel_index in 0..actual.channels().as_usize() {
        let actual = actual.channel(channel_index).unwrap();
        let expected = expected.channel(channel_index).unwrap();
        assert_eq!(actual.len(), expected.len());

        for (frame_index, (&actual, &expected)) in actual.iter().zip(expected).enumerate() {
            assert_eq!(
                actual.to_bits(),
                expected.to_bits(),
                "channel {channel_index} frame {frame_index} differed: {actual} != {expected}"
            );
        }
    }
}

pub(crate) fn assert_close_by_one_lsb(left: &[f32], right: &[f32]) {
    assert_eq!(left.len(), right.len());
    for (index, (left, right)) in left.iter().zip(right).enumerate() {
        let delta = (left - right).abs();
        assert!(
            delta <= 1.0 / 32768.0,
            "sample {index} differed by {delta}: {left} != {right}"
        );
    }
}

pub(crate) fn decode_path(path: &Path) -> AudioBuffer {
    decode_pcm16_path(path).unwrap()
}

pub(crate) fn decode_path_pcm8(path: &Path) -> AudioBuffer {
    decode_pcm8_path(path).unwrap()
}

pub(crate) fn decode_path_pcm24(path: &Path) -> AudioBuffer {
    decode_pcm24_path(path).unwrap()
}

pub(crate) fn decode_path_pcm32(path: &Path) -> AudioBuffer {
    decode_pcm32_path(path).unwrap()
}

pub(crate) fn decode_path_float32(path: &Path) -> AudioBuffer {
    decode_float32_path(path).unwrap()
}

pub(crate) fn decode_path_float64(path: &Path) -> AudioBuffer {
    decode_float64_path(path).unwrap()
}

pub(crate) fn decode_path_ulaw(path: &Path) -> AudioBuffer {
    decode_ulaw_path(path).unwrap()
}

pub(crate) fn decode_path_alaw(path: &Path) -> AudioBuffer {
    decode_alaw_path(path).unwrap()
}

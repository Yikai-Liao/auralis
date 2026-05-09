#![allow(dead_code)]

use std::{
    path::PathBuf,
    time::{SystemTime, UNIX_EPOCH},
};

use auralis_core::{AudioBuffer, AudioSpec, ChannelCount, FrameCount, SampleFormat, SampleRate};

pub fn audio_buffer(samples: Vec<f32>) -> AudioBuffer {
    let frames = samples.len().try_into().unwrap();
    let spec = AudioSpec::new(
        SampleRate::new(48_000).unwrap(),
        ChannelCount::new(1).unwrap(),
        SampleFormat::Float32,
    );

    AudioBuffer::from_planar_f32(spec, FrameCount::new(frames), samples).unwrap()
}

pub fn stereo_audio_buffer(samples: Vec<f32>) -> AudioBuffer {
    assert_eq!(samples.len() % 2, 0);
    let frames = u64::try_from(samples.len() / 2).unwrap();

    audio_buffer_with_shape(samples, frames, 48_000, 2, SampleFormat::Float32)
}

pub fn audio_buffer_with_spec(
    samples: Vec<f32>,
    sample_rate: u32,
    channels: u16,
    sample_format: SampleFormat,
) -> AudioBuffer {
    assert_eq!(samples.len() % usize::from(channels), 0);
    let frames = u64::try_from(samples.len() / usize::from(channels)).unwrap();

    audio_buffer_with_shape(samples, frames, sample_rate, channels, sample_format)
}

pub fn audio_buffer_with_shape(
    samples: Vec<f32>,
    frames: u64,
    sample_rate: u32,
    channels: u16,
    sample_format: SampleFormat,
) -> AudioBuffer {
    let spec = AudioSpec::new(
        SampleRate::new(sample_rate).unwrap(),
        ChannelCount::new(channels).unwrap(),
        sample_format,
    );

    AudioBuffer::from_planar_f32(spec, FrameCount::new(frames), samples).unwrap()
}

pub fn assert_samples_close(actual: &[f32], expected: &[f32]) {
    assert_eq!(actual.len(), expected.len());

    for (actual, expected) in actual.iter().zip(expected) {
        let tolerance = 1.0e-4;
        let difference = (actual - expected).abs();

        assert!(
            difference <= tolerance,
            "expected {actual} to be within {tolerance} of {expected}, difference was {difference}"
        );
    }
}

pub fn assert_sample_bits_eq(actual: &[f32], expected: &[f32]) {
    assert_eq!(actual.len(), expected.len());

    for (index, (&actual, &expected)) in actual.iter().zip(expected).enumerate() {
        assert_eq!(
            actual.to_bits(),
            expected.to_bits(),
            "sample {index} differed: {actual} != {expected}"
        );
    }
}

pub fn temp_dir() -> PathBuf {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();

    std::env::temp_dir().join(format!("auralis-chain-api-{nanos}"))
}

pub fn temp_path(prefix: &str, extension: &str) -> PathBuf {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();

    std::env::temp_dir().join(format!("{prefix}-{nanos}.{extension}"))
}

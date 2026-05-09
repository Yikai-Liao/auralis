use std::fmt;

use auralis_core::{AudioBuffer, FrameCount};

use crate::{Result, WavError};

/// WAV sample encoding category used in unsupported-format errors.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WavSampleEncoding {
    /// Integer PCM samples.
    Integer,
    /// IEEE floating-point samples.
    Float,
}

impl fmt::Display for WavSampleEncoding {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Integer => formatter.write_str("integer PCM"),
            Self::Float => formatter.write_str("IEEE float"),
        }
    }
}

pub(crate) fn ensure_pcm16(spec: hound::WavSpec) -> Result<()> {
    if spec.sample_format == hound::SampleFormat::Int && spec.bits_per_sample == 16 {
        Ok(())
    } else {
        Err(WavError::UnsupportedSampleFormat {
            bits_per_sample: spec.bits_per_sample,
            encoding: match spec.sample_format {
                hound::SampleFormat::Int => WavSampleEncoding::Integer,
                hound::SampleFormat::Float => WavSampleEncoding::Float,
            },
        })
    }
}

pub(crate) fn hound_spec(audio: &AudioBuffer) -> hound::WavSpec {
    hound::WavSpec {
        channels: audio.channels().as_u16(),
        sample_rate: audio.spec().sample_rate().as_u32(),
        bits_per_sample: 16,
        sample_format: hound::SampleFormat::Int,
    }
}

pub(crate) fn frame_count(frames_per_channel: u32) -> FrameCount {
    FrameCount::new(u64::from(frames_per_channel))
}

pub(crate) fn malformed(error: &hound::Error) -> WavError {
    WavError::Malformed {
        message: error.to_string(),
    }
}

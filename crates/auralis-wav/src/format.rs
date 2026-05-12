use std::fmt;

use auralis_codec::WavSampleFormat;
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

pub(crate) fn wav_sample_format(spec: hound::WavSpec) -> Result<WavSampleFormat> {
    match (spec.sample_format, spec.bits_per_sample) {
        (hound::SampleFormat::Int, 8) => Ok(WavSampleFormat::Pcm8),
        (hound::SampleFormat::Int, 16) => Ok(WavSampleFormat::Pcm16),
        (hound::SampleFormat::Int, 24) => Ok(WavSampleFormat::Pcm24),
        _ => Err(WavError::UnsupportedSampleFormat {
            bits_per_sample: spec.bits_per_sample,
            encoding: match spec.sample_format {
                hound::SampleFormat::Int => WavSampleEncoding::Integer,
                hound::SampleFormat::Float => WavSampleEncoding::Float,
            },
        }),
    }
}

pub(crate) fn ensure_pcm16(spec: hound::WavSpec) -> Result<()> {
    match wav_sample_format(spec)? {
        WavSampleFormat::Pcm16 => Ok(()),
        _ => Err(WavError::UnsupportedSampleFormat {
            bits_per_sample: spec.bits_per_sample,
            encoding: WavSampleEncoding::Integer,
        }),
    }
}

pub(crate) fn hound_spec(audio: &AudioBuffer, sample_format: WavSampleFormat) -> hound::WavSpec {
    #[allow(
        clippy::match_same_arms,
        reason = "WavSampleFormat is non-exhaustive; the wildcard preserves a backward-compatible PCM16 default for future variants until they gain explicit handling."
    )]
    let bits_per_sample = match sample_format {
        WavSampleFormat::Pcm8 => 8,
        WavSampleFormat::Pcm16 => 16,
        WavSampleFormat::Pcm24 => 24,
        _ => 16,
    };

    hound::WavSpec {
        channels: audio.channels().as_u16(),
        sample_rate: audio.spec().sample_rate().as_u32(),
        bits_per_sample,
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

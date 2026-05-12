use auralis_simd::{
    BackendKind, SampleConversionError, f32_to_i16_with_backend, i16_to_f32_with_backend,
    select_backend,
};

use crate::{Result, WavError};

pub(crate) fn pcm16_to_f32_with_backend(
    requested_backend: BackendKind,
    input: &[i16],
    output: &mut [f32],
) -> Result<()> {
    i16_to_f32_with_backend(select_backend(requested_backend), input, output)
        .map_err(|_| WavError::InvalidBufferShape)
}

pub(crate) fn pcm8_to_f32(input: &[i8], output: &mut [f32]) -> Result<()> {
    if input.len() != output.len() {
        return Err(WavError::InvalidBufferShape);
    }

    for (source, destination) in input.iter().zip(output.iter_mut()) {
        *destination = f32::from(*source) / 128.0;
    }

    Ok(())
}

pub(crate) fn f32_to_pcm16_with_backend(
    requested_backend: BackendKind,
    input: &[f32],
    output: &mut [i16],
    channels: usize,
) -> Result<()> {
    f32_to_i16_with_backend(select_backend(requested_backend), input, output)
        .map_err(|error| sample_conversion_error(&error, channels))
}

pub(crate) fn f32_to_pcm8(input: &[f32], output: &mut [i8], channels: usize) -> Result<()> {
    if input.len() != output.len() {
        return Err(WavError::WriteFailed {
            message: "sample conversion buffer length mismatch".to_owned(),
        });
    }

    for (sample_index, (&sample, destination)) in input.iter().zip(output.iter_mut()).enumerate() {
        if !sample.is_finite() {
            return Err(WavError::NonFiniteSample {
                channel_index: sample_index % channels,
                frame_index: sample_index / channels,
            });
        }

        let clipped = sample.clamp(-1.0, 1.0);
        #[allow(
            clippy::cast_possible_truncation,
            reason = "The value is rounded and clamped to the i8 range immediately before the cast."
        )]
        let quantized = (clipped * 128.0).round() as i16;
        let quantized = quantized.clamp(i16::from(i8::MIN), i16::from(i8::MAX));
        *destination = i8::try_from(quantized).map_err(|_| WavError::WriteFailed {
            message: "PCM8 sample quantization overflowed".to_owned(),
        })?;
    }

    Ok(())
}

fn sample_conversion_error(error: &SampleConversionError, channels: usize) -> WavError {
    match error {
        SampleConversionError::NonFiniteSample { sample_index } => WavError::NonFiniteSample {
            channel_index: *sample_index % channels,
            frame_index: *sample_index / channels,
        },
        _ => WavError::WriteFailed {
            message: error.to_string(),
        },
    }
}

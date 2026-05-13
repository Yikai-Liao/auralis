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

pub(crate) fn pcm24_to_f32(input: &[i32], output: &mut [f32]) -> Result<()> {
    if input.len() != output.len() {
        return Err(WavError::InvalidBufferShape);
    }

    #[allow(
        clippy::cast_precision_loss,
        reason = "PCM24 samples are bounded to 24 significant bits, which remain exactly representable in f32 before normalization."
    )]
    for (source, destination) in input.iter().zip(output.iter_mut()) {
        *destination = (*source as f32) / 8_388_608.0;
    }

    Ok(())
}

pub(crate) fn pcm32_to_f32(input: &[i32], output: &mut [f32]) -> Result<()> {
    if input.len() != output.len() {
        return Err(WavError::InvalidBufferShape);
    }

    #[allow(
        clippy::cast_possible_truncation,
        reason = "PCM32 decode normalizes full-scale integers into the existing f32 processing buffer, so the explicit narrowing to f32 is the crate's public sample-model boundary."
    )]
    for (source, destination) in input.iter().zip(output.iter_mut()) {
        *destination = (f64::from(*source) / 2_147_483_648.0) as f32;
    }

    Ok(())
}

pub(crate) fn float32_to_f32(input: &[f32], output: &mut [f32], channels: usize) -> Result<()> {
    if input.len() != output.len() {
        return Err(WavError::InvalidBufferShape);
    }

    for (sample_index, (&sample, destination)) in input.iter().zip(output.iter_mut()).enumerate() {
        if !sample.is_finite() {
            return Err(WavError::NonFiniteSample {
                channel_index: sample_index % channels,
                frame_index: sample_index / channels,
            });
        }
        *destination = sample;
    }

    Ok(())
}

pub(crate) fn float64_to_f32(input: &[f64], output: &mut [f32], channels: usize) -> Result<()> {
    if input.len() != output.len() {
        return Err(WavError::InvalidBufferShape);
    }

    for (sample_index, (&sample, destination)) in input.iter().zip(output.iter_mut()).enumerate() {
        if !sample.is_finite() {
            return Err(WavError::NonFiniteSample {
                channel_index: sample_index % channels,
                frame_index: sample_index / channels,
            });
        }
        #[allow(
            clippy::cast_possible_truncation,
            reason = "Float64 WAV decode narrows into Auralis' public f32 processing buffer."
        )]
        {
            *destination = sample as f32;
        }
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

pub(crate) fn f32_to_pcm24(input: &[f32], output: &mut [i32], channels: usize) -> Result<()> {
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
            reason = "The value is rounded and clamped to the valid PCM24 integer range immediately before the cast."
        )]
        let quantized = (clipped * 8_388_608.0).round() as i64;
        let quantized = quantized.clamp(i64::from(-8_388_608), i64::from(8_388_607));
        *destination = i32::try_from(quantized).map_err(|_| WavError::WriteFailed {
            message: "PCM24 sample quantization overflowed".to_owned(),
        })?;
    }

    Ok(())
}

pub(crate) fn f32_to_pcm32(input: &[f32], output: &mut [i32], channels: usize) -> Result<()> {
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
            reason = "The value is rounded and clamped to the valid PCM32 integer range immediately before the cast."
        )]
        let quantized = (f64::from(clipped) * 2_147_483_648.0).round() as i64;
        let quantized = quantized.clamp(i64::from(i32::MIN), i64::from(i32::MAX));
        *destination = i32::try_from(quantized).map_err(|_| WavError::WriteFailed {
            message: "PCM32 sample quantization overflowed".to_owned(),
        })?;
    }

    Ok(())
}

pub(crate) fn f32_to_float32(input: &[f32], output: &mut [f32], channels: usize) -> Result<()> {
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
        *destination = sample;
    }

    Ok(())
}

pub(crate) fn f32_to_float64(input: &[f32], output: &mut [f64], channels: usize) -> Result<()> {
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
        *destination = f64::from(sample);
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

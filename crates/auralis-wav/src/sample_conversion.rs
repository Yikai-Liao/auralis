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

pub(crate) fn f32_to_pcm16_with_backend(
    requested_backend: BackendKind,
    input: &[f32],
    output: &mut [i16],
    channels: usize,
) -> Result<()> {
    f32_to_i16_with_backend(select_backend(requested_backend), input, output)
        .map_err(|error| sample_conversion_error(&error, channels))
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

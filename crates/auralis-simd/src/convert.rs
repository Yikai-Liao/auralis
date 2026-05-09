use core::fmt;

use crate::{BackendKind, BackendSelection};

/// Errors produced by sample conversion kernels.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum SampleConversionError {
    /// The source and destination buffers did not have the same length.
    BufferLengthMismatch {
        /// Number of input samples.
        input_len: usize,
        /// Number of output samples.
        output_len: usize,
    },

    /// A floating-point input sample was NaN or infinite.
    NonFiniteSample {
        /// Zero-based index of the first non-finite input sample.
        sample_index: usize,
    },
}

impl fmt::Display for SampleConversionError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::BufferLengthMismatch {
                input_len,
                output_len,
            } => write!(
                formatter,
                "sample conversion input length {input_len} did not match output length {output_len}"
            ),
            Self::NonFiniteSample { sample_index } => write!(
                formatter,
                "sample conversion input sample {sample_index} was NaN or infinite"
            ),
        }
    }
}

impl std::error::Error for SampleConversionError {}

/// Converts signed 16-bit PCM samples into normalized `f32` samples using the
/// scalar reference backend.
///
/// Samples are scaled by `1.0 / 32768.0`, so `i16::MIN` maps exactly to
/// `-1.0`, `0` maps to `0.0`, and `i16::MAX` maps to `0.9999695`. The
/// conversion is deterministic and exact for every PCM16 input because the
/// scale denominator is a power of two. The function allocates no memory and
/// accepts empty buffers.
///
/// # Errors
///
/// Returns [`SampleConversionError::BufferLengthMismatch`] when `input` and
/// `output` have different lengths.
///
/// # Examples
///
/// ```
/// let input = [i16::MIN, 0, i16::MAX];
/// let mut output = [0.0; 3];
///
/// auralis_simd::i16_to_f32_scalar(&input, &mut output)?;
///
/// assert_eq!(output[0].to_bits(), (-1.0_f32).to_bits());
/// assert_eq!(output[1].to_bits(), 0.0_f32.to_bits());
/// assert_eq!(output[2].to_bits(), (f32::from(i16::MAX) / 32768.0).to_bits());
/// # Ok::<(), auralis_simd::SampleConversionError>(())
/// ```
pub fn i16_to_f32_scalar(input: &[i16], output: &mut [f32]) -> Result<(), SampleConversionError> {
    validate_conversion_lengths(input.len(), output.len())?;
    i16_to_f32_scalar_unchecked(input, output);
    Ok(())
}

/// Converts signed 16-bit PCM samples into normalized `f32` samples using the
/// backend recorded by `selection`.
///
/// Callers that need deterministic tests can pass the result of
/// [`select_backend`] with either [`BackendKind::Scalar`] or
/// [`BackendKind::Simd`]. When a SIMD request falls back to scalar, this
/// function follows the selected backend recorded in the selection metadata.
/// The numerical mapping is identical to [`i16_to_f32_scalar`].
///
/// # Errors
///
/// Returns [`SampleConversionError::BufferLengthMismatch`] when `input` and
/// `output` have different lengths.
pub fn i16_to_f32_with_backend(
    selection: BackendSelection,
    input: &[i16],
    output: &mut [f32],
) -> Result<(), SampleConversionError> {
    validate_conversion_lengths(input.len(), output.len())?;

    match selection.selected_kind() {
        BackendKind::Scalar => i16_to_f32_scalar_unchecked(input, output),
        BackendKind::Simd => i16_to_f32_selected_simd(input, output),
    }

    Ok(())
}

/// Converts normalized `f32` samples into signed 16-bit PCM using the scalar
/// reference backend.
///
/// Samples must be finite. Each sample is clipped to `[-1.0, 1.0]`, scaled by
/// `32768.0`, rounded to the nearest integer with halfway cases rounded away
/// from zero, then clipped to the `i16` range. This maps `-1.0` to
/// [`i16::MIN`], `1.0` to [`i16::MAX`], and values that came from
/// [`i16_to_f32_scalar`] back to their original PCM16 value where possible.
/// The function allocates no memory and accepts empty buffers.
///
/// # Errors
///
/// Returns [`SampleConversionError::BufferLengthMismatch`] when `input` and
/// `output` have different lengths. Returns
/// [`SampleConversionError::NonFiniteSample`] for the first NaN or infinite
/// input sample.
///
/// # Examples
///
/// ```
/// let input = [-1.0, 0.0, 0.5, 1.0];
/// let mut output = [0; 4];
///
/// auralis_simd::f32_to_i16_scalar(&input, &mut output)?;
///
/// assert_eq!(output, [i16::MIN, 0, 16_384, i16::MAX]);
/// # Ok::<(), auralis_simd::SampleConversionError>(())
/// ```
pub fn f32_to_i16_scalar(input: &[f32], output: &mut [i16]) -> Result<(), SampleConversionError> {
    validate_conversion_lengths(input.len(), output.len())?;
    validate_finite_samples(input)?;
    f32_to_i16_scalar_unchecked(input, output);
    Ok(())
}

/// Converts normalized `f32` samples into signed 16-bit PCM using the backend
/// recorded by `selection`.
///
/// Callers that need deterministic tests can pass the result of
/// [`select_backend`] with either [`BackendKind::Scalar`] or
/// [`BackendKind::Simd`]. When a SIMD request falls back to scalar, this
/// function follows the selected backend recorded in the selection metadata.
/// The clipping, rounding, and NaN/infinity behavior are identical to
/// [`f32_to_i16_scalar`].
///
/// # Errors
///
/// Returns [`SampleConversionError::BufferLengthMismatch`] when `input` and
/// `output` have different lengths. Returns
/// [`SampleConversionError::NonFiniteSample`] for the first NaN or infinite
/// input sample.
pub fn f32_to_i16_with_backend(
    selection: BackendSelection,
    input: &[f32],
    output: &mut [i16],
) -> Result<(), SampleConversionError> {
    validate_conversion_lengths(input.len(), output.len())?;
    validate_finite_samples(input)?;

    match selection.selected_kind() {
        BackendKind::Scalar => f32_to_i16_scalar_unchecked(input, output),
        BackendKind::Simd => f32_to_i16_selected_simd(input, output),
    }

    Ok(())
}

const PCM16_TO_F32_SCALE: f32 = 1.0 / 32768.0;
const F32_TO_PCM16_SCALE: f32 = 32768.0;

fn validate_conversion_lengths(
    input_len: usize,
    output_len: usize,
) -> Result<(), SampleConversionError> {
    if input_len == output_len {
        Ok(())
    } else {
        Err(SampleConversionError::BufferLengthMismatch {
            input_len,
            output_len,
        })
    }
}

fn validate_finite_samples(input: &[f32]) -> Result<(), SampleConversionError> {
    for (sample_index, sample) in input.iter().enumerate() {
        if !sample.is_finite() {
            return Err(SampleConversionError::NonFiniteSample { sample_index });
        }
    }

    Ok(())
}

#[inline]
fn i16_to_f32_scalar_unchecked(input: &[i16], output: &mut [f32]) {
    for (&input, output) in input.iter().zip(output) {
        *output = f32::from(input) * PCM16_TO_F32_SCALE;
    }
}

#[inline]
fn f32_to_i16_scalar_unchecked(input: &[f32], output: &mut [i16]) {
    for (&input, output) in input.iter().zip(output) {
        *output = f32_to_i16_scalar_sample(input);
    }
}

#[allow(
    clippy::cast_possible_truncation,
    reason = "the sample is rounded and clamped to the i16 range before casting"
)]
#[inline]
fn f32_to_i16_scalar_sample(sample: f32) -> i16 {
    let scaled = (sample.clamp(-1.0, 1.0) * F32_TO_PCM16_SCALE)
        .round()
        .clamp(f32::from(i16::MIN), f32::from(i16::MAX));

    scaled as i16
}

#[cfg(not(feature = "simd"))]
fn i16_to_f32_selected_simd(input: &[i16], output: &mut [f32]) {
    i16_to_f32_scalar_unchecked(input, output);
}

#[cfg(not(feature = "simd"))]
fn f32_to_i16_selected_simd(input: &[f32], output: &mut [i16]) {
    f32_to_i16_scalar_unchecked(input, output);
}

#[cfg(feature = "simd")]
fn i16_to_f32_selected_simd(input: &[i16], output: &mut [f32]) {
    use rten_simd::{
        Isa, SimdOp,
        ops::{Extend, NumOps, ToFloat},
    };

    struct Convert<'input, 'output> {
        input: &'input [i16],
        output: &'output mut [f32],
    }

    impl SimdOp for Convert<'_, '_> {
        type Output = ();

        #[expect(
            clippy::inline_always,
            reason = "rten-simd recommends inlining eval so target-feature intrinsics compile into the dispatched kernel body"
        )]
        #[inline(always)]
        fn eval<I: Isa>(self, isa: I) -> Self::Output {
            let i16_ops = isa.i16();
            let i32_ops = isa.i32();
            let f32_ops = isa.f32();
            let scale = f32_ops.splat(PCM16_TO_F32_SCALE);
            let input_vector_len = i16_ops.len();
            let output_half_vector_len = i32_ops.len();

            let mut input_chunks = self.input.chunks_exact(input_vector_len);
            let mut output_chunks = self.output.chunks_exact_mut(input_vector_len);

            for (input_chunk, output_chunk) in input_chunks.by_ref().zip(output_chunks.by_ref()) {
                let input_i16 = i16_ops.load(input_chunk);
                let (low_extended, high_extended) = i16_ops.extend(input_i16);
                let low_scaled = f32_ops.mul(i32_ops.to_float(low_extended), scale);
                let high_scaled = f32_ops.mul(i32_ops.to_float(high_extended), scale);
                let (low_output, high_output) = output_chunk.split_at_mut(output_half_vector_len);

                f32_ops.store(low_scaled, low_output);
                f32_ops.store(high_scaled, high_output);
            }

            i16_to_f32_scalar_unchecked(input_chunks.remainder(), output_chunks.into_remainder());
        }
    }

    Convert { input, output }.dispatch();
}

#[cfg(feature = "simd")]
fn f32_to_i16_selected_simd(input: &[f32], output: &mut [i16]) {
    use rten_simd::{
        Isa, SimdOp,
        ops::{FloatOps, NarrowSaturate, NumOps},
    };

    struct Convert<'input, 'output> {
        input: &'input [f32],
        output: &'output mut [i16],
    }

    impl SimdOp for Convert<'_, '_> {
        type Output = ();

        #[expect(
            clippy::inline_always,
            reason = "rten-simd recommends inlining eval so target-feature intrinsics compile into the dispatched kernel body"
        )]
        #[inline(always)]
        fn eval<I: Isa>(self, isa: I) -> Self::Output {
            let f32_ops = isa.f32();
            let i32_ops = isa.i32();
            let i16_ops = isa.i16();
            let f32_vector_len = f32_ops.len();
            let output_vector_len = i16_ops.len();
            let min_sample = f32_ops.splat(-1.0);
            let max_sample = f32_ops.splat(1.0);
            let scale = f32_ops.splat(F32_TO_PCM16_SCALE);
            let zero = f32_ops.zero();
            let positive_round_offset = f32_ops.splat(0.5);
            let negative_round_offset = f32_ops.splat(-0.5);

            let mut input_chunks = self.input.chunks_exact(output_vector_len);
            let mut output_chunks = self.output.chunks_exact_mut(output_vector_len);

            for (input_chunk, output_chunk) in input_chunks.by_ref().zip(output_chunks.by_ref()) {
                let (low_input, high_input) = input_chunk.split_at(f32_vector_len);
                let low_scaled = f32_ops.mul(
                    f32_ops.clamp(f32_ops.load(low_input), min_sample, max_sample),
                    scale,
                );
                let high_scaled = f32_ops.mul(
                    f32_ops.clamp(f32_ops.load(high_input), min_sample, max_sample),
                    scale,
                );
                let low_round_offset = f32_ops.select(
                    positive_round_offset,
                    negative_round_offset,
                    f32_ops.ge(low_scaled, zero),
                );
                let high_round_offset = f32_ops.select(
                    positive_round_offset,
                    negative_round_offset,
                    f32_ops.ge(high_scaled, zero),
                );
                let low_i32 = f32_ops.to_int_trunc(f32_ops.add(low_scaled, low_round_offset));
                let high_i32 = f32_ops.to_int_trunc(f32_ops.add(high_scaled, high_round_offset));
                let packed = i32_ops.narrow_saturate(low_i32, high_i32);

                i16_ops.store(packed, output_chunk);
            }

            f32_to_i16_scalar_unchecked(input_chunks.remainder(), output_chunks.into_remainder());
        }
    }

    Convert { input, output }.dispatch();
}

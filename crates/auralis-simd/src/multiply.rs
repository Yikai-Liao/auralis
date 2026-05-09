use core::fmt;

use crate::{BackendKind, BackendSelection};

/// Errors produced by sample multiplication kernels.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum MultiplyError {
    /// One input slice was longer than the output slice it would multiply into.
    InputLongerThanOutput {
        /// Zero-based input slice index.
        input_index: usize,

        /// Number of samples in the input slice.
        input_len: usize,

        /// Number of samples in the output slice.
        output_len: usize,
    },
}

impl fmt::Display for MultiplyError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InputLongerThanOutput {
                input_index,
                input_len,
                output_len,
            } => write!(
                formatter,
                "multiply input {input_index} length {input_len} exceeds output length {output_len}"
            ),
        }
    }
}

impl std::error::Error for MultiplyError {}

/// Multiplies input sample slices into `output` using the scalar reference backend.
///
/// For each output index, the kernel multiplies the corresponding sample from
/// every input. Missing tail samples are treated as silence, so if any input is
/// shorter than `output`, the missing tail becomes `0.0`. A single input is an
/// identity copy for its covered samples. With no inputs, `output` is reset to
/// silence. The operation is deterministic, accepts empty input and output
/// slices, and does not clip, normalize, or validate sample values. NaN and
/// infinity inputs retain ordinary floating-point multiplication semantics.
///
/// # Errors
///
/// Returns [`MultiplyError::InputLongerThanOutput`] when an input slice is
/// longer than `output`.
///
/// # Examples
///
/// ```
/// let first = [0.5, -0.5, 1.0];
/// let second = [0.25, 1.0];
/// let mut output = [9.0; 3];
///
/// auralis_simd::multiply_f32_scalar(&[&first, &second], &mut output)?;
///
/// assert_eq!(output, [0.125, -0.5, 0.0]);
/// # Ok::<(), auralis_simd::MultiplyError>(())
/// ```
pub fn multiply_f32_scalar(inputs: &[&[f32]], output: &mut [f32]) -> Result<(), MultiplyError> {
    validate_multiply_lengths(inputs, output.len())?;
    multiply_f32_scalar_unchecked(inputs, output);
    Ok(())
}

/// Multiplies input sample slices into `output` using the backend recorded by `selection`.
///
/// Callers that need deterministic tests can pass the result of
/// [`select_backend`] with either [`BackendKind::Scalar`] or
/// [`BackendKind::Simd`]. When a SIMD request falls back to scalar, this
/// function follows the selected backend recorded in the selection metadata.
/// Numerical behavior is identical to [`multiply_f32_scalar`].
///
/// # Errors
///
/// Returns [`MultiplyError::InputLongerThanOutput`] when an input slice is
/// longer than `output`.
pub fn multiply_f32_with_backend(
    selection: BackendSelection,
    inputs: &[&[f32]],
    output: &mut [f32],
) -> Result<(), MultiplyError> {
    validate_multiply_lengths(inputs, output.len())?;

    match selection.selected_kind() {
        BackendKind::Scalar => multiply_f32_scalar_unchecked(inputs, output),
        BackendKind::Simd => multiply_f32_selected_simd(inputs, output),
    }

    Ok(())
}

fn validate_multiply_lengths(inputs: &[&[f32]], output_len: usize) -> Result<(), MultiplyError> {
    for (input_index, input) in inputs.iter().enumerate() {
        if input.len() > output_len {
            return Err(MultiplyError::InputLongerThanOutput {
                input_index,
                input_len: input.len(),
                output_len,
            });
        }
    }

    Ok(())
}

#[inline]
fn multiply_f32_scalar_unchecked(inputs: &[&[f32]], output: &mut [f32]) {
    if inputs.is_empty() {
        output.fill(0.0);
        return;
    }

    output.fill(1.0);
    let mut active_len = output.len();

    for input in inputs {
        active_len = active_len.min(input.len());
        for (output, &input) in output[..active_len]
            .iter_mut()
            .zip(input[..active_len].iter())
        {
            *output *= input;
        }
        output[active_len..].fill(0.0);
    }
}

#[cfg(not(feature = "simd"))]
fn multiply_f32_selected_simd(inputs: &[&[f32]], output: &mut [f32]) {
    multiply_f32_scalar_unchecked(inputs, output);
}

#[cfg(feature = "simd")]
fn multiply_f32_selected_simd(inputs: &[&[f32]], output: &mut [f32]) {
    use rten_simd::{Isa, SimdOp, ops::NumOps};

    struct Multiply<'inputs, 'samples, 'output> {
        inputs: &'inputs [&'samples [f32]],
        output: &'output mut [f32],
    }

    impl SimdOp for Multiply<'_, '_, '_> {
        type Output = ();

        #[expect(
            clippy::inline_always,
            reason = "rten-simd recommends inlining eval so target-feature intrinsics compile into the dispatched kernel body"
        )]
        #[inline(always)]
        fn eval<I: Isa>(self, isa: I) -> Self::Output {
            let f32_ops = isa.f32();
            let vector_len = f32_ops.len();

            if self.inputs.is_empty() {
                self.output.fill(0.0);
                return;
            }

            self.output.fill(1.0);
            let mut active_len = self.output.len();

            for input in self.inputs {
                active_len = active_len.min(input.len());
                let mut input_chunks = input[..active_len].chunks_exact(vector_len);
                let mut output_chunks = self.output[..active_len].chunks_exact_mut(vector_len);

                for (input_chunk, output_chunk) in input_chunks.by_ref().zip(output_chunks.by_ref())
                {
                    let product =
                        f32_ops.mul(f32_ops.load(output_chunk), f32_ops.load(input_chunk));

                    f32_ops.store(product, output_chunk);
                }

                for (output, &input) in output_chunks
                    .into_remainder()
                    .iter_mut()
                    .zip(input_chunks.remainder())
                {
                    *output *= input;
                }

                self.output[active_len..].fill(0.0);
            }
        }
    }

    Multiply { inputs, output }.dispatch();
}

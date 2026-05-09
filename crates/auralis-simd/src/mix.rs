use core::fmt;

use crate::{BackendKind, BackendSelection};

/// Errors produced by sample mixing kernels.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum MixError {
    /// One input slice was longer than the output slice it would mix into.
    InputLongerThanOutput {
        /// Zero-based input slice index.
        input_index: usize,

        /// Number of samples in the input slice.
        input_len: usize,

        /// Number of samples in the output slice.
        output_len: usize,
    },
}

impl fmt::Display for MixError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InputLongerThanOutput {
                input_index,
                input_len,
                output_len,
            } => write!(
                formatter,
                "mix input {input_index} length {input_len} exceeds output length {output_len}"
            ),
        }
    }
}

impl std::error::Error for MixError {}

/// Mixes input sample slices into `output` using the scalar reference backend.
///
/// `output` is reset to silence before accumulation. For each input sample at
/// an output index, the kernel adds `sample * scale`; when an input is shorter
/// than `output`, its missing tail is treated as silence. The operation is
/// deterministic, accepts empty input and output slices, and does not clip,
/// normalize, or validate sample values. NaN and infinity inputs retain
/// ordinary floating-point multiplication and addition semantics.
///
/// # Errors
///
/// Returns [`MixError::InputLongerThanOutput`] when an input slice is longer
/// than `output`.
///
/// # Examples
///
/// ```
/// let first = [1.0, -1.0, 0.5];
/// let second = [0.0, 0.5];
/// let mut output = [9.0; 3];
///
/// auralis_simd::mix_f32_scalar(&[&first, &second], &mut output, 0.5)?;
///
/// assert_eq!(output, [0.5, -0.25, 0.25]);
/// # Ok::<(), auralis_simd::MixError>(())
/// ```
pub fn mix_f32_scalar(inputs: &[&[f32]], output: &mut [f32], scale: f32) -> Result<(), MixError> {
    validate_mix_lengths(inputs, output.len())?;
    mix_f32_scalar_unchecked(inputs, output, scale);
    Ok(())
}

/// Mixes input sample slices into `output` using the backend recorded by `selection`.
///
/// Callers that need deterministic tests can pass the result of
/// [`select_backend`] with either [`BackendKind::Scalar`] or
/// [`BackendKind::Simd`]. When a SIMD request falls back to scalar, this
/// function follows the selected backend recorded in the selection metadata.
/// Numerical behavior is identical to [`mix_f32_scalar`].
///
/// # Errors
///
/// Returns [`MixError::InputLongerThanOutput`] when an input slice is longer
/// than `output`.
pub fn mix_f32_with_backend(
    selection: BackendSelection,
    inputs: &[&[f32]],
    output: &mut [f32],
    scale: f32,
) -> Result<(), MixError> {
    validate_mix_lengths(inputs, output.len())?;

    match selection.selected_kind() {
        BackendKind::Scalar => mix_f32_scalar_unchecked(inputs, output, scale),
        BackendKind::Simd => mix_f32_selected_simd(inputs, output, scale),
    }

    Ok(())
}

fn validate_mix_lengths(inputs: &[&[f32]], output_len: usize) -> Result<(), MixError> {
    for (input_index, input) in inputs.iter().enumerate() {
        if input.len() > output_len {
            return Err(MixError::InputLongerThanOutput {
                input_index,
                input_len: input.len(),
                output_len,
            });
        }
    }

    Ok(())
}

#[inline]
fn mix_f32_scalar_unchecked(inputs: &[&[f32]], output: &mut [f32], scale: f32) {
    output.fill(0.0);

    for input in inputs {
        for (output, &input) in output.iter_mut().zip(*input) {
            *output += input * scale;
        }
    }
}

#[cfg(not(feature = "simd"))]
fn mix_f32_selected_simd(inputs: &[&[f32]], output: &mut [f32], scale: f32) {
    mix_f32_scalar_unchecked(inputs, output, scale);
}

#[cfg(feature = "simd")]
fn mix_f32_selected_simd(inputs: &[&[f32]], output: &mut [f32], scale: f32) {
    use rten_simd::{Isa, SimdOp, ops::NumOps};

    struct Mix<'inputs, 'samples, 'output> {
        inputs: &'inputs [&'samples [f32]],
        output: &'output mut [f32],
        scale: f32,
    }

    impl SimdOp for Mix<'_, '_, '_> {
        type Output = ();

        #[expect(
            clippy::inline_always,
            reason = "rten-simd recommends inlining eval so target-feature intrinsics compile into the dispatched kernel body"
        )]
        #[inline(always)]
        fn eval<I: Isa>(self, isa: I) -> Self::Output {
            let f32_ops = isa.f32();
            let scale = f32_ops.splat(self.scale);
            let vector_len = f32_ops.len();

            self.output.fill(0.0);

            for input in self.inputs {
                let mut input_chunks = input.chunks_exact(vector_len);
                let mut output_chunks = self.output[..input.len()].chunks_exact_mut(vector_len);

                for (input_chunk, output_chunk) in input_chunks.by_ref().zip(output_chunks.by_ref())
                {
                    let mixed = f32_ops.add(
                        f32_ops.load(output_chunk),
                        f32_ops.mul(f32_ops.load(input_chunk), scale),
                    );

                    f32_ops.store(mixed, output_chunk);
                }

                for (output, &input) in output_chunks
                    .into_remainder()
                    .iter_mut()
                    .zip(input_chunks.remainder())
                {
                    *output += input * self.scale;
                }
            }
        }
    }

    Mix {
        inputs,
        output,
        scale,
    }
    .dispatch();
}

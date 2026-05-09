use crate::{BackendKind, BackendSelection};

/// Adds a constant normalized full-scale offset to `samples` using the scalar
/// reference backend.
///
/// The operation is deterministic, in-place, non-allocating, and accepts empty
/// buffers. It does not clip, normalize, or validate samples. Finite samples
/// with a finite `shift` follow ordinary IEEE `f32` addition. NaN and infinity
/// inputs retain the usual floating-point addition semantics.
///
/// # Examples
///
/// ```
/// let mut samples = [-0.5, 0.0, 0.5];
///
/// auralis_simd::dc_shift_f32_in_place_scalar(&mut samples, 0.25);
///
/// assert_eq!(samples, [-0.25, 0.25, 0.75]);
/// ```
pub fn dc_shift_f32_in_place_scalar(samples: &mut [f32], shift: f32) {
    dc_shift_f32_scalar_unchecked(samples, shift);
}

/// Adds a constant normalized full-scale offset to `samples` using the backend
/// recorded by `selection`.
///
/// Callers that need deterministic tests can pass the result of
/// [`select_backend`] with either [`BackendKind::Scalar`] or
/// [`BackendKind::Simd`]. When a SIMD request falls back to scalar, this
/// function follows the selected backend recorded in the selection metadata.
/// Numerical behavior is identical to [`dc_shift_f32_in_place_scalar`].
pub fn dc_shift_f32_in_place_with_backend(
    selection: BackendSelection,
    samples: &mut [f32],
    shift: f32,
) {
    match selection.selected_kind() {
        BackendKind::Scalar => dc_shift_f32_scalar_unchecked(samples, shift),
        BackendKind::Simd => dc_shift_f32_selected_simd(samples, shift),
    }
}

#[inline]
fn dc_shift_f32_scalar_unchecked(samples: &mut [f32], shift: f32) {
    for sample in samples {
        *sample += shift;
    }
}

#[cfg(not(feature = "simd"))]
fn dc_shift_f32_selected_simd(samples: &mut [f32], shift: f32) {
    dc_shift_f32_scalar_unchecked(samples, shift);
}

#[cfg(feature = "simd")]
fn dc_shift_f32_selected_simd(samples: &mut [f32], shift: f32) {
    use rten_simd::{Isa, SimdOp, ops::NumOps};

    struct ApplyDcShift<'samples> {
        samples: &'samples mut [f32],
        shift: f32,
    }

    impl SimdOp for ApplyDcShift<'_> {
        type Output = ();

        #[expect(
            clippy::inline_always,
            reason = "rten-simd recommends inlining eval so target-feature intrinsics compile into the dispatched kernel body"
        )]
        #[inline(always)]
        fn eval<I: Isa>(self, isa: I) -> Self::Output {
            let f32_ops = isa.f32();
            let shift = f32_ops.splat(self.shift);
            let vector_len = f32_ops.len();
            let mut chunks = self.samples.chunks_exact_mut(vector_len);

            for chunk in chunks.by_ref() {
                let shifted = f32_ops.add(f32_ops.load(chunk), shift);

                f32_ops.store(shifted, chunk);
            }

            dc_shift_f32_scalar_unchecked(chunks.into_remainder(), self.shift);
        }
    }

    ApplyDcShift { samples, shift }.dispatch();
}

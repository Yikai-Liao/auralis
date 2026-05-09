use crate::{BackendKind, BackendSelection};

/// Applies a linear gain multiplier to `samples` using the scalar reference backend.
///
/// `multiplier` is a linear amplitude scale, not a decibel value. The operation
/// is deterministic, in-place, non-allocating, and accepts empty buffers. It
/// does not clip, normalize, or validate samples. Finite samples with a finite
/// multiplier follow ordinary IEEE `f32` multiplication. An infinite multiplier
/// maps non-zero finite samples to signed infinity while preserving positive
/// and negative zero, so finite input samples do not become `NaN`.
///
/// # Examples
///
/// ```
/// let mut samples = [0.25, -0.5, 1.0];
///
/// auralis_simd::gain_f32_in_place_scalar(&mut samples, 2.0);
///
/// assert_eq!(samples, [0.5, -1.0, 2.0]);
/// ```
pub fn gain_f32_in_place_scalar(samples: &mut [f32], multiplier: f32) {
    gain_f32_scalar_unchecked(samples, multiplier);
}

/// Applies a linear gain multiplier to `samples` using the backend recorded by `selection`.
///
/// Callers that need deterministic tests can pass the result of
/// [`select_backend`] with either [`BackendKind::Scalar`] or
/// [`BackendKind::Simd`]. When a SIMD request falls back to scalar, this
/// function follows the selected backend recorded in the selection metadata.
/// Numerical behavior is identical to [`gain_f32_in_place_scalar`].
pub fn gain_f32_in_place_with_backend(
    selection: BackendSelection,
    samples: &mut [f32],
    multiplier: f32,
) {
    if multiplier.is_infinite() {
        gain_f32_scalar_unchecked(samples, multiplier);
        return;
    }

    match selection.selected_kind() {
        BackendKind::Scalar => gain_f32_scalar_unchecked(samples, multiplier),
        BackendKind::Simd => gain_f32_selected_simd(samples, multiplier),
    }
}

#[inline]
fn gain_f32_scalar_unchecked(samples: &mut [f32], multiplier: f32) {
    if multiplier.is_infinite() {
        let infinity = if multiplier.is_sign_negative() {
            f32::NEG_INFINITY
        } else {
            f32::INFINITY
        };

        for sample in samples {
            if *sample != 0.0 {
                *sample = sample.signum() * infinity;
            }
        }
        return;
    }

    for sample in samples {
        *sample *= multiplier;
    }
}

#[cfg(not(feature = "simd"))]
fn gain_f32_selected_simd(samples: &mut [f32], multiplier: f32) {
    gain_f32_scalar_unchecked(samples, multiplier);
}

#[cfg(feature = "simd")]
fn gain_f32_selected_simd(samples: &mut [f32], multiplier: f32) {
    use rten_simd::{Isa, SimdOp, ops::NumOps};

    struct ApplyGain<'samples> {
        samples: &'samples mut [f32],
        multiplier: f32,
    }

    impl SimdOp for ApplyGain<'_> {
        type Output = ();

        #[expect(
            clippy::inline_always,
            reason = "rten-simd recommends inlining eval so target-feature intrinsics compile into the dispatched kernel body"
        )]
        #[inline(always)]
        fn eval<I: Isa>(self, isa: I) -> Self::Output {
            let f32_ops = isa.f32();
            let multiplier = f32_ops.splat(self.multiplier);
            let vector_len = f32_ops.len();
            let mut chunks = self.samples.chunks_exact_mut(vector_len);

            for chunk in chunks.by_ref() {
                let scaled = f32_ops.mul(f32_ops.load(chunk), multiplier);

                f32_ops.store(scaled, chunk);
            }

            gain_f32_scalar_unchecked(chunks.into_remainder(), self.multiplier);
        }
    }

    ApplyGain {
        samples,
        multiplier,
    }
    .dispatch();
}

use crate::{BackendKind, BackendSelection};

/// Applies a linear fade envelope to a channel segment using the scalar reference backend.
///
/// `total_frames` is the full channel length, `start_frame` is the frame index
/// of `samples[0]`, and `fade_in` / `fade_out` are frame lengths. A fade-in
/// length of `4` uses coefficients `[0.0, 0.25, 0.5, 0.75]`; a fade-out length
/// of `4` applies `[0.75, 0.5, 0.25, 0.0]` to the final four frames. When the
/// fade regions overlap, their coefficients are multiplied. Empty buffers and
/// zero-length fades are accepted.
///
/// The operation is deterministic, in-place, non-allocating, and does not clip
/// or validate samples. NaN and infinity inputs retain ordinary floating-point
/// multiplication semantics.
///
/// # Examples
///
/// ```
/// let mut samples = [1.0; 4];
///
/// auralis_simd::fade_f32_in_place_scalar(&mut samples, 4, 0, 2, 2);
///
/// assert_eq!(samples, [0.0, 0.5, 0.5, 0.0]);
/// ```
pub fn fade_f32_in_place_scalar(
    samples: &mut [f32],
    total_frames: u64,
    start_frame: u64,
    fade_in: u64,
    fade_out: u64,
) {
    fade_f32_scalar_unchecked(samples, total_frames, start_frame, fade_in, fade_out);
}

/// Applies a linear fade envelope to a channel segment using the backend
/// recorded by `selection`.
///
/// Callers that need deterministic tests can pass the result of
/// [`select_backend`] with either [`BackendKind::Scalar`] or
/// [`BackendKind::Simd`]. When a SIMD request falls back to scalar, this
/// function follows the selected backend recorded in the selection metadata.
/// Numerical behavior is identical to [`fade_f32_in_place_scalar`].
pub fn fade_f32_in_place_with_backend(
    selection: BackendSelection,
    samples: &mut [f32],
    total_frames: u64,
    start_frame: u64,
    fade_in: u64,
    fade_out: u64,
) {
    match selection.selected_kind() {
        BackendKind::Scalar => {
            fade_f32_scalar_unchecked(samples, total_frames, start_frame, fade_in, fade_out);
        }
        BackendKind::Simd => {
            fade_f32_selected_simd(samples, total_frames, start_frame, fade_in, fade_out);
        }
    }
}

#[inline]
fn fade_f32_scalar_unchecked(
    samples: &mut [f32],
    total_frames: u64,
    start_frame: u64,
    fade_in: u64,
    fade_out: u64,
) {
    for (offset, sample) in samples.iter_mut().enumerate() {
        let Ok(offset) = u64::try_from(offset) else {
            return;
        };
        let Some(frame_index) = start_frame.checked_add(offset) else {
            return;
        };
        *sample *= fade_coefficient(frame_index, total_frames, fade_in, fade_out);
    }
}

fn fade_coefficient(frame_index: u64, total_frames: u64, fade_in: u64, fade_out: u64) -> f32 {
    let mut coefficient = 1.0;

    if fade_in != 0 && frame_index < fade_in {
        coefficient *= ratio(frame_index, fade_in);
    }

    if fade_out != 0 && frame_index < total_frames {
        let remaining = total_frames - frame_index - 1;
        if remaining < fade_out {
            coefficient *= ratio(remaining, fade_out);
        }
    }

    coefficient
}

fn ratio(numerator: u64, denominator: u64) -> f32 {
    #[allow(
        clippy::cast_precision_loss,
        reason = "Fade coefficients are applied at the f32 sample boundary; exact integer precision above f32 mantissa range is not meaningful for audio buffers."
    )]
    {
        numerator as f32 / denominator as f32
    }
}

#[cfg(not(feature = "simd"))]
fn fade_f32_selected_simd(
    samples: &mut [f32],
    total_frames: u64,
    start_frame: u64,
    fade_in: u64,
    fade_out: u64,
) {
    fade_f32_scalar_unchecked(samples, total_frames, start_frame, fade_in, fade_out);
}

#[cfg(feature = "simd")]
fn fade_f32_selected_simd(
    samples: &mut [f32],
    total_frames: u64,
    start_frame: u64,
    fade_in: u64,
    fade_out: u64,
) {
    use rten_simd::{Isa, SimdOp, ops::NumOps};

    const MAX_F32_VECTOR_LANES: usize = 16;

    struct ApplyFade<'samples> {
        samples: &'samples mut [f32],
        total_frames: u64,
        start_frame: u64,
        fade_in: u64,
        fade_out: u64,
    }

    impl SimdOp for ApplyFade<'_> {
        type Output = ();

        #[expect(
            clippy::inline_always,
            reason = "rten-simd recommends inlining eval so target-feature intrinsics compile into the dispatched kernel body"
        )]
        #[inline(always)]
        fn eval<I: Isa>(self, isa: I) -> Self::Output {
            let f32_ops = isa.f32();
            let vector_len = f32_ops.len();

            if vector_len == 0 || vector_len > MAX_F32_VECTOR_LANES {
                fade_f32_scalar_unchecked(
                    self.samples,
                    self.total_frames,
                    self.start_frame,
                    self.fade_in,
                    self.fade_out,
                );
                return;
            }

            let Ok(vector_len_u64) = u64::try_from(vector_len) else {
                fade_f32_scalar_unchecked(
                    self.samples,
                    self.total_frames,
                    self.start_frame,
                    self.fade_in,
                    self.fade_out,
                );
                return;
            };
            let last_lane = vector_len_u64 - 1;
            let mut chunks = self.samples.chunks_exact_mut(vector_len);
            let mut chunk_start_frame = self.start_frame;

            for chunk in chunks.by_ref() {
                if chunk_start_frame.checked_add(last_lane).is_none() {
                    fade_f32_scalar_unchecked(
                        chunk,
                        self.total_frames,
                        chunk_start_frame,
                        self.fade_in,
                        self.fade_out,
                    );
                    return;
                }

                let mut coefficients = [0.0; MAX_F32_VECTOR_LANES];
                for (lane, coefficient) in coefficients[..vector_len].iter_mut().enumerate() {
                    let Ok(lane) = u64::try_from(lane) else {
                        return;
                    };
                    *coefficient = fade_coefficient(
                        chunk_start_frame + lane,
                        self.total_frames,
                        self.fade_in,
                        self.fade_out,
                    );
                }

                let faded = f32_ops.mul(f32_ops.load(chunk), f32_ops.load(&coefficients));

                f32_ops.store(faded, chunk);

                let Some(next_start_frame) = chunk_start_frame.checked_add(vector_len_u64) else {
                    return;
                };
                chunk_start_frame = next_start_frame;
            }

            fade_f32_scalar_unchecked(
                chunks.into_remainder(),
                self.total_frames,
                chunk_start_frame,
                self.fade_in,
                self.fade_out,
            );
        }
    }

    ApplyFade {
        samples,
        total_frames,
        start_frame,
        fade_in,
        fade_out,
    }
    .dispatch();
}

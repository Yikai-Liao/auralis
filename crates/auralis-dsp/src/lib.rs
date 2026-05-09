//! DSP kernels for Auralis.
//!
//! The kernels in this crate operate on planar `f32` sample slices and avoid
//! format-specific assumptions. They are deterministic scalar references for
//! later effect processors and optimized backends.
//!
//! # Examples
//!
//! ```
//! use auralis_core::Decibels;
//! use auralis_dsp::{dc_shift_in_place, fade_in_place, gain_in_place};
//!
//! let mut samples = [0.25, -0.5, 1.0];
//! gain_in_place(&mut samples, Decibels::new(6.0)?);
//! dc_shift_in_place(&mut samples, -0.25);
//! fade_in_place(&mut samples, 3, 0, 3, 0);
//!
//! assert_eq!(samples[0], 0.0);
//! # Ok::<(), auralis_core::AuralisError>(())
//! ```

use auralis_core::Decibels;
use auralis_simd::{
    BackendKind, BackendSelection, dc_shift_f32_in_place_with_backend,
    gain_f32_in_place_with_backend, select_backend,
};

/// Applies constant gain to each sample in place.
///
/// `db` is measured in decibels, where `0 dB` is identity and the linear
/// multiplier is `10^(db / 20)`. The kernel does not clip or normalize; full
/// scale inputs may exceed `[-1.0, 1.0]` after positive gain, and very large
/// gains may produce signed infinity. Finite input samples never become `NaN`.
///
/// The operation is deterministic and streaming-safe because each sample is
/// transformed independently. It allocates no memory and runs in `O(n)` time.
#[inline]
pub fn gain_in_place(samples: &mut [f32], db: Decibels) {
    gain_in_place_with_backend(select_backend(BackendKind::Scalar), samples, db);
}

/// Applies constant gain to each sample in place using the selected backend.
///
/// `db` uses the same decibel-to-linear conversion and numerical behavior as
/// [`gain_in_place`]. Callers can pass [`auralis_simd::select_backend`] with
/// either [`BackendKind::Scalar`] or [`BackendKind::Simd`] to force deterministic
/// scalar-vs-SIMD conformance runs. Unsupported SIMD requests follow the
/// fallback recorded in `selection`.
#[inline]
pub fn gain_in_place_with_backend(selection: BackendSelection, samples: &mut [f32], db: Decibels) {
    let multiplier = linear_gain(db);

    gain_f32_in_place_with_backend(selection, samples, multiplier);
}

/// Returns the linear amplitude multiplier for a decibel value.
///
/// This helper is exposed so effect processors and tests can use the same
/// reference conversion as [`gain_in_place`].
#[must_use]
#[inline]
#[allow(
    clippy::cast_possible_truncation,
    reason = "the public DSP sample format is f32, so the f64 unit value is intentionally rounded once"
)]
pub fn linear_gain(db: Decibels) -> f32 {
    let multiplier = 10.0_f64.powf(db.as_f64() / 20.0);

    if multiplier > f64::from(f32::MAX) {
        f32::INFINITY
    } else {
        multiplier as f32
    }
}

/// Adds a constant normalized DC offset to each sample in place.
///
/// `shift` is measured in full-scale sample units, so `0.25` adds one quarter
/// of full scale to every sample and `0.0` is identity. This scalar reference
/// kernel does not clip or normalize; samples may exceed `[-1.0, 1.0]` and are
/// clipped later by boundary writers such as PCM16 WAV encoding. Finite input
/// samples and a finite shift never produce `NaN`.
///
/// The operation is deterministic and streaming-safe because each sample is
/// transformed independently. It allocates no memory and runs in `O(n)` time.
#[inline]
pub fn dc_shift_in_place(samples: &mut [f32], shift: f32) {
    dc_shift_in_place_with_backend(select_backend(BackendKind::Scalar), samples, shift);
}

/// Adds a constant normalized DC offset to each sample in place using the
/// selected backend.
///
/// `shift` uses the same normalized full-scale units and numerical behavior as
/// [`dc_shift_in_place`]. Callers can pass [`auralis_simd::select_backend`] with
/// either [`BackendKind::Scalar`] or [`BackendKind::Simd`] to force deterministic
/// scalar-vs-SIMD conformance runs. Unsupported SIMD requests follow the
/// fallback recorded in `selection`.
#[inline]
pub fn dc_shift_in_place_with_backend(
    selection: BackendSelection,
    samples: &mut [f32],
    shift: f32,
) {
    dc_shift_f32_in_place_with_backend(selection, samples, shift);
}

/// Applies a linear fade envelope to a contiguous channel segment in place.
///
/// `total_frames` is the full channel length, `start_frame` is the frame index
/// of `samples[0]` in that full channel, and `fade_in` / `fade_out` are frame
/// lengths. A fade-in length of `4` uses coefficients
/// `[0.0, 0.25, 0.5, 0.75]`; the first frame after the fade reaches `1.0`.
/// A fade-out length of `4` applies `[0.75, 0.5, 0.25, 0.0]` to the final four
/// frames. If fade-in and fade-out overlap, their coefficients are multiplied.
/// Zero-length fades are identity transforms.
///
/// The operation is deterministic, non-allocating, and streaming-aware when
/// callers pass the correct full-channel frame positions for each segment.
#[inline]
pub fn fade_in_place(
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

#[cfg(test)]
mod tests {
    use super::{
        dc_shift_in_place, dc_shift_in_place_with_backend, fade_in_place, gain_in_place,
        gain_in_place_with_backend, linear_gain,
    };
    use auralis_core::Decibels;
    use auralis_simd::{BackendKind, select_backend};

    #[test]
    fn zero_db_is_identity() {
        let mut samples = [-1.0, -0.25, 0.0, 0.25, 1.0];

        gain_in_place(&mut samples, db(0.0));

        assert_samples_close(&samples, &[-1.0, -0.25, 0.0, 0.25, 1.0]);
    }

    #[test]
    fn minus_six_db_matches_reference_multiplier() {
        let mut samples = [-1.0, -0.5, 0.25, 1.0];
        let expected_multiplier = 10.0_f32.powf(-6.0 / 20.0);

        gain_in_place(&mut samples, db(-6.0));

        assert_samples_close(
            &samples,
            &[
                -expected_multiplier,
                -0.5 * expected_multiplier,
                0.25 * expected_multiplier,
                expected_multiplier,
            ],
        );
    }

    #[test]
    fn plus_six_db_matches_reference_multiplier() {
        let mut samples = [-1.0, -0.5, 0.25, 1.0];
        let expected_multiplier = 10.0_f32.powf(6.0 / 20.0);

        gain_in_place(&mut samples, db(6.0));

        assert_samples_close(
            &samples,
            &[
                -expected_multiplier,
                -0.5 * expected_multiplier,
                0.25 * expected_multiplier,
                expected_multiplier,
            ],
        );
    }

    #[test]
    fn empty_slice_is_accepted() {
        let mut samples = [];

        gain_in_place(&mut samples, db(-12.0));

        assert!(samples.is_empty());
    }

    #[test]
    fn one_sample_is_scaled() {
        let mut samples = [0.5];

        gain_in_place(&mut samples, db(6.0));

        assert_close(samples[0], 0.5 * 10.0_f32.powf(6.0 / 20.0));
    }

    #[test]
    fn odd_lengths_are_scaled() {
        let mut samples = [-0.75, -0.25, 0.0, 0.25, 0.75];
        let expected_multiplier = 10.0_f32.powf(-6.0 / 20.0);

        gain_in_place(&mut samples, db(-6.0));

        assert_samples_close(
            &samples,
            &[
                -0.75 * expected_multiplier,
                -0.25 * expected_multiplier,
                0.0,
                0.25 * expected_multiplier,
                0.75 * expected_multiplier,
            ],
        );
    }

    #[test]
    fn full_scale_input_is_not_clipped() {
        let mut samples = [-1.0, 1.0];
        let expected_multiplier = 10.0_f32.powf(6.0 / 20.0);

        gain_in_place(&mut samples, db(6.0));

        assert_samples_close(&samples, &[-expected_multiplier, expected_multiplier]);
        assert!(samples[1] > 1.0);
    }

    #[test]
    fn finite_inputs_do_not_produce_nan() {
        let mut samples = [-1.0, -0.0, 0.0, 1.0, f32::MAX];

        gain_in_place(&mut samples, db(10_000.0));

        assert!(samples.iter().all(|sample| !sample.is_nan()));
        assert_eq!(samples[1].to_bits(), (-0.0_f32).to_bits());
        assert_eq!(samples[2].to_bits(), 0.0_f32.to_bits());
    }

    #[test]
    fn gain_matches_under_forced_scalar_and_requested_simd() {
        let source = [
            -1.0,
            -0.999_984_74,
            -0.5,
            -1.0 / 32768.0,
            -0.0,
            0.0,
            1.0 / 32768.0,
            0.5,
            0.999_984_74,
            1.0,
        ];
        let mut scalar = source;
        let mut simd = source;

        gain_in_place_with_backend(select_backend(BackendKind::Scalar), &mut scalar, db(6.0));
        gain_in_place_with_backend(select_backend(BackendKind::Simd), &mut simd, db(6.0));

        assert_sample_bits_eq(&simd, &scalar);
    }

    #[test]
    fn linear_gain_matches_decibel_formula() {
        assert_close(linear_gain(db(-6.0)), 10.0_f32.powf(-6.0 / 20.0));
        assert_close(linear_gain(db(6.0)), 10.0_f32.powf(6.0 / 20.0));
    }

    #[test]
    fn zero_dc_shift_is_identity() {
        let mut samples = [-1.0, -0.25, 0.0, 0.25, 1.0];

        dc_shift_in_place(&mut samples, 0.0);

        assert_samples_close(&samples, &[-1.0, -0.25, 0.0, 0.25, 1.0]);
    }

    #[test]
    fn positive_and_negative_dc_shifts_add_constant_offset() {
        let mut positive = [-0.5, 0.0, 0.5];
        let mut negative = [-0.5, 0.0, 0.5];

        dc_shift_in_place(&mut positive, 0.25);
        dc_shift_in_place(&mut negative, -0.25);

        assert_samples_close(&positive, &[-0.25, 0.25, 0.75]);
        assert_samples_close(&negative, &[-0.75, -0.25, 0.25]);
    }

    #[test]
    fn dc_shift_does_not_clip_full_scale_input() {
        let mut samples = [-1.0, 1.0];

        dc_shift_in_place(&mut samples, 0.25);

        assert_samples_close(&samples, &[-0.75, 1.25]);
        assert!(samples[1] > 1.0);
    }

    #[test]
    fn finite_dc_shift_inputs_do_not_produce_nan() {
        let mut samples = [-1.0, -0.0, 0.0, 1.0, f32::MAX];

        dc_shift_in_place(&mut samples, 2.0);

        assert!(samples.iter().all(|sample| !sample.is_nan()));
    }

    #[test]
    fn dc_shift_matches_under_forced_scalar_and_requested_simd() {
        let source = [
            -1.0,
            -0.999_984_74,
            -f32::MIN_POSITIVE,
            -f32::from_bits(1),
            -0.0,
            0.0,
            f32::from_bits(1),
            f32::MIN_POSITIVE,
            0.999_984_74,
            1.0,
        ];
        let mut scalar = source;
        let mut simd = source;

        dc_shift_in_place_with_backend(select_backend(BackendKind::Scalar), &mut scalar, 0.125);
        dc_shift_in_place_with_backend(select_backend(BackendKind::Simd), &mut simd, 0.125);

        assert_sample_bits_eq(&simd, &scalar);
    }

    #[test]
    fn fade_coefficients_are_linear() {
        let mut samples = [1.0; 6];

        fade_in_place(&mut samples, 6, 0, 4, 3);

        assert_samples_close(&samples, &[0.0, 0.25, 0.5, 0.5, 1.0 / 3.0, 0.0]);
    }

    #[test]
    fn zero_length_fade_is_identity() {
        let mut samples = [-0.5, 0.0, 0.5];

        fade_in_place(&mut samples, 3, 0, 0, 0);

        assert_samples_close(&samples, &[-0.5, 0.0, 0.5]);
    }

    #[test]
    fn fade_in_and_fade_out_only_apply_one_side() {
        let mut fade_in = [1.0; 5];
        let mut fade_out = [1.0; 5];

        fade_in_place(&mut fade_in, 5, 0, 4, 0);
        fade_in_place(&mut fade_out, 5, 0, 0, 4);

        assert_samples_close(&fade_in, &[0.0, 0.25, 0.5, 0.75, 1.0]);
        assert_samples_close(&fade_out, &[1.0, 0.75, 0.5, 0.25, 0.0]);
    }

    #[test]
    fn fade_chunked_segments_match_whole_slice() {
        let mut whole = [1.0; 9];
        let mut chunked = [1.0; 9];
        let mut start = 0_u64;

        fade_in_place(&mut whole, 9, 0, 4, 4);
        for chunk in chunked.chunks_mut(3) {
            fade_in_place(chunk, 9, start, 4, 4);
            start += u64::try_from(chunk.len()).unwrap();
        }

        assert_samples_close(&whole, &chunked);
    }

    fn db(value: f64) -> Decibels {
        Decibels::new(value).unwrap()
    }

    fn assert_samples_close(actual: &[f32], expected: &[f32]) {
        assert_eq!(actual.len(), expected.len());

        for (actual, expected) in actual.iter().zip(expected) {
            assert_close(*actual, *expected);
        }
    }

    fn assert_sample_bits_eq(actual: &[f32], expected: &[f32]) {
        assert_eq!(actual.len(), expected.len());

        for (index, (&actual, &expected)) in actual.iter().zip(expected).enumerate() {
            assert_eq!(
                actual.to_bits(),
                expected.to_bits(),
                "sample {index} differed: {actual} != {expected}"
            );
        }
    }

    fn assert_close(actual: f32, expected: f32) {
        let tolerance = 1.0e-6;
        let difference = (actual - expected).abs();

        assert!(
            difference <= tolerance,
            "expected {actual} to be within {tolerance} of {expected}, difference was {difference}"
        );
    }
}

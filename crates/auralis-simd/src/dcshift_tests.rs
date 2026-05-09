use crate::{BackendKind, dc_shift_f32_in_place_scalar};

use crate::test_support::{
    assert_sample_bits_eq, assert_scalar_and_simd_dc_shift_match, assert_semantically_same_samples,
    dc_shift_with_backend, patterned_f32, seeded_f32,
};

#[test]
fn scalar_dc_shift_f32_matches_known_values_exactly() {
    let mut samples = [-1.0, -0.5, -0.0, 0.0, 0.25, 1.0];

    dc_shift_f32_in_place_scalar(&mut samples, 0.25);

    assert_sample_bits_eq(&samples, &[-0.75, -0.25, 0.25, 0.25, 0.5, 1.25]);
}

#[test]
fn dc_shift_f32_handles_empty_one_sample_odd_and_tail_lengths() {
    for len in [0, 1, 3, 17, 33, 65] {
        let input = patterned_f32(len);

        assert_scalar_and_simd_dc_shift_match(&input, 0.125);
    }
}

#[test]
fn dc_shift_f32_random_finite_values_match_scalar_under_requested_simd() {
    let mut input = seeded_f32(0x1d58_13f7_2f0a_e4b9, 4099);
    input.extend([
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
    ]);

    for shift in [-2.0, -0.25, 0.0, 0.125, 2.0] {
        assert_scalar_and_simd_dc_shift_match(&input, shift);
    }
}

#[test]
fn dc_shift_f32_non_finite_values_follow_documented_behavior() {
    let input = [
        f32::NAN,
        f32::INFINITY,
        f32::NEG_INFINITY,
        -f32::from_bits(1),
        -0.0,
        0.0,
        f32::from_bits(1),
        1.0,
    ];

    for shift in [-0.25, 0.0, 0.25] {
        let scalar = dc_shift_with_backend(BackendKind::Scalar, &input, shift);
        let simd = dc_shift_with_backend(BackendKind::Simd, &input, shift);

        assert_semantically_same_samples(&simd, &scalar);
    }
}

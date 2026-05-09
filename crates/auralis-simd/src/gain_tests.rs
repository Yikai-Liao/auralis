use crate::{BackendKind, gain_f32_in_place_scalar};

use crate::test_support::{
    assert_sample_bits_eq, assert_scalar_and_simd_gain_match, assert_semantically_same_samples,
    gain_with_backend, patterned_f32, seeded_f32,
};

#[test]
fn scalar_gain_f32_matches_known_values_exactly() {
    let mut samples = [-1.0, -0.5, -0.0, 0.0, 0.25, 1.0];

    gain_f32_in_place_scalar(&mut samples, 2.0);

    assert_sample_bits_eq(&samples, &[-2.0, -1.0, -0.0, 0.0, 0.5, 2.0]);
}

#[test]
fn gain_f32_handles_empty_one_sample_odd_and_tail_lengths() {
    for len in [0, 1, 3, 17, 33, 65] {
        let input = patterned_f32(len);

        assert_scalar_and_simd_gain_match(&input, 0.5);
    }
}

#[test]
fn gain_f32_random_finite_values_match_scalar_under_requested_simd() {
    let mut input = seeded_f32(0x8f25_c5a2_d9f3_1011, 4099);
    input.extend([
        -1.0,
        -0.999_984_74,
        -1.0 / 32768.0,
        -0.0,
        0.0,
        1.0 / 32768.0,
        0.999_984_74,
        1.0,
    ]);

    for multiplier in [0.0, 0.5, 1.0, 1.995_262_4, 2.0] {
        assert_scalar_and_simd_gain_match(&input, multiplier);
    }
}

#[test]
fn gain_f32_non_finite_values_follow_documented_behavior() {
    let input = [
        f32::NAN,
        f32::INFINITY,
        f32::NEG_INFINITY,
        -1.0,
        -0.0,
        0.0,
        1.0,
    ];

    for multiplier in [2.0, f32::INFINITY] {
        let scalar = gain_with_backend(BackendKind::Scalar, &input, multiplier);
        let simd = gain_with_backend(BackendKind::Simd, &input, multiplier);

        assert_semantically_same_samples(&simd, &scalar);
    }
}

use crate::{BackendKind, MultiplyError, multiply_f32_scalar};

use crate::test_support::{
    assert_sample_bits_eq, assert_scalar_and_simd_multiply_match, assert_semantically_same_samples,
    multiply_with_backend, patterned_f32, seeded_f32,
};

#[test]
fn scalar_multiply_f32_matches_known_values_exactly() {
    let first = [0.5, -0.5, 1.0];
    let second = [0.25, 1.0];
    let mut output = [9.0; 3];

    multiply_f32_scalar(&[&first, &second], &mut output).unwrap();

    assert_sample_bits_eq(&output, &[0.125, -0.5, 0.0]);
}

#[test]
fn multiply_f32_single_input_is_identity_for_covered_samples() {
    let first = [0.5, -0.5, 1.0];
    let mut output = [9.0; 3];

    multiply_f32_scalar(&[&first], &mut output).unwrap();

    assert_sample_bits_eq(&output, &first);
}

#[test]
fn multiply_f32_zero_input_sample_zeros_output_sample() {
    let first = [0.5, -0.5, 1.0];
    let second = [1.0, 0.0, 1.0];
    let mut output = [9.0; 3];

    multiply_f32_scalar(&[&first, &second], &mut output).unwrap();

    assert_sample_bits_eq(&output, &[0.5, -0.0, 1.0]);
}

#[test]
fn multiply_f32_rejects_input_longer_than_output() {
    let first = [0.0, 0.25, 0.5];
    let mut output = [0.0; 2];

    let error = multiply_f32_scalar(&[&first], &mut output).unwrap_err();

    assert_eq!(
        error,
        MultiplyError::InputLongerThanOutput {
            input_index: 0,
            input_len: 3,
            output_len: 2,
        }
    );
    assert_eq!(
        error.to_string(),
        "multiply input 0 length 3 exceeds output length 2"
    );
    assert_sample_bits_eq(&output, &[0.0, 0.0]);
}

#[test]
fn multiply_f32_handles_empty_one_sample_odd_and_tail_lengths() {
    for len in [0, 1, 3, 17, 33, 65] {
        let first = patterned_f32(len);
        let second = patterned_f32(len.saturating_sub(1));

        assert_scalar_and_simd_multiply_match(&[&first, &second], len);
    }
}

#[test]
fn multiply_f32_random_finite_values_match_scalar_under_requested_simd() {
    let mut first = seeded_f32(0x2c1b_e9f7_0238_a551, 4099);
    let second = seeded_f32(0x47ad_0d99_8f31_6c21, 4077);
    let third = seeded_f32(0xa5a5_71c3_91e4_f00d, 123);
    first.extend([-1.0, -0.999_984_74, -0.0, 0.0, 0.999_984_74, 1.0]);

    assert_scalar_and_simd_multiply_match(&[&first, &second, &third], first.len());
}

#[test]
fn multiply_f32_non_finite_values_follow_documented_behavior() {
    let first = [f32::NAN, f32::INFINITY, f32::NEG_INFINITY, -1.0];
    let second = [1.0, -1.0, 0.0];
    let scalar = multiply_with_backend(BackendKind::Scalar, &[&first, &second], 4);
    let simd = multiply_with_backend(BackendKind::Simd, &[&first, &second], 4);

    assert_semantically_same_samples(&simd, &scalar);
}

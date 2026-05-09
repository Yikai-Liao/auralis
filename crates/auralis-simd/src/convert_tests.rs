use crate::{SampleConversionError, f32_to_i16_scalar, i16_to_f32_scalar};

use crate::test_support::{
    assert_sample_bits_eq, assert_scalar_and_simd_conversion_match,
    assert_scalar_and_simd_f32_to_i16_match, patterned_f32, patterned_pcm16, seeded_f32,
    seeded_pcm16,
};

#[test]
fn scalar_i16_to_f32_matches_known_pcm16_values_exactly() {
    let input = [i16::MIN, -16_384, -1, 0, 1, 16_384, i16::MAX];
    let mut output = [0.0; 7];

    i16_to_f32_scalar(&input, &mut output).unwrap();

    assert_sample_bits_eq(
        &output,
        &[
            -1.0,
            -0.5,
            -1.0 / 32768.0,
            0.0,
            1.0 / 32768.0,
            0.5,
            f32::from(i16::MAX) / 32768.0,
        ],
    );
}

#[test]
fn i16_to_f32_rejects_mismatched_buffer_lengths() {
    let input = [0, 1, 2];
    let mut output = [0.0; 2];

    let error = i16_to_f32_scalar(&input, &mut output).unwrap_err();

    assert_eq!(
        error,
        SampleConversionError::BufferLengthMismatch {
            input_len: 3,
            output_len: 2,
        }
    );
    assert_eq!(
        error.to_string(),
        "sample conversion input length 3 did not match output length 2"
    );
}

#[test]
fn i16_to_f32_handles_empty_one_sample_odd_and_tail_lengths() {
    for len in [0, 1, 3, 17, 33, 65] {
        let input = patterned_pcm16(len);

        assert_scalar_and_simd_conversion_match(&input);
    }
}

#[test]
fn i16_to_f32_random_pcm16_values_match_scalar_under_requested_simd() {
    let mut input = seeded_pcm16(0x9e37_79b9_7f4a_7c15, 4099);
    input.extend([i16::MIN, i16::MAX, -1, 0, 1]);

    assert_scalar_and_simd_conversion_match(&input);
}

#[test]
fn scalar_f32_to_i16_matches_known_values_exactly() {
    let input = [
        -1.5,
        -1.0,
        -0.5,
        -1.0 / 32768.0,
        -0.5 / 32768.0,
        0.0,
        0.5 / 32768.0,
        1.0 / 32768.0,
        0.5,
        f32::from(i16::MAX) / 32768.0,
        1.0,
        1.5,
    ];
    let mut output = [0; 12];

    f32_to_i16_scalar(&input, &mut output).unwrap();

    assert_eq!(
        output,
        [
            i16::MIN,
            i16::MIN,
            -16_384,
            -1,
            -1,
            0,
            1,
            1,
            16_384,
            i16::MAX,
            i16::MAX,
            i16::MAX,
        ]
    );
}

#[test]
fn f32_to_i16_rejects_mismatched_buffer_lengths() {
    let input = [0.0, 0.25, 0.5];
    let mut output = [0; 2];

    let error = f32_to_i16_scalar(&input, &mut output).unwrap_err();

    assert_eq!(
        error,
        SampleConversionError::BufferLengthMismatch {
            input_len: 3,
            output_len: 2,
        }
    );
}

#[test]
fn f32_to_i16_rejects_nan_and_infinity() {
    let input = [0.0, f32::INFINITY, f32::NAN, f32::NEG_INFINITY];
    let mut output = [0; 4];

    let error = f32_to_i16_scalar(&input, &mut output).unwrap_err();

    assert_eq!(
        error,
        SampleConversionError::NonFiniteSample { sample_index: 1 }
    );
    assert_eq!(
        error.to_string(),
        "sample conversion input sample 1 was NaN or infinite"
    );
}

#[test]
fn f32_to_i16_handles_empty_one_sample_odd_and_tail_lengths() {
    for len in [0, 1, 3, 17, 33, 65] {
        let input = patterned_f32(len);

        assert_scalar_and_simd_f32_to_i16_match(&input);
    }
}

#[test]
fn f32_to_i16_random_near_clipping_values_match_scalar_under_requested_simd() {
    let mut input = seeded_f32(0xd1b5_4a32_d192_ed03, 4099);
    input.extend([
        -1.5,
        -1.0,
        -0.999_984_74,
        -1.0 / 65536.0,
        1.0 / 65536.0,
        0.999_984_74,
        1.0,
        1.5,
    ]);

    assert_scalar_and_simd_f32_to_i16_match(&input);
}

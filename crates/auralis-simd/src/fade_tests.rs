use crate::{BackendKind, fade_f32_in_place_scalar};

use crate::test_support::{
    assert_sample_bits_eq, assert_scalar_and_simd_fade_match, assert_semantically_same_samples,
    fade_with_backend, patterned_f32, seeded_f32,
};

#[test]
fn scalar_fade_f32_matches_known_values_exactly() {
    let mut samples = [1.0; 6];

    fade_f32_in_place_scalar(&mut samples, 6, 0, 4, 3);

    assert_sample_bits_eq(&samples, &[0.0, 0.25, 0.5, 0.5, 1.0_f32 / 3.0, 0.0]);
}

#[test]
fn fade_f32_handles_empty_one_sample_odd_and_tail_lengths() {
    for len in [0, 1, 3, 7, 17, 33, 65] {
        let input = patterned_f32(len);
        let total_frames = u64::try_from(len).expect("test length should fit in u64");

        assert_scalar_and_simd_fade_match(&input, total_frames, 0, 4, 4);
    }
}

#[test]
fn fade_f32_deterministic_fixtures_match_scalar_under_requested_simd() {
    let cases = [
        (17, 0, 17, 8, 0),
        (17, 0, 17, 0, 8),
        (17, 0, 17, 8, 8),
        (31, 3, 19, 11, 13),
    ];

    for (total_frames, start_frame, len, fade_in, fade_out) in cases {
        let input = patterned_f32(len);

        assert_scalar_and_simd_fade_match(&input, total_frames, start_frame, fade_in, fade_out);
    }
}

#[test]
fn fade_f32_random_finite_values_match_scalar_under_requested_simd() {
    let input = seeded_f32(0xc31f_202a_74d0_8e11, 4099);

    for (fade_in, fade_out) in [(0, 4096), (4096, 0), (2048, 3072), (8192, 8192)] {
        assert_scalar_and_simd_fade_match(&input, 4099, 0, fade_in, fade_out);
    }
}

#[test]
fn fade_f32_non_finite_values_follow_documented_behavior() {
    let input = [
        f32::NAN,
        f32::INFINITY,
        f32::NEG_INFINITY,
        -1.0,
        -0.0,
        0.0,
        1.0,
    ];
    let scalar = fade_with_backend(BackendKind::Scalar, &input, 7, 0, 4, 4);
    let simd = fade_with_backend(BackendKind::Simd, &input, 7, 0, 4, 4);

    assert_semantically_same_samples(&simd, &scalar);
}

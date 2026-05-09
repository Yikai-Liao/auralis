use crate::{
    BackendKind, dc_shift_f32_in_place_with_backend, f32_to_i16_with_backend,
    fade_f32_in_place_with_backend, gain_f32_in_place_with_backend, i16_to_f32_with_backend,
    mix_f32_with_backend, multiply_f32_with_backend, select_backend,
};

pub(crate) fn assert_scalar_and_simd_conversion_match(input: &[i16]) {
    let scalar = convert_with_backend(BackendKind::Scalar, input);
    let simd = convert_with_backend(BackendKind::Simd, input);

    assert_sample_bits_eq(&scalar, &reference_conversion(input));
    assert_sample_bits_eq(&simd, &scalar);
}

pub(crate) fn assert_scalar_and_simd_f32_to_i16_match(input: &[f32]) {
    let scalar = convert_f32_to_i16_with_backend(BackendKind::Scalar, input);
    let simd = convert_f32_to_i16_with_backend(BackendKind::Simd, input);

    assert_eq!(scalar, reference_f32_to_i16(input));
    assert_eq!(simd, scalar);
}

pub(crate) fn assert_scalar_and_simd_gain_match(input: &[f32], multiplier: f32) {
    let scalar = gain_with_backend(BackendKind::Scalar, input, multiplier);
    let simd = gain_with_backend(BackendKind::Simd, input, multiplier);

    assert_sample_bits_eq(&simd, &scalar);
}

pub(crate) fn assert_scalar_and_simd_dc_shift_match(input: &[f32], shift: f32) {
    let scalar = dc_shift_with_backend(BackendKind::Scalar, input, shift);
    let simd = dc_shift_with_backend(BackendKind::Simd, input, shift);

    assert_sample_bits_eq(&simd, &scalar);
}

pub(crate) fn assert_scalar_and_simd_fade_match(
    input: &[f32],
    total_frames: u64,
    start_frame: u64,
    fade_in: u64,
    fade_out: u64,
) {
    let scalar = fade_with_backend(
        BackendKind::Scalar,
        input,
        total_frames,
        start_frame,
        fade_in,
        fade_out,
    );
    let simd = fade_with_backend(
        BackendKind::Simd,
        input,
        total_frames,
        start_frame,
        fade_in,
        fade_out,
    );

    assert_sample_bits_eq(&simd, &scalar);
}

pub(crate) fn assert_scalar_and_simd_mix_match(inputs: &[&[f32]], output_len: usize, scale: f32) {
    let scalar = mix_with_backend(BackendKind::Scalar, inputs, output_len, scale);
    let simd = mix_with_backend(BackendKind::Simd, inputs, output_len, scale);

    assert_sample_bits_eq(&simd, &scalar);
}

pub(crate) fn assert_scalar_and_simd_multiply_match(inputs: &[&[f32]], output_len: usize) {
    let scalar = multiply_with_backend(BackendKind::Scalar, inputs, output_len);
    let simd = multiply_with_backend(BackendKind::Simd, inputs, output_len);

    assert_sample_bits_eq(&simd, &scalar);
}

pub(crate) fn convert_with_backend(kind: BackendKind, input: &[i16]) -> Vec<f32> {
    let selection = select_backend(kind);
    let mut output = vec![0.0; input.len()];

    i16_to_f32_with_backend(selection, input, &mut output).unwrap();

    output
}

pub(crate) fn convert_f32_to_i16_with_backend(kind: BackendKind, input: &[f32]) -> Vec<i16> {
    let selection = select_backend(kind);
    let mut output = vec![0; input.len()];

    f32_to_i16_with_backend(selection, input, &mut output).unwrap();

    output
}

pub(crate) fn gain_with_backend(kind: BackendKind, input: &[f32], multiplier: f32) -> Vec<f32> {
    let selection = select_backend(kind);
    let mut samples = input.to_vec();

    gain_f32_in_place_with_backend(selection, &mut samples, multiplier);

    samples
}

pub(crate) fn dc_shift_with_backend(kind: BackendKind, input: &[f32], shift: f32) -> Vec<f32> {
    let selection = select_backend(kind);
    let mut samples = input.to_vec();

    dc_shift_f32_in_place_with_backend(selection, &mut samples, shift);

    samples
}

pub(crate) fn fade_with_backend(
    kind: BackendKind,
    input: &[f32],
    total_frames: u64,
    start_frame: u64,
    fade_in: u64,
    fade_out: u64,
) -> Vec<f32> {
    let selection = select_backend(kind);
    let mut samples = input.to_vec();

    fade_f32_in_place_with_backend(
        selection,
        &mut samples,
        total_frames,
        start_frame,
        fade_in,
        fade_out,
    );

    samples
}

pub(crate) fn mix_with_backend(
    kind: BackendKind,
    inputs: &[&[f32]],
    output_len: usize,
    scale: f32,
) -> Vec<f32> {
    let selection = select_backend(kind);
    let mut output = vec![42.0; output_len];

    mix_f32_with_backend(selection, inputs, &mut output, scale).unwrap();

    output
}

pub(crate) fn multiply_with_backend(
    kind: BackendKind,
    inputs: &[&[f32]],
    output_len: usize,
) -> Vec<f32> {
    let selection = select_backend(kind);
    let mut output = vec![42.0; output_len];

    multiply_f32_with_backend(selection, inputs, &mut output).unwrap();

    output
}

pub(crate) fn reference_conversion(input: &[i16]) -> Vec<f32> {
    input
        .iter()
        .map(|&sample| f32::from(sample) / 32768.0)
        .collect()
}

pub(crate) fn reference_f32_to_i16(input: &[f32]) -> Vec<i16> {
    input
        .iter()
        .map(|&sample| reference_f32_sample(sample))
        .collect()
}

#[allow(
    clippy::cast_possible_truncation,
    reason = "the reference sample is rounded and clamped to the i16 range before casting"
)]
pub(crate) fn reference_f32_sample(sample: f32) -> i16 {
    let scaled = (sample.clamp(-1.0, 1.0) * 32768.0)
        .round()
        .clamp(f32::from(i16::MIN), f32::from(i16::MAX));

    scaled as i16
}

pub(crate) fn patterned_pcm16(len: usize) -> Vec<i16> {
    (0..len)
        .map(|index| {
            let value = (index.wrapping_mul(977).wrapping_add(12_345)) & 0xffff;

            #[allow(
                clippy::cast_possible_truncation,
                reason = "test values are intentionally wrapped to the full 16-bit PCM domain"
            )]
            {
                let sample_bits =
                    u16::try_from(value).expect("masked test value should fit in u16");
                sample_bits.cast_signed()
            }
        })
        .collect()
}

pub(crate) fn patterned_f32(len: usize) -> Vec<f32> {
    (0..len)
        .map(|index| {
            let pcm16 = patterned_pcm16_value(index);
            let offset_bits = u16::try_from((index.wrapping_mul(37).wrapping_add(11)) & 0x03ff)
                .expect("masked test offset should fit in u16");
            let offset = (f32::from(offset_bits) / 1024.0) - 0.5;

            (f32::from(pcm16) + offset) / 32768.0
        })
        .collect()
}

fn patterned_pcm16_value(index: usize) -> i16 {
    let value = (index.wrapping_mul(977).wrapping_add(12_345)) & 0xffff;
    let sample_bits = u16::try_from(value).expect("masked test value should fit in u16");

    sample_bits.cast_signed()
}

pub(crate) fn seeded_pcm16(seed: u64, len: usize) -> Vec<i16> {
    let mut state = seed;

    (0..len)
        .map(|_| {
            state = state
                .wrapping_mul(6_364_136_223_846_793_005)
                .wrapping_add(1);
            let sample_bits =
                u16::try_from(state >> 48).expect("shifted test state should fit in u16");

            sample_bits.cast_signed()
        })
        .collect()
}

pub(crate) fn seeded_f32(seed: u64, len: usize) -> Vec<f32> {
    let mut state = seed;

    (0..len)
        .map(|_| {
            state = state
                .wrapping_mul(6_364_136_223_846_793_005)
                .wrapping_add(1);
            let sample_bits =
                u16::try_from(state >> 48).expect("shifted test state should fit in u16");
            state = state
                .wrapping_mul(6_364_136_223_846_793_005)
                .wrapping_add(1);
            let offset_bits = u16::try_from((state >> 54) & 0x03ff)
                .expect("shifted test offset should fit in u16");
            let offset = (f32::from(offset_bits) / 512.0) - 1.0;

            (f32::from(sample_bits.cast_signed()) + offset) / 32768.0
        })
        .collect()
}

pub(crate) fn assert_sample_bits_eq(actual: &[f32], expected: &[f32]) {
    assert_eq!(actual.len(), expected.len());

    for (index, (&actual, &expected)) in actual.iter().zip(expected).enumerate() {
        assert_eq!(
            actual.to_bits(),
            expected.to_bits(),
            "sample {index} differed: {actual} != {expected}"
        );
    }
}

pub(crate) fn assert_semantically_same_samples(actual: &[f32], expected: &[f32]) {
    assert_eq!(actual.len(), expected.len());

    for (index, (&actual, &expected)) in actual.iter().zip(expected).enumerate() {
        if expected.is_nan() {
            assert!(
                actual.is_nan(),
                "sample {index} should be NaN, got {actual}"
            );
        } else {
            assert_eq!(
                actual.to_bits(),
                expected.to_bits(),
                "sample {index} differed: {actual} != {expected}"
            );
        }
    }
}

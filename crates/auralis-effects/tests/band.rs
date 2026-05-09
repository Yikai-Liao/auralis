//! Integration coverage for SoX-ng-style resonator band-pass filtering.

use auralis_core::{AudioBuffer, AudioSpec, ChannelCount, FrameCount, SampleFormat, SampleRate};
use auralis_effects::{
    Band, Biquad, BiquadState, BiquadWidth, EffectCommandParseError, EffectError,
    parse_effect_chain,
};

#[test]
fn band_chain_parses_and_renders_supported_forms() {
    let default_width = parse_effect_chain(&["band", "1k"]).unwrap();
    let explicit_width = parse_effect_chain(&["band", "1000", "500"]).unwrap();
    let unpitched = parse_effect_chain(&["band", "-n", "1000", "2q"]).unwrap();

    assert_eq!(default_width.render_tokens(), ["band", "1000"]);
    assert_eq!(explicit_width.render_tokens(), ["band", "1000", "500h"]);
    assert_eq!(unpitched.render_tokens(), ["band", "-n", "1000", "2q"]);
}

#[test]
fn band_chain_matches_typed_processor() {
    let spec = AudioSpec::new(
        SampleRate::new(48_000).unwrap(),
        ChannelCount::new(2).unwrap(),
        SampleFormat::Float32,
    );
    let source = AudioBuffer::from_planar_f32(
        spec,
        FrameCount::new(5),
        vec![0.5, 0.25, 0.0, -0.25, -0.5, -0.25, 0.0, 0.25, 0.5, 0.25],
    )
    .unwrap();
    let band = Band::new(1_000.0, Some(BiquadWidth::hertz(500.0))).unwrap();
    let mut direct = source.clone();
    let mut parsed = source;

    band.process_buffer(&mut direct).unwrap();
    parse_effect_chain(&["band", "1000", "500"])
        .unwrap()
        .process_buffer(&mut parsed)
        .unwrap();

    assert_sample_bits_eq(parsed.as_planar_f32(), direct.as_planar_f32());
}

#[test]
fn band_supports_state_preserving_chunked_processing() {
    let band = Band::unpitched(1_000.0, Some(BiquadWidth::q(2.0))).unwrap();
    let spec = AudioSpec::new(
        SampleRate::new(48_000).unwrap(),
        ChannelCount::new(1).unwrap(),
        SampleFormat::Float32,
    );
    let source = AudioBuffer::from_planar_f32(
        spec,
        FrameCount::new(8),
        vec![1.0, 0.5, 0.0, -0.5, -1.0, -0.5, 0.0, 0.5],
    )
    .unwrap();
    let mut whole = source.clone();
    let mut chunked = source;
    let coefficients = band.coefficients(SampleRate::new(48_000).unwrap()).unwrap();
    let mut state = BiquadState::new(coefficients);

    band.process_buffer(&mut whole).unwrap();
    for chunk in chunked.as_planar_f32_mut().chunks_mut(3) {
        state.process_mono_samples(chunk);
    }

    assert_sample_bits_eq(chunked.as_planar_f32(), whole.as_planar_f32());
}

#[test]
fn band_rejects_bad_command_shapes_and_runtime_designs() {
    let missing = parse_effect_chain(&["band"]).unwrap_err();
    assert!(matches!(
        missing,
        auralis_effects::EffectChainParseError::CommandParseFailed {
            source: EffectCommandParseError::MissingArgument {
                effect: "band",
                argument: "frequency",
            },
            ..
        }
    ));

    let invalid = parse_effect_chain(&["band", "1000", "1s"]).unwrap_err();
    assert!(matches!(
        invalid,
        auralis_effects::EffectChainParseError::CommandParseFailed {
            source: EffectCommandParseError::InvalidEffectConfig {
                effect: "band",
                argument: "width",
                source: EffectError::InvalidBiquadDesign,
            },
            ..
        }
    ));

    let error = Band::new(24_000.0, Some(BiquadWidth::q(1.0)))
        .unwrap()
        .process_buffer(&mut one_frame_audio())
        .unwrap_err();
    assert_eq!(error, EffectError::InvalidBiquadDesign);
}

#[test]
fn band_coefficients_can_be_processed_by_biquad_primitive() {
    let band = Band::new(1_000.0, Some(BiquadWidth::q(2.0))).unwrap();
    let coefficients = band.coefficients(SampleRate::new(48_000).unwrap()).unwrap();
    let source = [1.0, 0.0, 0.0, 0.0];
    let mut via_band = mono_audio_from_samples(&source);
    let mut via_biquad = source;

    band.process_buffer(&mut via_band).unwrap();
    Biquad::new(coefficients).process_mono_samples(&mut via_biquad);

    assert_sample_bits_eq(via_band.as_planar_f32(), &via_biquad);
}

fn one_frame_audio() -> AudioBuffer {
    let spec = AudioSpec::new(
        SampleRate::new(48_000).unwrap(),
        ChannelCount::new(1).unwrap(),
        SampleFormat::Float32,
    );
    AudioBuffer::from_planar_f32(spec, FrameCount::new(1), vec![0.0]).unwrap()
}

fn mono_audio_from_samples(samples: &[f32]) -> AudioBuffer {
    let spec = AudioSpec::new(
        SampleRate::new(48_000).unwrap(),
        ChannelCount::new(1).unwrap(),
        SampleFormat::Float32,
    );
    AudioBuffer::from_planar_f32(
        spec,
        FrameCount::new(u64::try_from(samples.len()).unwrap()),
        samples.to_vec(),
    )
    .unwrap()
}

fn assert_sample_bits_eq(actual: &[f32], expected: &[f32]) {
    assert_eq!(actual.len(), expected.len());
    for (index, (actual, expected)) in actual.iter().zip(expected).enumerate() {
        assert_eq!(
            actual.to_bits(),
            expected.to_bits(),
            "sample {index} differed: actual={actual} expected={expected}"
        );
    }
}

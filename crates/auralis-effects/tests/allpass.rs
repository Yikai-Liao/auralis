//! Integration coverage for SoX-ng-style all-pass filtering.

use auralis_core::{AudioBuffer, AudioSpec, ChannelCount, FrameCount, SampleFormat, SampleRate};
use auralis_effects::{
    AllPass, Biquad, BiquadState, BiquadWidth, EffectCommandParseError, EffectError,
    parse_effect_chain,
};

#[test]
fn allpass_chain_parses_and_renders_supported_forms() {
    let rbj = parse_effect_chain(&["allpass", "1k", "0.707q"]).unwrap();
    let one_pole = parse_effect_chain(&["allpass", "-1", "500"]).unwrap();
    let two_pole = parse_effect_chain(&["allpass", "-2", "2k"]).unwrap();

    assert_eq!(rbj.render_tokens(), ["allpass", "1000", "0.707q"]);
    assert_eq!(one_pole.render_tokens(), ["allpass", "-1", "500"]);
    assert_eq!(two_pole.render_tokens(), ["allpass", "-2", "2000"]);
}

#[test]
fn allpass_chain_matches_typed_processor() {
    let source = stereo_impulse();
    let all_pass = AllPass::new(1_000.0, BiquadWidth::q(0.707)).unwrap();
    let mut direct = source.clone();
    let mut parsed = source;

    all_pass.process_buffer(&mut direct).unwrap();
    parse_effect_chain(&["allpass", "1000", "0.707q"])
        .unwrap()
        .process_buffer(&mut parsed)
        .unwrap();

    assert_sample_bits_eq(parsed.as_planar_f32(), direct.as_planar_f32());
}

#[test]
fn allpass_supports_state_preserving_chunked_processing() {
    let source = stereo_impulse();
    let all_pass = AllPass::two_pole(1_000.0).unwrap();
    let coefficients = all_pass
        .coefficients(source.spec().sample_rate())
        .expect("fixture design is valid");
    let mut whole = source.clone();
    let mut chunked = source;

    all_pass.process_buffer(&mut whole).unwrap();
    for channel_index in 0..chunked.channels().as_usize() {
        let channel = chunked
            .channel_mut(channel_index)
            .expect("channel index is within the fixture shape");
        let mut state = BiquadState::new(coefficients);
        for chunk in channel.chunks_mut(3) {
            state.process_mono_samples(chunk);
        }
    }

    assert_sample_bits_eq(chunked.as_planar_f32(), whole.as_planar_f32());
}

#[test]
fn allpass_rejects_bad_command_shapes_and_runtime_designs() {
    let missing = parse_effect_chain(&["allpass", "1000"]).unwrap_err();
    assert!(matches!(
        missing,
        auralis_effects::EffectChainParseError::CommandParseFailed {
            source: EffectCommandParseError::MissingArgument {
                effect: "allpass",
                argument: "width",
            },
            ..
        }
    ));

    let invalid = parse_effect_chain(&["allpass", "0", "1q"]).unwrap_err();
    assert!(matches!(
        invalid,
        auralis_effects::EffectChainParseError::CommandParseFailed {
            source: EffectCommandParseError::InvalidEffectConfig {
                effect: "allpass",
                argument: "filter-design",
                source: EffectError::InvalidBiquadDesign,
            },
            ..
        }
    ));

    let mut audio = stereo_impulse();
    let error = AllPass::new(24_000.0, BiquadWidth::q(1.0))
        .unwrap()
        .process_buffer(&mut audio)
        .unwrap_err();
    assert_eq!(error, EffectError::InvalidBiquadDesign);
}

#[test]
fn allpass_coefficients_can_be_processed_by_biquad_primitive() {
    let all_pass = AllPass::one_pole(500.0).unwrap();
    let coefficients = all_pass
        .coefficients(SampleRate::new(48_000).unwrap())
        .unwrap();
    let mut via_allpass = [1.0, 0.0, 0.0, 0.0];
    let mut via_biquad = via_allpass;

    all_pass
        .process_mono_samples(&mut via_allpass, SampleRate::new(48_000).unwrap())
        .unwrap();
    Biquad::new(coefficients).process_mono_samples(&mut via_biquad);

    assert_sample_bits_eq(&via_allpass, &via_biquad);
}

fn stereo_impulse() -> AudioBuffer {
    let spec = AudioSpec::new(
        SampleRate::new(48_000).unwrap(),
        ChannelCount::new(2).unwrap(),
        SampleFormat::Float32,
    );

    AudioBuffer::from_planar_f32(
        spec,
        FrameCount::new(8),
        vec![
            0.5, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, -0.25, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0,
        ],
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

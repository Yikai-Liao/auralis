//! Integration coverage for SoX-ng-style high-pass filters.

use auralis_core::{AudioBuffer, AudioSpec, ChannelCount, FrameCount, SampleFormat, SampleRate};
use auralis_effects::{
    Biquad, BiquadState, BiquadWidth, EffectCommandParseError, EffectError, HighPass,
    parse_effect_chain,
};

#[test]
fn highpass_chain_parses_and_renders_supported_forms() {
    let default_width = parse_effect_chain(&["highpass", "1k"]).unwrap();
    let explicit_width = parse_effect_chain(&["highpass", "1000", "500"]).unwrap();
    let explicit_two_pole = parse_effect_chain(&["highpass", "-2", "1000", "1o"]).unwrap();
    let one_pole = parse_effect_chain(&["highpass", "-1", "500"]).unwrap();

    assert_eq!(
        default_width.render_tokens(),
        ["highpass", "1000", "0.7071067811865476q"]
    );
    assert_eq!(explicit_width.render_tokens(), ["highpass", "1000", "500h"]);
    assert_eq!(
        explicit_two_pole.render_tokens(),
        ["highpass", "1000", "1o"]
    );
    assert_eq!(one_pole.render_tokens(), ["highpass", "-1", "500"]);
}

#[test]
fn highpass_chain_matches_typed_processor() {
    let source = stereo_impulse();
    let high_pass = HighPass::with_width(1_000.0, BiquadWidth::q(0.707)).unwrap();
    let mut direct = source.clone();
    let mut parsed = source;

    high_pass.process_buffer(&mut direct).unwrap();
    parse_effect_chain(&["highpass", "1000", "0.707q"])
        .unwrap()
        .process_buffer(&mut parsed)
        .unwrap();

    assert_sample_bits_eq(parsed.as_planar_f32(), direct.as_planar_f32());
}

#[test]
fn highpass_supports_state_preserving_chunked_processing() {
    let source = stereo_impulse();
    let high_pass = HighPass::with_width(1_000.0, BiquadWidth::q(0.707)).unwrap();
    let coefficients = high_pass
        .coefficients(source.spec().sample_rate())
        .expect("fixture design is valid");
    let mut whole = source.clone();
    let mut chunked = source;

    high_pass.process_buffer(&mut whole).unwrap();
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
fn highpass_rejects_bad_command_shapes_and_runtime_designs() {
    let missing = parse_effect_chain(&["highpass"]).unwrap_err();
    assert!(matches!(
        missing,
        auralis_effects::EffectChainParseError::CommandParseFailed {
            source: EffectCommandParseError::MissingArgument {
                effect: "highpass",
                argument: "frequency",
            },
            ..
        }
    ));

    let invalid_option = parse_effect_chain(&["highpass", "-x", "1000"]).unwrap_err();
    assert!(matches!(
        invalid_option,
        auralis_effects::EffectChainParseError::CommandParseFailed {
            source: EffectCommandParseError::UnsupportedOption {
                effect: "highpass",
                ..
            },
            ..
        }
    ));

    let invalid_width = parse_effect_chain(&["highpass", "1000", "1s"]).unwrap_err();
    assert!(matches!(
        invalid_width,
        auralis_effects::EffectChainParseError::CommandParseFailed {
            source: EffectCommandParseError::InvalidEffectConfig {
                effect: "highpass",
                argument: "width",
                source: EffectError::InvalidBiquadDesign,
            },
            ..
        }
    ));

    let mut audio = stereo_impulse();
    let error = HighPass::new(24_000.0)
        .unwrap()
        .process_buffer(&mut audio)
        .unwrap_err();
    assert_eq!(error, EffectError::InvalidBiquadDesign);
}

#[test]
fn highpass_coefficients_can_be_processed_by_biquad_primitive() {
    let high_pass = HighPass::with_width(1_000.0, BiquadWidth::q(0.707)).unwrap();
    let coefficients = high_pass
        .coefficients(SampleRate::new(48_000).unwrap())
        .unwrap();
    let mut via_highpass = [1.0, 0.0, 0.0, 0.0];
    let mut via_biquad = via_highpass;

    Biquad::new(coefficients).process_mono_samples(&mut via_biquad);
    let mut audio = mono_audio_from_samples(&via_highpass);
    high_pass.process_buffer(&mut audio).unwrap();
    via_highpass.copy_from_slice(audio.as_planar_f32());

    assert_sample_bits_eq(&via_highpass, &via_biquad);
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

fn mono_audio_from_samples(samples: &[f32]) -> AudioBuffer {
    let spec = AudioSpec::new(
        SampleRate::new(48_000).unwrap(),
        ChannelCount::new(1).unwrap(),
        SampleFormat::Float32,
    );

    AudioBuffer::from_planar_f32(
        spec,
        FrameCount::new(samples.len() as u64),
        samples.to_vec(),
    )
    .unwrap()
}

fn assert_sample_bits_eq(actual: &[f32], expected: &[f32]) {
    assert_eq!(actual.len(), expected.len());
    for (index, (&actual, &expected)) in actual.iter().zip(expected).enumerate() {
        assert_eq!(
            actual.to_bits(),
            expected.to_bits(),
            "sample {index}: actual={actual} expected={expected}"
        );
    }
}

//! Integration coverage for SoX-ng-style equalizer peaking filter.

use auralis_core::{AudioBuffer, AudioSpec, ChannelCount, FrameCount, SampleFormat, SampleRate};
use auralis_effects::{
    Biquad, BiquadState, BiquadWidth, EffectCommandParseError, EffectError, Equalizer,
    parse_effect_chain,
};

#[test]
fn equalizer_chain_parses_and_renders_supported_forms() {
    let q_width = parse_effect_chain(&["equalizer", "1k", "0.707q", "+6"]).unwrap();
    let hertz_width = parse_effect_chain(&["equalizer", "1000", "500", "-3"]).unwrap();
    let octave_width = parse_effect_chain(&["eq", "1000", "1o", "6"]).unwrap();

    assert_eq!(
        q_width.render_tokens(),
        ["equalizer", "1000", "0.707q", "6"]
    );
    assert_eq!(
        hertz_width.render_tokens(),
        ["equalizer", "1000", "500h", "-3"]
    );
    assert_eq!(
        octave_width.render_tokens(),
        ["equalizer", "1000", "1o", "6"]
    );
}

#[test]
fn equalizer_chain_matches_typed_processor() {
    let source = stereo_impulse();
    let equalizer = Equalizer::new(1_000.0, BiquadWidth::q(1.0), 6.0).unwrap();
    let mut direct = source.clone();
    let mut parsed = source;

    equalizer.process_buffer(&mut direct).unwrap();
    parse_effect_chain(&["equalizer", "1000", "1q", "6"])
        .unwrap()
        .process_buffer(&mut parsed)
        .unwrap();

    assert_sample_bits_eq(parsed.as_planar_f32(), direct.as_planar_f32());
}

#[test]
fn equalizer_supports_state_preserving_chunked_processing() {
    let source = stereo_impulse();
    let equalizer = Equalizer::new(1_000.0, BiquadWidth::q(1.0), 6.0).unwrap();
    let coefficients = equalizer
        .coefficients(source.spec().sample_rate())
        .expect("fixture design is valid");
    let mut whole = source.clone();
    let mut chunked = source;

    equalizer.process_buffer(&mut whole).unwrap();
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
fn equalizer_rejects_bad_command_shapes_and_runtime_designs() {
    let missing = parse_effect_chain(&["equalizer"]).unwrap_err();
    assert!(matches!(
        missing,
        auralis_effects::EffectChainParseError::CommandParseFailed {
            source: EffectCommandParseError::MissingArgument {
                effect: "equalizer",
                argument: "frequency",
            },
            ..
        }
    ));

    let invalid_option = parse_effect_chain(&["equalizer", "-b", "1000", "1q", "6"]).unwrap_err();
    assert!(matches!(
        invalid_option,
        auralis_effects::EffectChainParseError::CommandParseFailed {
            source: EffectCommandParseError::UnsupportedOption {
                effect: "equalizer",
                ..
            },
            ..
        }
    ));

    let invalid_width = parse_effect_chain(&["equalizer", "1000", "1s", "6"]).unwrap_err();
    assert!(matches!(
        invalid_width,
        auralis_effects::EffectChainParseError::CommandParseFailed {
            source: EffectCommandParseError::InvalidEffectConfig {
                effect: "equalizer",
                argument: "width",
                source: EffectError::InvalidBiquadDesign,
            },
            ..
        }
    ));

    let mut audio = stereo_impulse();
    let error = Equalizer::new(24_000.0, BiquadWidth::q(1.0), 6.0)
        .unwrap()
        .process_buffer(&mut audio)
        .unwrap_err();
    assert_eq!(error, EffectError::InvalidBiquadDesign);
}

#[test]
fn equalizer_coefficients_can_be_processed_by_biquad_primitive() {
    let equalizer = Equalizer::new(1_000.0, BiquadWidth::q(1.0), 6.0).unwrap();
    let coefficients = equalizer
        .coefficients(SampleRate::new(48_000).unwrap())
        .unwrap();
    let mut via_equalizer = [1.0, 0.0, 0.0, 0.0];
    let mut via_biquad = via_equalizer;

    Biquad::new(coefficients).process_mono_samples(&mut via_biquad);
    let mut audio = mono_audio_from_samples(&via_equalizer);
    equalizer.process_buffer(&mut audio).unwrap();
    via_equalizer.copy_from_slice(audio.as_planar_f32());

    assert_sample_bits_eq(&via_equalizer, &via_biquad);
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

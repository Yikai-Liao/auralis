//! Integration coverage for SoX-ng-style de-emphasis filtering.

use auralis_core::{AudioBuffer, AudioSpec, ChannelCount, FrameCount, SampleFormat, SampleRate};
use auralis_effects::{
    Biquad, BiquadState, Deemph, EffectCommandParseError, EffectError, parse_effect_chain,
};

#[test]
fn deemph_chain_parses_and_renders_bare_command() {
    let chain = parse_effect_chain(&["deemph"]).unwrap();

    assert_eq!(chain.render_tokens(), ["deemph"]);
}

#[test]
fn deemph_chain_matches_typed_processor() {
    let source = stereo_impulse(SampleRate::new(48_000).unwrap());
    let mut direct = source.clone();
    let mut parsed = source;

    Deemph::new().process_buffer(&mut direct).unwrap();
    parse_effect_chain(&["deemph"])
        .unwrap()
        .process_buffer(&mut parsed)
        .unwrap();

    assert_sample_bits_eq(parsed.as_planar_f32(), direct.as_planar_f32());
}

#[test]
fn deemph_supports_state_preserving_chunked_processing() {
    let source = stereo_impulse(SampleRate::new(44_100).unwrap());
    let deemph = Deemph::new();
    let coefficients = deemph
        .coefficients(source.spec().sample_rate())
        .expect("fixture sample rate has a SoX-ng deemph preset");
    let mut whole = source.clone();
    let mut chunked = source;

    deemph.process_buffer(&mut whole).unwrap();
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
fn deemph_rejects_arguments_and_unsupported_sample_rates() {
    let extra_argument = parse_effect_chain(&["deemph", "extra"]).unwrap_err();
    assert!(matches!(
        extra_argument,
        auralis_effects::EffectChainParseError::CommandParseFailed {
            source: EffectCommandParseError::UnexpectedArgument {
                effect: "deemph",
                ..
            },
            ..
        }
    ));

    let invalid_option = parse_effect_chain(&["deemph", "-x"]).unwrap_err();
    assert!(matches!(
        invalid_option,
        auralis_effects::EffectChainParseError::CommandParseFailed {
            source: EffectCommandParseError::UnsupportedOption {
                effect: "deemph",
                ..
            },
            ..
        }
    ));

    let mut audio = stereo_impulse(SampleRate::new(32_000).unwrap());
    assert_eq!(
        Deemph::new().process_buffer(&mut audio).unwrap_err(),
        EffectError::InvalidBiquadDesign
    );
}

#[test]
fn deemph_coefficients_can_be_processed_by_biquad_primitive() {
    let deemph = Deemph::new();
    let coefficients = deemph
        .coefficients(SampleRate::new(48_000).unwrap())
        .unwrap();
    let mut via_deemph = [1.0, 0.0, 0.0, 0.0];
    let mut via_biquad = via_deemph;

    Biquad::new(coefficients).process_mono_samples(&mut via_biquad);
    let mut audio = mono_audio_from_samples(&via_deemph);
    deemph.process_buffer(&mut audio).unwrap();
    via_deemph.copy_from_slice(audio.as_planar_f32());

    assert_sample_bits_eq(&via_deemph, &via_biquad);
}

fn stereo_impulse(sample_rate: SampleRate) -> AudioBuffer {
    let spec = AudioSpec::new(
        sample_rate,
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

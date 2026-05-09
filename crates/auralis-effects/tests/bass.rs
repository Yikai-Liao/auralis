//! Integration coverage for SoX-ng-style bass tone control.

use auralis_core::{AudioBuffer, AudioSpec, ChannelCount, FrameCount, SampleFormat, SampleRate};
use auralis_effects::{
    Bass, Biquad, BiquadState, BiquadWidth, EffectCommandParseError, EffectError,
    parse_effect_chain,
};

#[test]
fn bass_chain_parses_and_renders_supported_forms() {
    let defaults = parse_effect_chain(&["bass", "+6"]).unwrap();
    let q_width = parse_effect_chain(&["bass", "-3", "1k", "0.707q"]).unwrap();
    let slope_width = parse_effect_chain(&["bass", "6", "100", "0.5s"]).unwrap();

    assert_eq!(defaults.render_tokens(), ["bass", "6", "100", "0.5s"]);
    assert_eq!(q_width.render_tokens(), ["bass", "-3", "1000", "0.707q"]);
    assert_eq!(slope_width.render_tokens(), ["bass", "6", "100", "0.5s"]);
}

#[test]
fn bass_chain_matches_typed_processor() {
    let source = stereo_impulse();
    let bass = Bass::with_width(6.0, 100.0, BiquadWidth::slope(0.5)).unwrap();
    let mut direct = source.clone();
    let mut parsed = source;

    bass.process_buffer(&mut direct).unwrap();
    parse_effect_chain(&["bass", "6", "100", "0.5s"])
        .unwrap()
        .process_buffer(&mut parsed)
        .unwrap();

    assert_sample_bits_eq(parsed.as_planar_f32(), direct.as_planar_f32());
}

#[test]
fn bass_supports_state_preserving_chunked_processing() {
    let source = stereo_impulse();
    let bass = Bass::with_width(6.0, 100.0, BiquadWidth::slope(0.5)).unwrap();
    let coefficients = bass
        .coefficients(source.spec().sample_rate())
        .expect("fixture design is valid");
    let mut whole = source.clone();
    let mut chunked = source;

    bass.process_buffer(&mut whole).unwrap();
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
fn bass_rejects_bad_command_shapes_and_runtime_designs() {
    let missing = parse_effect_chain(&["bass"]).unwrap_err();
    assert!(matches!(
        missing,
        auralis_effects::EffectChainParseError::CommandParseFailed {
            source: EffectCommandParseError::MissingArgument {
                effect: "bass",
                argument: "gain",
            },
            ..
        }
    ));

    let invalid_option = parse_effect_chain(&["bass", "-b", "24"]).unwrap_err();
    assert!(matches!(
        invalid_option,
        auralis_effects::EffectChainParseError::CommandParseFailed {
            source: EffectCommandParseError::UnsupportedOption { effect: "bass", .. },
            ..
        }
    ));

    let invalid_slope = parse_effect_chain(&["bass", "6", "100", "2s"]).unwrap_err();
    assert!(matches!(
        invalid_slope,
        auralis_effects::EffectChainParseError::CommandParseFailed {
            source: EffectCommandParseError::InvalidEffectConfig {
                effect: "bass",
                argument: "filter-design",
                source: EffectError::InvalidBiquadDesign,
            },
            ..
        }
    ));

    let mut audio = stereo_impulse();
    let error = Bass::with_width(6.0, 24_000.0, BiquadWidth::q(1.0))
        .unwrap()
        .process_buffer(&mut audio)
        .unwrap_err();
    assert_eq!(error, EffectError::InvalidBiquadDesign);
}

#[test]
fn bass_coefficients_can_be_processed_by_biquad_primitive() {
    let bass = Bass::with_width(6.0, 100.0, BiquadWidth::slope(0.5)).unwrap();
    let coefficients = bass.coefficients(SampleRate::new(48_000).unwrap()).unwrap();
    let mut via_bass = [1.0, 0.0, 0.0, 0.0];
    let mut via_biquad = via_bass;

    Biquad::new(coefficients).process_mono_samples(&mut via_biquad);
    let mut audio = mono_audio_from_samples(&via_bass);
    bass.process_buffer(&mut audio).unwrap();
    via_bass.copy_from_slice(audio.as_planar_f32());

    assert_sample_bits_eq(&via_bass, &via_biquad);
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

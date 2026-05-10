//! Integration coverage for SoX-ng-style earwax headphone filtering.

use auralis_core::{AudioBuffer, AudioSpec, ChannelCount, FrameCount, SampleFormat, SampleRate};
use auralis_effects::{
    Earwax, EarwaxState, EffectCommandParseError, EffectError, parse_effect_chain,
};

#[test]
fn earwax_chain_parses_and_renders_bare_command() {
    let chain = parse_effect_chain(&["earwax"]).unwrap();

    assert_eq!(chain.render_tokens(), ["earwax"]);
}

#[test]
fn earwax_chain_matches_typed_processor() {
    let source = stereo_fixture();
    let mut direct = source.clone();
    let mut parsed = source;

    Earwax::new().process_buffer(&mut direct).unwrap();
    parse_effect_chain(&["earwax"])
        .unwrap()
        .process_buffer(&mut parsed)
        .unwrap();

    assert_sample_bits_eq(parsed.as_planar_f32(), direct.as_planar_f32());
}

#[test]
fn earwax_supports_state_preserving_chunked_processing() {
    let source = stereo_fixture();
    let mut whole = source.clone();
    let mut state = EarwaxState::new();
    let mut chunked_samples = vec![0.0; source.as_planar_f32().len()];
    let frames = usize::try_from(source.frames().as_u64()).unwrap();
    let (output_left, output_right) = chunked_samples.split_at_mut(frames);
    let left = source.channel(0).unwrap();
    let right = source.channel(1).unwrap();

    Earwax::new().process_buffer(&mut whole).unwrap();
    for chunk in [(0, 1), (1, 3), (3, frames)] {
        for frame in chunk.0..chunk.1 {
            let (left_sample, right_sample) = state.process_frame(left[frame], right[frame]);
            output_left[frame] = left_sample;
            output_right[frame] = right_sample;
        }
    }

    assert_sample_bits_eq(&chunked_samples, whole.as_planar_f32());
}

#[test]
fn earwax_rejects_arguments_and_invalid_input_shape() {
    let extra_argument = parse_effect_chain(&["earwax", "extra"]).unwrap_err();
    assert!(matches!(
        extra_argument,
        auralis_effects::EffectChainParseError::CommandParseFailed {
            source: EffectCommandParseError::UnexpectedArgument {
                effect: "earwax",
                ..
            },
            ..
        }
    ));

    let invalid_option = parse_effect_chain(&["earwax", "-x"]).unwrap_err();
    assert!(matches!(
        invalid_option,
        auralis_effects::EffectChainParseError::CommandParseFailed {
            source: EffectCommandParseError::UnsupportedOption {
                effect: "earwax",
                ..
            },
            ..
        }
    ));

    let mut wrong_rate = audio_buffer(48_000, 2, vec![0.0, 0.0, 0.0, 0.0]);
    assert_eq!(
        Earwax::new().process_buffer(&mut wrong_rate).unwrap_err(),
        EffectError::InvalidEarwaxInput
    );
}

#[test]
fn earwax_preserves_shape_for_valid_input() {
    let mut audio = stereo_fixture();
    let frames = audio.frames();
    let channels = audio.channels();

    Earwax::new().process_buffer(&mut audio).unwrap();

    assert_eq!(audio.frames(), frames);
    assert_eq!(audio.channels(), channels);
    assert!(
        audio
            .as_planar_f32()
            .iter()
            .all(|sample| sample.is_finite())
    );
}

fn stereo_fixture() -> AudioBuffer {
    audio_buffer(
        44_100,
        2,
        vec![
            0.5, 0.25, 0.0, -0.25, -0.5, -0.25, 0.0, 0.25, -0.25, 0.0, 0.25, 0.5, 0.25, 0.0, -0.25,
            -0.5,
        ],
    )
}

fn audio_buffer(sample_rate: u32, channels: u16, samples: Vec<f32>) -> AudioBuffer {
    let frames = samples.len() / usize::from(channels);
    let spec = AudioSpec::new(
        SampleRate::new(sample_rate).unwrap(),
        ChannelCount::new(channels).unwrap(),
        SampleFormat::Float32,
    );

    AudioBuffer::from_planar_f32(
        spec,
        FrameCount::new(u64::try_from(frames).unwrap()),
        samples,
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

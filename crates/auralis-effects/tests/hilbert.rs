//! Integration coverage for SoX-ng-style Hilbert transform filtering.

use auralis_core::{AudioBuffer, AudioSpec, ChannelCount, FrameCount, SampleFormat, SampleRate};
use auralis_effects::{
    EffectCommand, EffectCommandParseError, EffectError, FirState, Hilbert, parse_effect_chain,
    parse_effect_command,
};

#[test]
fn hilbert_command_parses_renders_and_groups_with_next_effect() {
    assert_eq!(
        parse_effect_command(&["hilbert"]).unwrap(),
        EffectCommand::Hilbert(Hilbert::default_taps())
    );
    assert_eq!(
        parse_effect_command(&["hilbert", "-n", "5"])
            .unwrap()
            .render_tokens(),
        ["hilbert", "-n", "5"]
    );

    let chain = parse_effect_chain(&["hilbert", "-n", "5", "reverse"]).unwrap();

    assert_eq!(chain.len(), 2);
    assert_eq!(chain.commands()[0].render_tokens(), ["hilbert", "-n", "5"]);
    assert_eq!(chain.commands()[1].render_tokens(), ["reverse"]);
}

#[test]
fn hilbert_chain_execution_matches_typed_processor() {
    let source = stereo_audio_buffer(vec![1.0, 0.0, 0.0, 0.0, -1.0, 0.0, 0.0, 0.0]);
    let expected = Hilbert::with_taps(5)
        .unwrap()
        .process_buffer(&source)
        .unwrap();
    let mut actual = source;

    parse_effect_chain(&["hilbert", "-n", "5"])
        .unwrap()
        .process_buffer(&mut actual)
        .unwrap();

    assert_samples_close(actual.as_planar_f32(), expected.as_planar_f32());
}

#[test]
fn hilbert_rejects_invalid_command_shapes() {
    let missing_taps = parse_effect_chain(&["hilbert", "-n"]).unwrap_err();
    assert!(matches!(
        missing_taps,
        auralis_effects::EffectChainParseError::CommandParseFailed {
            source: EffectCommandParseError::UnsupportedOption {
                effect: "hilbert",
                ..
            },
            ..
        }
    ));

    let even_taps = parse_effect_chain(&["hilbert", "-n", "4"]).unwrap_err();
    assert!(matches!(
        even_taps,
        auralis_effects::EffectChainParseError::CommandParseFailed {
            source: EffectCommandParseError::InvalidEffectConfig {
                effect: "hilbert",
                source: EffectError::InvalidHilbert,
                ..
            },
            ..
        }
    ));
}

#[test]
fn hilbert_output_is_finite_and_length_preserving() {
    let source = mono_audio_buffer(vec![0.25, -0.5, 0.75, -0.25, 0.0, 0.5]);
    let shifted = Hilbert::with_taps(7)
        .unwrap()
        .process_buffer(&source)
        .unwrap();

    assert_eq!(shifted.frames(), source.frames());
    assert_eq!(shifted.channels(), source.channels());
    assert!(
        shifted
            .as_planar_f32()
            .iter()
            .all(|sample| sample.is_finite())
    );
}

#[test]
fn hilbert_coefficients_support_chunked_fir_state() {
    let source = mono_audio_buffer(vec![0.0, 1.0, 0.5, -0.5, 0.0]);
    let hilbert = Hilbert::with_taps(5).unwrap();
    let whole = hilbert.process_buffer(&source).unwrap();
    let coefficients = hilbert
        .coefficients_for_sample_rate(source.spec().sample_rate())
        .unwrap();
    let mut state = FirState::new(coefficients);
    let mut chunked = Vec::new();

    state.process_mono_samples(&source.as_planar_f32()[..2], &mut chunked);
    state.process_mono_samples(&source.as_planar_f32()[2..3], &mut chunked);
    state.process_mono_samples(&source.as_planar_f32()[3..], &mut chunked);
    state.finish(&mut chunked);

    assert_samples_close(whole.as_planar_f32(), &chunked);
}

fn mono_audio_buffer(samples: Vec<f32>) -> AudioBuffer {
    audio_buffer(samples, 1)
}

fn stereo_audio_buffer(samples: Vec<f32>) -> AudioBuffer {
    audio_buffer(samples, 2)
}

fn audio_buffer(samples: Vec<f32>, channels: u16) -> AudioBuffer {
    let spec = AudioSpec::new(
        SampleRate::new(48_000).unwrap(),
        ChannelCount::new(channels).unwrap(),
        SampleFormat::Float32,
    );
    AudioBuffer::from_planar_f32(
        spec,
        FrameCount::new(u64::try_from(samples.len() / usize::from(channels)).unwrap()),
        samples,
    )
    .unwrap()
}

fn assert_samples_close(actual: &[f32], expected: &[f32]) {
    assert_eq!(actual.len(), expected.len());
    for (index, (&actual, &expected)) in actual.iter().zip(expected).enumerate() {
        let difference = (actual - expected).abs();
        assert!(
            difference <= 1.0e-6,
            "sample {index} differed by {difference}: {actual} != {expected}"
        );
    }
}

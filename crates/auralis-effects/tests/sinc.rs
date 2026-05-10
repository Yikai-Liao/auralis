//! Integration coverage for SoX-ng-style sinc filtering.

use auralis_core::{AudioBuffer, AudioSpec, ChannelCount, FrameCount, SampleFormat, SampleRate};
use auralis_effects::{
    EffectCommand, EffectCommandParseError, EffectError, FirState, Sinc, SincBand, SincOptions,
    parse_effect_chain, parse_effect_command,
};

#[test]
fn sinc_command_parses_renders_and_groups_with_next_effect() {
    assert_eq!(
        parse_effect_command(&["sinc", "-n", "11", "-4000"])
            .unwrap()
            .render_tokens(),
        ["sinc", "-n", "11", "-4000"]
    );
    assert_eq!(
        parse_effect_command(&["sinc", "-n11", "1000"]).unwrap(),
        EffectCommand::Sinc(
            Sinc::with_options(
                SincBand::HighPass {
                    frequency_hz: 1_000.0
                },
                SincOptions::with_taps(11).unwrap(),
            )
            .unwrap()
        )
    );
    assert_eq!(
        parse_effect_command(&["sinc", "-n", "11", "1000-4000"])
            .unwrap()
            .render_tokens(),
        ["sinc", "-n", "11", "1000-4000"]
    );
    assert_eq!(
        parse_effect_command(&["sinc", "-n", "11", "4000-1000"]).unwrap(),
        EffectCommand::Sinc(
            Sinc::with_options(
                SincBand::BandReject {
                    lower_hz: 1_000.0,
                    upper_hz: 4_000.0
                },
                SincOptions::with_taps(11).unwrap(),
            )
            .unwrap()
        )
    );

    let chain = parse_effect_chain(&["sinc", "-n", "11", "1000-4000", "reverse"]).unwrap();

    assert_eq!(chain.len(), 2);
    assert_eq!(
        chain.commands()[0].render_tokens(),
        ["sinc", "-n", "11", "1000-4000"]
    );
    assert_eq!(chain.commands()[1].render_tokens(), ["reverse"]);
}

#[test]
fn sinc_chain_execution_matches_typed_processor() {
    let source = stereo_audio_buffer(vec![1.0, 0.0, 0.0, -1.0, -0.5, 0.5, 0.25, -0.25]);
    let expected = Sinc::with_options(
        SincBand::low_pass(4_000.0, false),
        SincOptions::with_taps(11).unwrap(),
    )
    .unwrap()
    .process_buffer(&source)
    .unwrap();
    let mut actual = source;

    parse_effect_chain(&["sinc", "-n", "11", "-4000"])
        .unwrap()
        .process_buffer(&mut actual)
        .unwrap();

    assert_samples_close(actual.as_planar_f32(), expected.as_planar_f32());
}

#[test]
fn sinc_rejects_invalid_ranges_and_unsupported_phase_options() {
    let band = parse_effect_chain(&["sinc", "1000-1000"]).unwrap_err();
    assert!(matches!(
        band,
        auralis_effects::EffectChainParseError::CommandParseFailed {
            source: EffectCommandParseError::InvalidEffectConfig {
                effect: "sinc",
                source: EffectError::InvalidSinc,
                ..
            },
            ..
        }
    ));

    let phase = parse_effect_chain(&["sinc", "-M", "1000"]).unwrap_err();
    assert!(matches!(
        phase,
        auralis_effects::EffectChainParseError::CommandParseFailed {
            source: EffectCommandParseError::UnsupportedOption { effect: "sinc", .. },
            ..
        }
    ));
}

#[test]
fn sinc_output_is_finite_and_length_preserving() {
    let source = mono_audio_buffer(vec![0.25, -0.5, 0.75, -0.25, 0.0, 0.5]);
    let filtered = Sinc::with_options(
        SincBand::BandReject {
            lower_hz: 1_000.0,
            upper_hz: 4_000.0,
        },
        SincOptions::with_taps(11).unwrap(),
    )
    .unwrap()
    .process_buffer(&source)
    .unwrap();

    assert_eq!(filtered.frames(), source.frames());
    assert_eq!(filtered.channels(), source.channels());
    assert!(
        filtered
            .as_planar_f32()
            .iter()
            .all(|sample| sample.is_finite())
    );
}

#[test]
fn sinc_coefficients_support_chunked_fir_state() {
    let source = mono_audio_buffer(vec![0.0, 1.0, 0.5, -0.5, 0.0]);
    let sinc = Sinc::with_options(
        SincBand::low_pass(4_000.0, false),
        SincOptions::with_taps(11).unwrap(),
    )
    .unwrap();
    let whole = sinc.process_buffer(&source).unwrap();
    let coefficients = sinc
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

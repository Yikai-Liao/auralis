//! Integration coverage for SoX-ng-style deterministic dither.

use auralis_core::{AudioBuffer, AudioSpec, ChannelCount, FrameCount, SampleFormat, SampleRate};
use auralis_effects::{
    Dither, DitherMode, DitherNoiseShape, DitherState, EffectChainParseError, EffectCommand,
    EffectCommandParseError, EffectError, parse_effect_chain, parse_effect_command,
};

#[test]
fn dither_command_parses_renders_and_groups_with_next_effect() {
    assert_eq!(
        parse_effect_command(&["dither"]).unwrap().render_tokens(),
        ["dither", "-p", "16"]
    );
    assert_eq!(
        parse_effect_command(&["dither", "-S", "-p8"]).unwrap(),
        EffectCommand::Dither(Dither::sloped_tpdf().with_precision(8).unwrap())
    );
    assert_eq!(
        parse_effect_command(&["dither", "-s", "-p8"]).unwrap(),
        EffectCommand::Dither(Dither::shibata().with_precision(8).unwrap())
    );
    assert_eq!(
        parse_effect_command(&["dither", "-f", "shibata"])
            .unwrap()
            .render_tokens(),
        ["dither", "-s", "-p", "16"]
    );

    let chain = parse_effect_chain(&["dither", "-s", "-p", "8", "reverse"]).unwrap();

    assert_eq!(chain.len(), 2);
    assert_eq!(
        chain.commands()[0].render_tokens(),
        ["dither", "-s", "-p", "8"]
    );
    assert_eq!(chain.commands()[1].render_tokens(), ["reverse"]);
}

#[test]
fn dither_chain_execution_matches_typed_processor() {
    let source = audio_buffer(vec![0.0, 0.001, -0.001, 0.25, -0.25, 0.0], 1);
    let mut expected = source.clone();
    Dither::new()
        .with_precision(8)
        .unwrap()
        .process_buffer(&mut expected);
    let mut actual = source;

    parse_effect_chain(&["dither", "-p", "8"])
        .unwrap()
        .process_buffer(&mut actual)
        .unwrap();

    assert_eq!(actual.as_planar_f32(), expected.as_planar_f32());
}

#[test]
fn dither_is_deterministic_for_silence_and_near_zero_samples() {
    let mut silence = audio_buffer(vec![0.0; 8], 2);
    let mut repeated = silence.clone();
    let mut near_zero = audio_buffer(vec![0.0, 1.0 / 65_536.0, -1.0 / 65_536.0, 0.0], 1);
    let dither = Dither::new().with_precision(8).unwrap().with_seed(7);

    dither.process_buffer(&mut silence);
    dither.process_buffer(&mut repeated);
    Dither::sloped_tpdf()
        .with_precision(8)
        .unwrap()
        .with_seed(7)
        .process_buffer(&mut near_zero);

    assert_eq!(silence.as_planar_f32(), repeated.as_planar_f32());
    assert!(
        silence
            .as_planar_f32()
            .iter()
            .any(|sample| sample.to_bits() != 0.0_f32.to_bits())
    );
    assert!(
        near_zero
            .as_planar_f32()
            .iter()
            .all(|sample| sample.is_finite())
    );
    assert!(
        near_zero
            .as_planar_f32()
            .iter()
            .all(|sample| is_quantized_to_bits(*sample, 8))
    );
}

#[test]
fn dither_state_is_chunk_invariant() {
    let dither = Dither::shibata().with_precision(8).unwrap().with_seed(99);
    let mut whole = vec![0.0, 0.001, -0.001, 0.25, -0.25, 0.0, 0.5, -0.5];
    let mut chunked = whole.clone();
    let mut state = DitherState::new(dither);

    dither.process_samples(&mut whole);
    state.process_samples(&mut chunked[..3]);
    state.process_samples(&mut chunked[3..6]);
    state.process_samples(&mut chunked[6..]);

    assert_eq!(chunked, whole);
}

#[test]
fn dither_shibata_noise_shaping_differs_from_plain_tpdf() {
    let mut plain = audio_buffer(vec![0.0; 16], 1);
    let mut shaped = plain.clone();

    Dither::new()
        .with_precision(8)
        .unwrap()
        .with_seed(5)
        .process_buffer(&mut plain);
    Dither::shibata()
        .with_precision(8)
        .unwrap()
        .with_seed(5)
        .process_buffer(&mut shaped);

    assert_ne!(plain.as_planar_f32(), shaped.as_planar_f32());
    assert!(
        shaped
            .as_planar_f32()
            .iter()
            .all(|sample| is_quantized_to_bits(*sample, 8))
    );
}

#[test]
fn dither_rejects_auto_detect_unsupported_shapes_and_bad_precision() {
    let shaping = parse_effect_chain(&["dither", "-f", "gesemann"]).unwrap_err();
    assert!(matches!(
        shaping,
        EffectChainParseError::CommandParseFailed {
            source: EffectCommandParseError::UnsupportedOption {
                effect: "dither",
                ..
            },
            ..
        }
    ));

    let auto = parse_effect_chain(&["dither", "-a"]).unwrap_err();
    assert!(matches!(
        auto,
        EffectChainParseError::CommandParseFailed {
            source: EffectCommandParseError::UnsupportedOption {
                effect: "dither",
                ..
            },
            ..
        }
    ));

    let precision = parse_effect_chain(&["dither", "-p", "1"]).unwrap_err();
    assert!(matches!(
        precision,
        EffectChainParseError::CommandParseFailed {
            source: EffectCommandParseError::InvalidEffectConfig {
                effect: "dither",
                source: EffectError::InvalidDither,
                ..
            },
            ..
        }
    ));
}

#[test]
fn dither_typed_api_exposes_seed_config_policy() {
    let dither = Dither::sloped_tpdf()
        .with_precision(12)
        .unwrap()
        .with_seed(42);

    assert_eq!(dither.mode(), DitherMode::SlopedTpdf);
    assert_eq!(dither.precision_bits(), 12);
    assert_eq!(dither.seed(), 42);

    let shaped = dither.with_noise_shape(DitherNoiseShape::Shibata);
    assert_eq!(shaped.mode(), DitherMode::Tpdf);
    assert_eq!(shaped.noise_shape(), Some(DitherNoiseShape::Shibata));
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

fn is_quantized_to_bits(sample: f32, bits: u8) -> bool {
    let scale = 2.0_f32.powi(i32::from(bits - 1));
    let scaled = sample * scale;
    (scaled - scaled.round()).abs() <= f32::EPSILON
}

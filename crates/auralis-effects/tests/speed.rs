//! Integration coverage for SoX-ng-style speed.

use auralis_core::{AudioSpec, ChannelCount, FrameCount, SampleFormat, SampleRate};
use auralis_effects::{
    EffectChainParseError, EffectCommand, EffectCommandParseError, EffectError, Reverse, Speed,
    parse_effect_chain,
};

#[test]
fn speed_chain_parses_ratio_and_cents_factor() {
    let ratio = parse_effect_chain(&["speed", "1.5", "reverse"]).unwrap();
    assert_eq!(
        ratio.commands(),
        &[
            EffectCommand::Speed(Speed::new(1.5).unwrap()),
            EffectCommand::Reverse(Reverse::new())
        ]
    );
    assert_eq!(ratio.render_tokens(), ["speed", "1.5", "reverse"]);

    let cents = parse_effect_chain(&["speed", "100c"]).unwrap();
    let [EffectCommand::Speed(speed)] = cents.commands() else {
        panic!("expected one speed command");
    };
    assert!((speed.factor - 2.0_f64.powf(100.0 / 1200.0)).abs() < 1.0e-12);
}

#[test]
fn speed_changes_sample_rate_and_preserves_decoded_audio() {
    let mut audio = stereo_audio(vec![0.0, 0.25, -0.5], vec![0.5, -0.25, 1.0]);

    parse_effect_chain(&["speed", "1.5"])
        .unwrap()
        .process_buffer(&mut audio)
        .unwrap();

    assert_eq!(audio.frames(), FrameCount::new(3));
    assert_eq!(audio.spec().sample_rate().as_u32(), 72_000);
    assert_eq!(audio.as_planar_f32(), &[0.0, 0.25, -0.5, 0.5, -0.25, 1.0]);
}

#[test]
fn speed_cents_factor_rounds_output_sample_rate() {
    let mut audio = mono_audio(vec![0.0]);

    parse_effect_chain(&["speed", "-100c"])
        .unwrap()
        .process_buffer(&mut audio)
        .unwrap();

    assert_eq!(audio.spec().sample_rate().as_u32(), 45_306);
    assert_eq!(audio.as_planar_f32(), &[0.0]);
}

#[test]
fn speed_factor_one_is_identity_in_chain() {
    let mut audio = mono_audio(vec![-0.5, 0.0, 0.5]);

    parse_effect_chain(&["speed", "1"])
        .unwrap()
        .process_buffer(&mut audio)
        .unwrap();

    assert_eq!(audio.as_planar_f32(), &[-0.5, 0.0, 0.5]);
    assert_eq!(audio.frames(), FrameCount::new(3));
    assert_eq!(audio.spec().sample_rate().as_u32(), 48_000);
}

#[test]
fn speed_rejects_invalid_factor_and_unrepresentable_output_rate() {
    let invalid_factor = parse_effect_chain(&["speed", "0"]).unwrap_err();
    assert!(matches!(
        invalid_factor,
        EffectChainParseError::CommandParseFailed {
            source: EffectCommandParseError::InvalidEffectConfig {
                effect: "speed",
                argument: "factor",
                source: EffectError::InvalidSpeedFactor,
            },
            ..
        }
    ));

    let mut high_rate = audio_buffer(1, u32::MAX, vec![0.0]);
    let high_rate_error = parse_effect_chain(&["speed", "2"])
        .unwrap()
        .process_buffer(&mut high_rate)
        .unwrap_err();
    assert!(matches!(
        high_rate_error,
        auralis_effects::EffectChainError::CommandFailed {
            argument: "factor",
            source: EffectError::SpeedRateOutOfRange,
            ..
        }
    ));
}

fn mono_audio(samples: Vec<f32>) -> auralis_core::AudioBuffer {
    audio_buffer(1, 48_000, samples)
}

fn stereo_audio(left: Vec<f32>, right: Vec<f32>) -> auralis_core::AudioBuffer {
    assert_eq!(left.len(), right.len());
    audio_buffer(2, 48_000, left.into_iter().chain(right).collect())
}

fn audio_buffer(channels: u16, sample_rate: u32, samples: Vec<f32>) -> auralis_core::AudioBuffer {
    let channels = ChannelCount::new(channels).unwrap();
    let frames = samples.len() / channels.as_usize();
    let spec = AudioSpec::new(
        SampleRate::new(sample_rate).unwrap(),
        channels,
        SampleFormat::Float32,
    );
    auralis_core::AudioBuffer::from_planar_f32(
        spec,
        FrameCount::new(u64::try_from(frames).unwrap()),
        samples,
    )
    .unwrap()
}

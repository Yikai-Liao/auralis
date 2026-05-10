//! Integration coverage for SoX-ng-style tempo processing.

use auralis_core::{AudioSpec, ChannelCount, FrameCount, SampleFormat, SampleRate};
use auralis_effects::{
    EffectChainParseError, EffectCommand, EffectCommandParseError, EffectError, Reverse, Tempo,
    TempoProfile, parse_effect_chain,
};

#[test]
fn tempo_chain_parses_factor_and_stops_before_next_effect() {
    let chain = parse_effect_chain(&["tempo", "1.25", "reverse"]).unwrap();

    assert_eq!(
        chain.commands(),
        &[
            EffectCommand::Tempo(Tempo::new(1.25).unwrap()),
            EffectCommand::Reverse(Reverse::new())
        ]
    );
    assert_eq!(chain.render_tokens(), ["tempo", "1.25", "reverse"]);
}

#[test]
fn tempo_chain_parses_tuning_options_and_stops_before_next_effect() {
    let chain =
        parse_effect_chain(&["tempo", "-q", "-m", "1.25", "60", "10", "8", "reverse"]).unwrap();

    assert_eq!(
        chain.commands(),
        &[
            EffectCommand::Tempo(
                Tempo::with_tuning(
                    1.25,
                    true,
                    TempoProfile::Music,
                    Some(60.0),
                    Some(10.0),
                    Some(8.0)
                )
                .unwrap()
            ),
            EffectCommand::Reverse(Reverse::new())
        ]
    );
    assert_eq!(
        chain.render_tokens(),
        ["tempo", "-q", "-m", "1.25", "60", "10", "8", "reverse"]
    );
}

#[test]
fn tempo_chain_changes_duration_and_preserves_sample_rate() {
    let mut audio = mono_audio(
        (0_u16..8192)
            .map(|frame| f32::from(frame % 128) / 128.0)
            .collect(),
    );

    parse_effect_chain(&["tempo", "1.5"])
        .unwrap()
        .process_buffer(&mut audio)
        .unwrap();

    assert_eq!(audio.frames().as_u64(), 5461);
    assert_eq!(audio.spec().sample_rate().as_u32(), 48_000);
    assert!(
        audio
            .as_planar_f32()
            .iter()
            .all(|sample| sample.is_finite())
    );
}

#[test]
fn tempo_chain_preserves_stereo_channel_count() {
    let mut audio = stereo_audio(
        (0_u16..8192)
            .map(|frame| f32::from(frame % 128) / 128.0)
            .collect(),
        (0_u16..8192)
            .map(|frame| -f32::from(frame % 128) / 128.0)
            .collect(),
    );

    parse_effect_chain(&["tempo", "0.75"])
        .unwrap()
        .process_buffer(&mut audio)
        .unwrap();

    assert_eq!(audio.channels(), ChannelCount::new(2).unwrap());
    assert_eq!(audio.frames().as_u64(), 10923);
}

#[test]
fn tempo_chain_rejects_missing_unsupported_options_and_invalid_values() {
    assert_eq!(
        parse_effect_chain(&["tempo"]).unwrap_err(),
        EffectChainParseError::CommandParseFailed {
            index: 0,
            command: "tempo".to_owned(),
            source: EffectCommandParseError::MissingArgument {
                effect: "tempo",
                argument: "factor",
            },
        }
    );
    assert_eq!(
        parse_effect_chain(&["tempo", "-x", "1.25"]).unwrap_err(),
        EffectChainParseError::CommandParseFailed {
            index: 0,
            command: "tempo -x 1.25".to_owned(),
            source: EffectCommandParseError::UnsupportedOption {
                effect: "tempo",
                option: "-x".to_owned(),
            },
        }
    );
    assert_eq!(
        parse_effect_chain(&["tempo", "101"]).unwrap_err(),
        EffectChainParseError::CommandParseFailed {
            index: 0,
            command: "tempo 101".to_owned(),
            source: EffectCommandParseError::InvalidEffectConfig {
                effect: "tempo",
                argument: "tempo",
                source: EffectError::InvalidTempoFactor,
            },
        }
    );
    assert_eq!(
        parse_effect_chain(&["tempo", "1.25", "9"]).unwrap_err(),
        EffectChainParseError::CommandParseFailed {
            index: 0,
            command: "tempo 1.25 9".to_owned(),
            source: EffectCommandParseError::InvalidEffectConfig {
                effect: "tempo",
                argument: "tempo",
                source: EffectError::InvalidTempoTuning,
            },
        }
    );
}

fn mono_audio(samples: Vec<f32>) -> auralis_core::AudioBuffer {
    let spec = AudioSpec::new(
        SampleRate::new(48_000).unwrap(),
        ChannelCount::new(1).unwrap(),
        SampleFormat::Float32,
    );
    auralis_core::AudioBuffer::from_planar_f32(
        spec,
        FrameCount::new(u64::try_from(samples.len()).unwrap()),
        samples,
    )
    .unwrap()
}

fn stereo_audio(left: Vec<f32>, right: Vec<f32>) -> auralis_core::AudioBuffer {
    assert_eq!(left.len(), right.len());
    let spec = AudioSpec::new(
        SampleRate::new(48_000).unwrap(),
        ChannelCount::new(2).unwrap(),
        SampleFormat::Float32,
    );
    let frames = FrameCount::new(u64::try_from(left.len()).unwrap());
    let mut samples = left;
    samples.extend(right);
    auralis_core::AudioBuffer::from_planar_f32(spec, frames, samples).unwrap()
}

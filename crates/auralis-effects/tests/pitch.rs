//! Integration coverage for SoX-ng-style pitch processing.

use auralis_core::{AudioSpec, ChannelCount, FrameCount, SampleFormat, SampleRate};
use auralis_effects::{
    EffectChainError, EffectChainParseError, EffectCommand, EffectCommandParseError, EffectError,
    Pitch, Reverse, parse_effect_chain,
};

#[test]
fn pitch_chain_parses_shift_and_stops_before_next_effect() {
    let chain = parse_effect_chain(&["pitch", "1200", "reverse"]).unwrap();

    assert_eq!(
        chain.commands(),
        &[
            EffectCommand::Pitch(Pitch::new(1200.0).unwrap()),
            EffectCommand::Reverse(Reverse::new())
        ]
    );
    assert_eq!(chain.render_tokens(), ["pitch", "1200", "reverse"]);
}

#[test]
fn pitch_chain_parses_quick_and_explicit_timing_options() {
    let chain = parse_effect_chain(&["pitch", "-q", "-1200", "60", "10", "8", "reverse"]).unwrap();

    assert_eq!(
        chain.commands(),
        &[
            EffectCommand::Pitch(
                Pitch::with_tuning(-1200.0, true, Some(60.0), Some(10.0), Some(8.0)).unwrap()
            ),
            EffectCommand::Reverse(Reverse::new())
        ]
    );
    assert_eq!(
        chain.render_tokens(),
        ["pitch", "-q", "-1200", "60", "10", "8", "reverse"]
    );
}

#[test]
fn pitch_chain_changes_sample_rate_and_preserves_duration() {
    let mut audio = mono_audio(
        (0_u16..8192)
            .map(|frame| f32::from(frame % 128) / 128.0)
            .collect(),
    );

    parse_effect_chain(&["pitch", "1200"])
        .unwrap()
        .process_buffer(&mut audio)
        .unwrap();

    assert_eq!(audio.spec().sample_rate().as_u32(), 96_000);
    assert_eq!(audio.frames().as_u64(), 16_384);
    assert!(
        audio
            .as_planar_f32()
            .iter()
            .all(|sample| sample.is_finite())
    );
}

#[test]
fn pitch_chain_preserves_stereo_channel_count() {
    let mut audio = stereo_audio(
        (0_u16..8192)
            .map(|frame| f32::from(frame % 128) / 128.0)
            .collect(),
        (0_u16..8192)
            .map(|frame| -f32::from(frame % 128) / 128.0)
            .collect(),
    );

    parse_effect_chain(&["pitch", "-1200"])
        .unwrap()
        .process_buffer(&mut audio)
        .unwrap();

    assert_eq!(audio.channels(), ChannelCount::new(2).unwrap());
    assert_eq!(audio.spec().sample_rate().as_u32(), 24_000);
    assert_eq!(audio.frames().as_u64(), 4096);
}

#[test]
fn pitch_chain_rejects_missing_unsupported_options_invalid_values_and_rate_overflow() {
    assert_eq!(
        parse_effect_chain(&["pitch"]).unwrap_err(),
        EffectChainParseError::CommandParseFailed {
            index: 0,
            command: "pitch".to_owned(),
            source: EffectCommandParseError::MissingArgument {
                effect: "pitch",
                argument: "shift",
            },
        }
    );
    assert_eq!(
        parse_effect_chain(&["pitch", "-m", "1200"]).unwrap_err(),
        EffectChainParseError::CommandParseFailed {
            index: 0,
            command: "pitch -m 1200".to_owned(),
            source: EffectCommandParseError::UnsupportedOption {
                effect: "pitch",
                option: "-m".to_owned(),
            },
        }
    );
    assert_eq!(
        parse_effect_chain(&["pitch", "5000"]).unwrap_err(),
        EffectChainParseError::CommandParseFailed {
            index: 0,
            command: "pitch 5000".to_owned(),
            source: EffectCommandParseError::InvalidEffectConfig {
                effect: "pitch",
                argument: "pitch",
                source: EffectError::InvalidPitchShift,
            },
        }
    );
    assert_eq!(
        parse_effect_chain(&["pitch", "1200", "9"]).unwrap_err(),
        EffectChainParseError::CommandParseFailed {
            index: 0,
            command: "pitch 1200 9".to_owned(),
            source: EffectCommandParseError::InvalidEffectConfig {
                effect: "pitch",
                argument: "pitch",
                source: EffectError::InvalidPitchTuning,
            },
        }
    );

    let mut high_rate = mono_audio_with_rate(u32::MAX, vec![0.0]);
    let high_rate_error = parse_effect_chain(&["pitch", "1200"])
        .unwrap()
        .process_buffer(&mut high_rate)
        .unwrap_err();
    assert!(matches!(
        high_rate_error,
        EffectChainError::CommandFailed {
            argument: "pitch",
            source: EffectError::PitchRateOutOfRange,
            ..
        }
    ));
}

fn mono_audio(samples: Vec<f32>) -> auralis_core::AudioBuffer {
    mono_audio_with_rate(48_000, samples)
}

fn mono_audio_with_rate(sample_rate: u32, samples: Vec<f32>) -> auralis_core::AudioBuffer {
    let spec = AudioSpec::new(
        SampleRate::new(sample_rate).unwrap(),
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

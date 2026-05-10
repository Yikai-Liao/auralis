//! Integration coverage for SoX-ng-style silence trimming.

use auralis_core::{AudioBuffer, AudioSpec, ChannelCount, FrameCount, SampleFormat, SampleRate};
use auralis_effects::{
    EffectChainParseError, EffectCommand, EffectCommandParseError, EffectError, Silence,
    parse_effect_chain,
};

#[test]
fn silence_chain_parses_copy_trim_and_restart_forms() {
    let copy = parse_effect_chain(&["silence", "0", "reverse"]).unwrap();
    assert_eq!(copy.render_tokens(), ["silence", "0", "reverse"]);
    assert_eq!(
        copy.commands()[0],
        EffectCommand::Silence(Silence::copy_through())
    );

    let restart = parse_effect_chain(&[
        "silence", "-l", "1", "1s", "0%", "-1", "2s", "-40d", "reverse",
    ])
    .unwrap();
    assert_eq!(
        restart.render_tokens(),
        [
            "silence", "-l", "1", "1s", "0%", "-1", "2s", "-40d", "reverse",
        ]
    );
}

#[test]
fn silence_trims_leading_and_trailing_regions_in_chain() {
    let mut audio = mono_audio_buffer(vec![0.0, 0.0, 0.25, 0.5, 0.0, 0.0]);

    parse_effect_chain(&["silence", "1", "1s", "0%", "1", "2s", "0%"])
        .unwrap()
        .process_buffer(&mut audio)
        .unwrap();

    assert_eq!(audio.as_planar_f32(), &[0.25, 0.5]);
    assert_eq!(audio.frames(), FrameCount::new(2));
}

#[test]
fn silence_restart_removes_middle_silence() {
    let mut audio = mono_audio_buffer(vec![0.0, 0.25, 0.5, 0.0, 0.0, 0.75, 0.25]);

    parse_effect_chain(&["silence", "1", "1s", "0%", "-1", "2s", "0%"])
        .unwrap()
        .process_buffer(&mut audio)
        .unwrap();

    assert_eq!(audio.as_planar_f32(), &[0.25, 0.5, 0.75, 0.25]);
    assert_eq!(audio.frames(), FrameCount::new(4));
}

#[test]
fn silence_uses_all_channels_for_below_period_detection() {
    let mut audio = stereo_audio_buffer(vec![0.25, 0.5, 0.0, 0.25, 0.5, 0.0]);

    parse_effect_chain(&["silence", "0", "1", "1s", "0%"])
        .unwrap()
        .process_buffer(&mut audio)
        .unwrap();

    assert_eq!(audio.as_planar_f32(), &[0.25, 0.5, 0.25, 0.5]);
    assert_eq!(audio.frames(), FrameCount::new(2));
}

#[test]
fn silence_rejects_invalid_values_and_extra_arguments() {
    let invalid = parse_effect_chain(&["silence", "-l", "0"]).unwrap_err();
    assert!(matches!(
        invalid,
        EffectChainParseError::CommandParseFailed {
            source: EffectCommandParseError::InvalidEffectConfig {
                effect: "silence",
                argument: "silence",
                source: EffectError::InvalidSilence,
            },
            ..
        }
    ));

    let extra = parse_effect_chain(&["silence", "0", "1", "1s", "0%", "tail"]).unwrap_err();
    assert!(matches!(
        extra,
        EffectChainParseError::CommandParseFailed {
            source: EffectCommandParseError::UnexpectedArgument {
                effect: "silence",
                argument,
            },
            ..
        } if argument == "tail"
    ));
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

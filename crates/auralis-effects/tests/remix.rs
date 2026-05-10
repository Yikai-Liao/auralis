//! Integration coverage for SoX-ng-style remix routing.

use auralis_core::{AudioSpec, ChannelCount, FrameCount, SampleFormat, SampleRate};
use auralis_effects::{
    EffectChainParseError, EffectCommand, EffectCommandParseError, Remix, RemixGain,
    RemixLevelMode, RemixOutputSpec, RemixSource, parse_effect_chain,
};

#[test]
fn remix_chain_parses_and_renders_basic_routing() {
    let chain = parse_effect_chain(&["remix", "1,2", "0", "-"]).unwrap();

    assert_eq!(
        chain.commands(),
        &[EffectCommand::Remix(
            Remix::new([
                RemixOutputSpec::new([
                    RemixSource::channel(1).unwrap(),
                    RemixSource::channel(2).unwrap(),
                ])
                .unwrap(),
                RemixOutputSpec::silent(),
                RemixOutputSpec::new([RemixSource::all()]).unwrap(),
            ])
            .unwrap()
        )]
    );
    assert_eq!(chain.render_tokens(), ["remix", "1,2", "0", "-"]);
}

#[test]
fn remix_mixes_routes_and_inserts_silence_in_chain() {
    let mut audio = stereo_audio(vec![0.25, 0.75], vec![-0.5, 0.5]);

    parse_effect_chain(&["remix", "1,2", "0", "2"])
        .unwrap()
        .process_buffer(&mut audio)
        .unwrap();

    assert_eq!(audio.channels(), ChannelCount::new(3).unwrap());
    assert_eq!(audio.frames(), FrameCount::new(2));
    assert_eq!(audio.as_planar_f32(), &[-0.125, 0.625, 0.0, 0.0, -0.5, 0.5]);
}

#[test]
fn remix_supports_gain_modifiers_and_level_options() {
    let chain = parse_effect_chain(&["remix", "-a", "-p", "1v0.5,2p-6,2i0"]).unwrap();

    assert_eq!(
        chain.commands(),
        &[EffectCommand::Remix(
            Remix::with_level_options(
                [RemixOutputSpec::with_gains(
                    [
                        RemixSource::channel(1).unwrap(),
                        RemixSource::channel(2).unwrap(),
                        RemixSource::channel(2).unwrap(),
                    ],
                    [
                        Some(RemixGain::voltage(0.5).unwrap()),
                        Some(RemixGain::PowerDb(
                            auralis_core::Decibels::new(-6.0).unwrap()
                        )),
                        Some(RemixGain::InvertedPowerDb(
                            auralis_core::Decibels::new(0.0).unwrap()
                        )),
                    ],
                )
                .unwrap()],
                RemixLevelMode::Automatic,
                true,
            )
            .unwrap()
        )]
    );
    assert_eq!(
        chain.render_tokens(),
        ["remix", "-a", "-p", "1v0.5,2p-6,2i0"]
    );
}

#[test]
fn remix_gain_modifiers_affect_chain_output() {
    let mut semi = stereo_audio(vec![0.25, 0.75], vec![0.5, -0.5]);
    parse_effect_chain(&["remix", "1v0.5,2"])
        .unwrap()
        .process_buffer(&mut semi)
        .unwrap();

    let mut auto = stereo_audio(vec![0.25, 0.75], vec![0.5, -0.5]);
    parse_effect_chain(&["remix", "-a", "1v0.5,2"])
        .unwrap()
        .process_buffer(&mut auto)
        .unwrap();

    let mut manual = stereo_audio(vec![0.75, 0.5], vec![0.75, 0.75]);
    parse_effect_chain(&["remix", "-m", "1,2"])
        .unwrap()
        .process_buffer(&mut manual)
        .unwrap();

    assert_eq!(semi.as_planar_f32(), &[0.625, -0.125]);
    assert_eq!(auto.as_planar_f32(), &[0.375, 0.125]);
    assert_eq!(manual.as_planar_f32(), &[1.0, 1.0]);
}

#[test]
fn remix_supports_open_ranges() {
    let mut audio = audio_buffer(3, vec![0.0, 0.3, 1.0, 1.3, -1.0, -0.4]);

    parse_effect_chain(&["remix", "-2", "2-"])
        .unwrap()
        .process_buffer(&mut audio)
        .unwrap();

    assert_eq!(audio.channels(), ChannelCount::new(2).unwrap());
    assert_eq!(audio.as_planar_f32(), &[0.5, 0.799_999_95, 0.0, 0.45]);
}

#[test]
fn remix_rejects_missing_invalid_specs_bad_options_and_out_of_bounds_channels() {
    let missing = parse_effect_chain(&["remix"]).unwrap_err();
    assert!(matches!(
        missing,
        EffectChainParseError::CommandParseFailed {
            source: EffectCommandParseError::MissingArgument {
                effect: "remix",
                argument: "out-spec"
            },
            ..
        }
    ));

    let option = parse_effect_chain(&["remix", "-a", "-m", "1,2"]).unwrap_err();
    assert!(matches!(
        option,
        EffectChainParseError::CommandParseFailed {
            source: EffectCommandParseError::InvalidOptionCombination {
                effect: "remix",
                ..
            },
            ..
        }
    ));

    let invalid = parse_effect_chain(&["remix", "0,1"]).unwrap_err();
    assert!(matches!(
        invalid,
        EffectChainParseError::CommandParseFailed {
            source: EffectCommandParseError::InvalidEffectConfig {
                effect: "remix",
                argument: "out-spec",
                ..
            },
            ..
        }
    ));

    let mut audio = mono_audio(vec![0.25, -0.5]);
    let out_of_bounds = parse_effect_chain(&["remix", "2"])
        .unwrap()
        .process_buffer(&mut audio)
        .unwrap_err();
    assert!(matches!(
        out_of_bounds,
        auralis_effects::EffectChainError::CommandFailed {
            command,
            argument: "out-spec",
            ..
        } if matches!(command.as_ref(), EffectCommand::Remix(_))
    ));
}

fn mono_audio(samples: Vec<f32>) -> auralis_core::AudioBuffer {
    audio_buffer(1, samples)
}

fn stereo_audio(left: Vec<f32>, right: Vec<f32>) -> auralis_core::AudioBuffer {
    assert_eq!(left.len(), right.len());
    audio_buffer(2, left.into_iter().chain(right).collect())
}

fn audio_buffer(channels: u16, samples: Vec<f32>) -> auralis_core::AudioBuffer {
    let channels = ChannelCount::new(channels).unwrap();
    let frames = samples.len() / channels.as_usize();
    let spec = AudioSpec::new(
        SampleRate::new(48_000).unwrap(),
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

//! Integration coverage for SoX-ng-style basic remix routing.

use auralis_core::{AudioSpec, ChannelCount, FrameCount, SampleFormat, SampleRate};
use auralis_effects::{
    EffectChainParseError, EffectCommand, EffectCommandParseError, Remix, RemixOutputSpec,
    RemixSource, parse_effect_chain,
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
fn remix_rejects_missing_future_options_invalid_specs_and_out_of_bounds_channels() {
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

    let option = parse_effect_chain(&["remix", "-a", "1,2"]).unwrap_err();
    assert!(matches!(
        option,
        EffectChainParseError::CommandParseFailed {
            source: EffectCommandParseError::UnsupportedOption { effect: "remix", option },
            ..
        } if option == "-a"
    ));

    let gain_modifier = parse_effect_chain(&["remix", "1v0.5"]).unwrap_err();
    assert!(matches!(
        gain_modifier,
        EffectChainParseError::CommandParseFailed {
            source: EffectCommandParseError::UnsupportedOption { effect: "remix", option },
            ..
        } if option == "v"
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
            command: EffectCommand::Remix(_),
            argument: "out-spec",
            ..
        }
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

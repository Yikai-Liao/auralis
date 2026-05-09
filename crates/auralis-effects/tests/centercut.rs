//! Integration coverage for SoX-ng-style center-cut stereo separation.

use auralis_core::{AudioSpec, ChannelCount, FrameCount, SampleFormat, SampleRate};
use auralis_effects::{
    Centercut, EffectChainParseError, EffectCommand, EffectCommandParseError, parse_effect_chain,
};

#[test]
fn centercut_chain_parses_and_renders_options() {
    let chain = parse_effect_chain(&["centercut", "-a", "0.5", "-b", "-w", "16"]).unwrap();

    assert_eq!(
        chain.commands(),
        &[EffectCommand::Centercut(
            Centercut::with_options(0.5, true, 16).unwrap()
        )]
    );
    assert_eq!(
        chain.render_tokens(),
        ["centercut", "-a", "0.5", "-b", "-w", "16"]
    );
}

#[test]
fn centercut_emits_three_channel_chain_output() {
    let mut audio = stereo_buffer(vec![0.25; 64], vec![0.25; 64]);

    parse_effect_chain(&["centercut"])
        .unwrap()
        .process_buffer(&mut audio)
        .unwrap();

    assert_eq!(audio.channels(), ChannelCount::new(3).unwrap());
    assert_eq!(audio.frames(), FrameCount::new(64));
}

#[test]
fn centercut_options_affect_typed_output() {
    let audio = stereo_buffer(sine(256, 3.0, 0.5), sine(256, 3.0, 0.5));
    let default = Centercut::new().process_buffer(&audio).unwrap();
    let gained = Centercut::with_options(0.25, false, 8192)
        .unwrap()
        .process_buffer(&audio)
        .unwrap();

    assert_close_scaled(
        gained.as_planar_f32(),
        default.as_planar_f32(),
        0.25,
        0.000_001,
    );
}

#[test]
fn centercut_rejects_non_stereo_input_in_chain() {
    let mut audio = audio_buffer(1, vec![0.25, -0.5]);
    let error = parse_effect_chain(&["centercut"])
        .unwrap()
        .process_buffer(&mut audio)
        .unwrap_err();

    assert!(matches!(
        error,
        auralis_effects::EffectChainError::CommandFailed {
            command: EffectCommand::Centercut(_),
            argument: "channels",
            ..
        }
    ));
}

#[test]
fn centercut_rejects_invalid_command_options() {
    let extra = parse_effect_chain(&["centercut", "extra"]).unwrap_err();
    assert!(matches!(
        extra,
        EffectChainParseError::CommandParseFailed {
            source: EffectCommandParseError::UnexpectedArgument {
                effect: "centercut",
                argument,
            },
            ..
        } if argument == "extra"
    ));

    let option = parse_effect_chain(&["centercut", "-x"]).unwrap_err();
    assert!(matches!(
        option,
        EffectChainParseError::CommandParseFailed {
            source: EffectCommandParseError::UnsupportedOption {
                effect: "centercut",
                option,
            },
            ..
        } if option == "-x"
    ));
}

fn stereo_buffer(left: Vec<f32>, right: Vec<f32>) -> auralis_core::AudioBuffer {
    assert_eq!(left.len(), right.len());
    let mut samples = left;
    samples.extend(right);
    audio_buffer(2, samples)
}

fn audio_buffer(channels: u16, samples: Vec<f32>) -> auralis_core::AudioBuffer {
    assert_eq!(samples.len() % usize::from(channels), 0);
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

#[allow(
    clippy::cast_possible_truncation,
    clippy::cast_precision_loss,
    reason = "Test fixtures use small frame counts and intentional f32 sample output."
)]
fn sine(frames: usize, cycles: f64, amplitude: f32) -> Vec<f32> {
    (0..frames)
        .map(|frame| {
            (std::f64::consts::TAU * cycles * frame as f64 / frames as f64).sin() as f32 * amplitude
        })
        .collect()
}

fn assert_close_scaled(actual: &[f32], expected: &[f32], scale: f32, epsilon: f32) {
    assert_eq!(actual.len(), expected.len());
    for (index, (&actual, &expected)) in actual.iter().zip(expected).enumerate() {
        assert!(
            (actual - expected * scale).abs() <= epsilon,
            "sample {index}: expected {}, got {actual}",
            expected * scale
        );
    }
}

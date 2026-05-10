//! Integration coverage for SoX-ng-style basic waveform synthesis.

use auralis_core::{AudioBuffer, AudioSpec, ChannelCount, FrameCount, SampleFormat, SampleRate};
use auralis_effects::{
    EffectChainParseError, EffectCommand, EffectCommandParseError, EffectError, Synth,
    SynthChannel, SynthLength, SynthWaveform, parse_effect_chain, parse_effect_command,
};

#[test]
fn synth_command_parses_renders_and_groups_with_next_effect() {
    assert_eq!(
        parse_effect_command(&["synth"]).unwrap(),
        EffectCommand::Synth(Synth::new().unwrap())
    );
    assert_eq!(
        parse_effect_command(&["synth", "-n", "4s", "sine", "1"])
            .unwrap()
            .render_tokens(),
        ["synth", "-n", "4s", "sine", "1"]
    );

    let chain = parse_effect_chain(&["synth", "4s", "sine", "1", "reverse"]).unwrap();

    assert_eq!(chain.len(), 2);
    assert_eq!(
        chain.commands()[0].render_tokens(),
        ["synth", "4s", "sine", "1"]
    );
    assert_eq!(chain.commands()[1].render_tokens(), ["reverse"]);
}

#[test]
fn synth_chain_execution_replaces_audio_with_generated_waveform() {
    let mut audio = audio_buffer(4, 1, vec![0.25, 0.25, 0.25, 0.25]);

    parse_effect_chain(&["synth", "sine", "1"])
        .unwrap()
        .process_buffer(&mut audio)
        .unwrap();

    assert_samples_close(audio.as_planar_f32(), &[0.0, 1.0, 0.0, -1.0]);
}

#[test]
fn synth_can_set_output_length_and_repeat_channel_specs() {
    let audio = audio_buffer(4, 2, vec![0.0, 0.0, 0.0, 0.0]);
    let synth = Synth::with_channels(
        Some(SynthLength::frames(FrameCount::new(2))),
        [SynthChannel::new(SynthWaveform::Square, 1.0).unwrap()],
    )
    .unwrap();

    let output = synth.process_buffer(&audio).unwrap();

    assert_eq!(output.frames(), FrameCount::new(2));
    assert_samples_close(output.as_planar_f32(), &[1.0, 1.0, 1.0, 1.0]);
}

#[test]
fn synth_rejects_deferred_noise_sweep_and_combine_modes() {
    let noise = parse_effect_chain(&["synth", "noise"]).unwrap_err();
    assert!(matches!(
        noise,
        EffectChainParseError::CommandParseFailed {
            source: EffectCommandParseError::UnsupportedOption {
                effect: "synth",
                ..
            },
            ..
        }
    ));

    let sweep = parse_effect_chain(&["synth", "sine", "440:880"]).unwrap_err();
    assert!(matches!(
        sweep,
        EffectChainParseError::CommandParseFailed {
            source: EffectCommandParseError::InvalidNumber {
                effect: "synth",
                argument: "frequency",
                ..
            },
            ..
        }
    ));

    let combine = parse_effect_chain(&["synth", "sine", "mix"]).unwrap_err();
    assert!(matches!(
        combine,
        EffectChainParseError::CommandParseFailed {
            source: EffectCommandParseError::UnsupportedOption {
                effect: "synth",
                option,
            },
            ..
        } if option == "mix"
    ));
}

#[test]
fn synth_rejects_invalid_typed_configs() {
    assert_eq!(
        SynthChannel::new(SynthWaveform::Sine, -1.0).unwrap_err(),
        EffectError::InvalidSynth
    );
    assert_eq!(
        SynthChannel::with_parameters(
            SynthWaveform::Triangle,
            440.0,
            0.0,
            0.0,
            Some(0.0),
            None,
            None,
        )
        .unwrap_err(),
        EffectError::InvalidSynth
    );
}

fn audio_buffer(sample_rate: u32, channels: u16, samples: Vec<f32>) -> AudioBuffer {
    let spec = AudioSpec::new(
        SampleRate::new(sample_rate).unwrap(),
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
        assert!(
            (actual - expected).abs() <= 0.000_001,
            "sample {index}: actual={actual}, expected={expected}"
        );
    }
}

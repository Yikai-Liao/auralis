//! Integration coverage for the typed VAD core.

use auralis_core::{AudioBuffer, AudioSpec, ChannelCount, FrameCount, SampleFormat, SampleRate};
use auralis_effects::{
    EffectChain, EffectCommand, Vad, VadOptions, parse_effect_chain, parse_effect_command,
};

#[test]
fn vad_trims_leading_non_voice() {
    let source = mono_audio_buffer(vec![0.0, 0.01, 0.03, 0.02, -0.04]);
    let trimmed = Vad::default().process_buffer(&source).unwrap();

    assert_eq!(trimmed.as_planar_f32(), &[0.03, 0.02, -0.04]);
    assert_eq!(trimmed.frames(), FrameCount::new(3));
    assert_eq!(trimmed.channels(), source.channels());
}

#[test]
fn vad_returns_empty_buffer_when_no_voice_is_detected() {
    let source = stereo_audio_buffer(vec![0.0, 0.01, -0.01, 0.0]);
    let trimmed = Vad::default().process_buffer(&source).unwrap();

    assert!(trimmed.as_planar_f32().is_empty());
    assert_eq!(trimmed.frames(), FrameCount::new(0));
    assert_eq!(trimmed.channels(), source.channels());
}

#[test]
fn vad_retains_configured_pre_trigger_frames() {
    let source = mono_audio_buffer(vec![0.0, 0.0, 0.01, 0.03, 0.05]);
    let vad = Vad::new(
        0.02,
        FrameCount::new(1),
        FrameCount::new(2),
        FrameCount::new(0),
    )
    .unwrap();
    let trimmed = vad.process_buffer(&source).unwrap();

    assert_eq!(trimmed.as_planar_f32(), &[0.0, 0.01, 0.03, 0.05]);
}

#[test]
fn vad_trigger_can_tolerate_short_quiet_gaps() {
    let source = mono_audio_buffer(vec![0.0, 0.03, 0.0, 0.04, 0.0]);
    let vad = Vad::new(
        0.02,
        FrameCount::new(2),
        FrameCount::new(0),
        FrameCount::new(1),
    )
    .unwrap();
    let trimmed = vad.process_buffer(&source).unwrap();

    assert_eq!(trimmed.as_planar_f32(), &[0.03, 0.0, 0.04, 0.0]);
}

#[test]
fn vad_checks_all_channels_for_voice() {
    let source = stereo_audio_buffer(vec![0.0, 0.0, 0.0, 0.0, 0.03, 0.04]);
    let trimmed = Vad::default().process_buffer(&source).unwrap();

    assert_eq!(trimmed.as_planar_f32(), &[0.0, 0.0, 0.03, 0.04]);
    assert_eq!(trimmed.frames(), FrameCount::new(2));
    assert_eq!(trimmed.channels(), source.channels());
}

#[test]
fn vad_command_parses_and_renders_sox_options() {
    let command =
        parse_effect_command(&["vad", "-T", "0.01", "-t", "1", "-g", "0.1", "-p", "0.001"])
            .unwrap();

    assert_eq!(
        command.render_tokens(),
        ["vad", "-T", "0.01", "-t", "1", "-g", "0.1", "-p", "0.001"]
    );
    let EffectCommand::Vad(vad) = command else {
        panic!("expected vad command");
    };
    let options = vad.sox_options().unwrap();
    assert_eq!(options.trigger_time.to_bits(), 0.01_f64.to_bits());
    assert_eq!(options.trigger_level.to_bits(), 1.0_f64.to_bits());
}

#[test]
fn vad_command_rejects_out_of_range_options() {
    let error = parse_effect_command(&["vad", "-t", "21"]).unwrap_err();

    assert!(error.to_string().contains("invalid `vad`"));
}

#[test]
fn vad_command_executes_in_chain() {
    let source = mono_audio_buffer(vec![0.0, 0.0, 0.1, 0.2]);
    let mut audio = source.clone();
    let command = EffectCommand::Vad(
        Vad::from_sox_options(VadOptions {
            trigger_time: 0.01,
            trigger_level: 1.0,
            ..VadOptions::default()
        })
        .unwrap(),
    );
    let chain = EffectChain::new(vec![command]);

    chain.process_buffer(&mut audio).unwrap();

    assert_eq!(audio.as_planar_f32(), &[0.1, 0.2]);
    assert_eq!(audio.frames(), FrameCount::new(2));
}

#[test]
fn vad_positional_chain_groups_option_values() {
    let chain = parse_effect_chain(&["vad", "-T", "0.01", "-t", "1", "reverse"]).unwrap();

    assert_eq!(chain.len(), 2);
    assert_eq!(
        chain.commands()[0].render_tokens(),
        ["vad", "-T", "0.01", "-t", "1"]
    );
    assert_eq!(chain.commands()[1].render_tokens(), ["reverse"]);
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

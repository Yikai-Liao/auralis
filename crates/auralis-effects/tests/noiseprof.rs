//! Integration coverage for SoX-ng-style noise profile collection.

use auralis_core::{AudioBuffer, AudioSpec, ChannelCount, FrameCount, SampleFormat, SampleRate};
use auralis_effects::{
    EffectCommand, NOISE_PROFILE_FREQ_COUNT, NoiseProf, parse_effect_chain, parse_effect_command,
};

#[test]
fn noiseprof_command_parses_and_renders_default_and_path() {
    let default = parse_effect_command(&["noiseprof"]).unwrap();
    assert_eq!(default.render_tokens(), ["noiseprof", "-"]);
    assert_eq!(default, EffectCommand::NoiseProf(NoiseProf::stdout()));

    let explicit = parse_effect_command(&["noiseprof", "profile.prof"]).unwrap();
    assert_eq!(explicit.render_tokens(), ["noiseprof", "profile.prof"]);
    let EffectCommand::NoiseProf(noiseprof) = explicit else {
        panic!("expected noiseprof command");
    };
    assert_eq!(noiseprof.output_path(), Some("profile.prof"));
}

#[test]
fn noiseprof_chain_groups_one_optional_profile_path() {
    let chain = parse_effect_chain(&["noiseprof", "profile.prof", "reverse"]).unwrap();

    assert_eq!(chain.len(), 2);
    assert_eq!(
        chain.commands()[0].render_tokens(),
        ["noiseprof", "profile.prof"]
    );
    assert_eq!(chain.commands()[1].render_tokens(), ["reverse"]);
}

#[test]
fn noiseprof_chain_execution_passes_audio_through() {
    let mut audio = mono_audio_buffer(vec![0.0, 0.25, -0.5, 0.5]);
    let original = audio.clone();

    parse_effect_chain(&["noiseprof"])
        .unwrap()
        .process_buffer(&mut audio)
        .unwrap();

    assert_eq!(audio, original);
}

#[test]
fn noiseprof_collects_zero_profile_for_silence() {
    let audio = mono_audio_buffer(vec![0.0; 8]);
    let profile = NoiseProf::default().profile(&audio).unwrap();

    assert_eq!(profile.channels().len(), 1);
    assert_eq!(profile.channels()[0], vec![0.0; NOISE_PROFILE_FREQ_COUNT]);
}

#[test]
fn noiseprof_render_text_is_sox_style_channel_major() {
    let audio = stereo_audio_buffer(vec![0.0, 0.25, 0.5, -0.5]);
    let text = NoiseProf::default().profile(&audio).unwrap().render_text();

    assert!(text.starts_with("Channel 0: "));
    assert!(text.contains("\nChannel 1: "));
    assert_eq!(text.lines().count(), 2);
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

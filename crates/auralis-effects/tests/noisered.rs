//! Integration coverage for SoX-ng-style noise reduction.

use auralis_core::{AudioBuffer, AudioSpec, ChannelCount, FrameCount, SampleFormat, SampleRate};
use auralis_effects::{
    EffectCommand, NOISE_PROFILE_FREQ_COUNT, NoiseProfile, NoiseRed, parse_effect_chain,
    parse_effect_command,
};

#[test]
fn noisered_command_parses_and_renders_defaults() {
    let default = parse_effect_command(&["noisered"]).unwrap();
    assert_eq!(default.render_tokens(), ["noisered", "-", "0.5"]);
    assert_eq!(
        default,
        EffectCommand::NoiseRed(NoiseRed::from_profile_path(None, 0.5).unwrap())
    );

    let explicit = parse_effect_command(&["noisered", "profile.prof", "0.25"]).unwrap();
    assert_eq!(
        explicit.render_tokens(),
        ["noisered", "profile.prof", "0.25"]
    );
}

#[test]
fn noisered_chain_groups_profile_and_amount() {
    let chain = parse_effect_chain(&["noisered", "profile.prof", "0.25", "reverse"]).unwrap();

    assert_eq!(chain.len(), 2);
    assert_eq!(
        chain.commands()[0].render_tokens(),
        ["noisered", "profile.prof", "0.25"]
    );
    assert_eq!(chain.commands()[1].render_tokens(), ["reverse"]);
}

#[test]
fn noisered_rejects_invalid_amount() {
    assert!(parse_effect_command(&["noisered", "profile.prof", "1.25"]).is_err());
}

#[test]
fn noisered_typed_processing_matches_sox_window_length_shape() {
    let profile = NoiseProfile::new(vec![vec![0.0; NOISE_PROFILE_FREQ_COUNT]]).unwrap();
    let processor = NoiseRed::from_profile(profile, 0.5).unwrap();
    let audio = mono_audio_buffer(vec![0.0; 4096]);

    let reduced = processor.process_buffer(&audio).unwrap();

    assert_eq!(reduced.frames(), FrameCount::new(3072));
    assert!(reduced.as_planar_f32().iter().all(|sample| *sample == 0.0));
}

fn mono_audio_buffer(samples: Vec<f32>) -> AudioBuffer {
    let spec = AudioSpec::new(
        SampleRate::new(48_000).unwrap(),
        ChannelCount::new(1).unwrap(),
        SampleFormat::Float32,
    );
    AudioBuffer::from_planar_f32(
        spec,
        FrameCount::new(u64::try_from(samples.len()).unwrap()),
        samples,
    )
    .unwrap()
}

//! Integration coverage for the typed VAD core.

use auralis_core::{AudioBuffer, AudioSpec, ChannelCount, FrameCount, SampleFormat, SampleRate};
use auralis_effects::Vad;

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

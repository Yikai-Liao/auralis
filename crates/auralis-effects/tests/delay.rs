//! Integration coverage for SoX-ng-style per-channel delay.

use auralis_core::{AudioBuffer, AudioSpec, ChannelCount, FrameCount, SampleFormat, SampleRate};
use auralis_effects::{
    Delay, DelayAmount, DelayAnchor, DelayPosition, EffectCommand, EffectCommandParseError,
    EffectError, parse_effect_chain, parse_effect_command,
};

#[test]
fn typed_delay_matches_chain_delay_command() {
    let source = stereo_buffer(&[0.25, 0.5], &[-0.25, -0.5]);
    let expected = Delay::new([FrameCount::new(2), FrameCount::new(1)])
        .process_buffer(&source)
        .unwrap();

    let mut chained = source;
    parse_effect_chain(&["delay", "2s", "1s"])
        .unwrap()
        .process_buffer(&mut chained)
        .unwrap();

    assert_eq!(chained, expected);
}

#[test]
fn command_parser_accepts_frame_seconds_and_relative_positions() {
    assert_eq!(
        parse_effect_command(&["delay", "2s", "0.001", "+1s"]).unwrap(),
        EffectCommand::Delay(Delay::with_positions([
            DelayPosition::frames(FrameCount::new(2)),
            DelayPosition::seconds(0.001).unwrap(),
            DelayPosition::new(
                DelayAnchor::Previous,
                DelayAmount::Frames(FrameCount::new(1))
            ),
        ]))
    );
    assert_eq!(
        parse_effect_command(&["delay", "2s", "1s"])
            .unwrap()
            .render_tokens(),
        ["delay", "2s", "1s"]
    );
}

#[test]
fn seconds_positions_resolve_using_input_sample_rate() {
    let source = mono_buffer(1_000, &[0.25, 0.5]);

    let delayed = Delay::with_positions([DelayPosition::seconds(0.002).unwrap()])
        .process_buffer(&source)
        .unwrap();

    assert_eq!(delayed.frames(), FrameCount::new(4));
    assert_eq!(delayed.as_planar_f32(), &[0.0, 0.0, 0.25, 0.5]);
}

#[test]
fn rejects_invalid_positions_and_too_many_channels() {
    assert!(matches!(
        parse_effect_command(&["delay", "abc"]).unwrap_err(),
        EffectCommandParseError::InvalidDelayPosition { .. }
    ));

    let source = mono_buffer(48_000, &[0.25]);
    let error = Delay::new([FrameCount::new(1), FrameCount::new(2)])
        .process_buffer(&source)
        .unwrap_err();

    assert_eq!(error, EffectError::DelayTooManyPositions);
}

fn mono_buffer(sample_rate_hz: u32, samples: &[f32]) -> AudioBuffer {
    let spec = AudioSpec::new(
        SampleRate::new(sample_rate_hz).unwrap(),
        ChannelCount::new(1).unwrap(),
        SampleFormat::Float32,
    );
    AudioBuffer::from_planar_f32(
        spec,
        FrameCount::new(u64::try_from(samples.len()).unwrap()),
        samples.to_vec(),
    )
    .unwrap()
}

fn stereo_buffer(left: &[f32], right: &[f32]) -> AudioBuffer {
    assert_eq!(left.len(), right.len());
    let spec = AudioSpec::new(
        SampleRate::new(48_000).unwrap(),
        ChannelCount::new(2).unwrap(),
        SampleFormat::Float32,
    );
    let mut samples = Vec::with_capacity(left.len() + right.len());
    samples.extend_from_slice(left);
    samples.extend_from_slice(right);
    AudioBuffer::from_planar_f32(
        spec,
        FrameCount::new(u64::try_from(left.len()).unwrap()),
        samples,
    )
    .unwrap()
}

//! Integration coverage for SoX-ng-style sample statistics.

use auralis_core::{AudioBuffer, AudioSpec, ChannelCount, FrameCount, SampleFormat, SampleRate};
use auralis_effects::{
    EffectChainParseError, EffectCommand, EffectCommandParseError, EffectError, Stat,
    parse_effect_chain, parse_effect_command,
};

#[test]
fn stat_command_parses_renders_and_groups_with_next_effect() {
    assert_eq!(
        parse_effect_command(&["stat"]).unwrap(),
        EffectCommand::Stat(Stat::new())
    );
    assert_eq!(
        parse_effect_command(&["stat", "-s", "2", "-rms", "-v", "-j"])
            .unwrap()
            .render_tokens(),
        ["stat", "-s", "2", "-rms", "-v", "-j"]
    );

    let chain = parse_effect_chain(&["stat", "-s", "2", "-j", "reverse"]).unwrap();

    assert_eq!(chain.len(), 2);
    assert_eq!(
        chain.commands()[0].render_tokens(),
        ["stat", "-s", "2", "-j"]
    );
    assert_eq!(chain.commands()[1].render_tokens(), ["reverse"]);
}

#[test]
fn stat_chain_execution_passes_audio_through() {
    let mut audio = audio_buffer(vec![0.0, 0.25, -0.5, 0.5], 1);
    let original = audio.clone();

    parse_effect_chain(&["stat"])
        .unwrap()
        .process_buffer(&mut audio)
        .unwrap();

    assert_eq!(audio, original);
}

#[test]
fn stat_report_uses_frame_major_order_for_multichannel_deltas() {
    let audio = audio_buffer(vec![0.5, -0.5, 0.25, -0.25], 2);
    let report = Stat::new().report(&audio).unwrap();

    assert_eq!(report.samples_read(), 4);
    assert!((report.maximum_amplitude() - 0.5).abs() < f64::EPSILON);
    assert!((report.minimum_amplitude() + 0.5).abs() < f64::EPSILON);
    assert_eq!(report.volume_adjustment(), Some(2.0));
    assert!(
        report
            .render_text()
            .contains("Samples read:                 4")
    );
    assert!(report.render_json().contains("\"rough_frequency\""));
}

#[test]
fn stat_rejects_unsupported_options_bad_scale_and_non_finite_samples() {
    let unsupported = parse_effect_chain(&["stat", "-freq"]).unwrap_err();
    assert!(matches!(
        unsupported,
        EffectChainParseError::CommandParseFailed {
            source: EffectCommandParseError::UnsupportedOption { effect: "stat", .. },
            ..
        }
    ));

    let bad_scale = parse_effect_chain(&["stat", "-s", "0"]).unwrap_err();
    assert!(matches!(
        bad_scale,
        EffectChainParseError::CommandParseFailed {
            source: EffectCommandParseError::InvalidEffectConfig {
                source: EffectError::InvalidStat,
                ..
            },
            ..
        }
    ));

    assert_eq!(
        Stat::new().report(&audio_buffer(vec![f32::INFINITY], 1)),
        Err(EffectError::InvalidStat)
    );
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

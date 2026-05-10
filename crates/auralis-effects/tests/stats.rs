//! Integration coverage for SoX-ng-style per-channel sample statistics.

use auralis_core::{AudioBuffer, AudioSpec, ChannelCount, FrameCount, SampleFormat, SampleRate};
use auralis_effects::{
    EffectChainParseError, EffectCommand, EffectCommandParseError, EffectError, Stats,
    StatsDisplayScale, parse_effect_chain, parse_effect_command,
};

#[test]
fn stats_command_parses_renders_and_groups_with_next_effect() {
    assert_eq!(
        parse_effect_command(&["stats"]).unwrap(),
        EffectCommand::Stats(Stats::new())
    );
    assert_eq!(
        parse_effect_command(&["stats", "-x", "16", "-w", "0.1", "-j"])
            .unwrap()
            .render_tokens(),
        ["stats", "-x", "16", "-w", "0.1", "-j"]
    );

    let chain = parse_effect_chain(&["stats", "-b", "16", "-j", "reverse"]).unwrap();

    assert_eq!(chain.len(), 2);
    assert_eq!(
        chain.commands()[0].render_tokens(),
        ["stats", "-b", "16", "-j"]
    );
    assert_eq!(chain.commands()[1].render_tokens(), ["reverse"]);
}

#[test]
fn stats_chain_execution_passes_audio_through() {
    let mut audio = audio_buffer(vec![0.0, 0.25, -0.5, 0.5], 1);
    let original = audio.clone();

    parse_effect_chain(&["stats"])
        .unwrap()
        .process_buffer(&mut audio)
        .unwrap();

    assert_eq!(audio, original);
}

#[test]
fn stats_report_exposes_overall_and_channel_summaries() {
    let audio = audio_buffer(vec![0.5, -0.5, 0.25, -0.25], 2);
    let report = Stats::new().report(&audio).unwrap();

    assert_eq!(report.channel_count(), 2);
    assert_eq!(report.num_samples(), 2);
    assert_eq!(report.channels().len(), 2);
    assert!((report.overall().max_level() - 0.5).abs() < f64::EPSILON);
    assert!((report.overall().min_level() + 0.5).abs() < f64::EPSILON);
    assert_eq!(report.overall().peak_count(), 4);
    assert!(
        report
            .render_text(Stats::new())
            .contains("Overall     Left      Right")
    );
    assert!(report.render_json().contains("\"channels\""));
}

#[test]
fn stats_rejects_bad_options_and_non_finite_samples() {
    let unsupported = parse_effect_chain(&["stats", "-rms"]).unwrap_err();
    assert!(matches!(
        unsupported,
        EffectChainParseError::CommandParseFailed {
            source: EffectCommandParseError::UnsupportedOption {
                effect: "stats",
                ..
            },
            ..
        }
    ));

    let bad_bits = parse_effect_chain(&["stats", "-b", "1"]).unwrap_err();
    assert!(matches!(
        bad_bits,
        EffectChainParseError::CommandParseFailed {
            source: EffectCommandParseError::InvalidEffectConfig {
                source: EffectError::InvalidStats,
                ..
            },
            ..
        }
    ));

    let stats = Stats::new()
        .with_scale(2.0)
        .unwrap()
        .with_hex_bits(16)
        .unwrap();
    assert_eq!(stats.display_scale(), StatsDisplayScale::HexBits(16));
    assert_eq!(
        Stats::new().report(&audio_buffer(vec![f32::INFINITY], 1)),
        Err(EffectError::InvalidStats)
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

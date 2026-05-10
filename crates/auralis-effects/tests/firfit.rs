//! Integration coverage for SoX-ng-style FIR response fitting.

use std::{fs, time::SystemTime};

use auralis_core::{AudioBuffer, AudioSpec, ChannelCount, FrameCount, SampleFormat, SampleRate};
use auralis_effects::{
    EffectChainError, EffectCommand, EffectError, FirFit, FirFitKnot, FirFitKnotSource,
    parse_effect_chain, parse_effect_command,
};

#[test]
fn firfit_command_parses_and_renders_stdin_file_and_inline_knots() {
    let default = parse_effect_command(&["firfit"]).unwrap();
    assert_eq!(default.render_tokens(), ["firfit", "-"]);
    assert_eq!(default, EffectCommand::FirFit(FirFit::stdin()));

    let file = parse_effect_command(&["firfit", "knots.txt"]).unwrap();
    assert_eq!(file.render_tokens(), ["firfit", "knots.txt"]);
    let EffectCommand::FirFit(firfit) = file else {
        panic!("expected firfit command");
    };
    assert_eq!(
        firfit.source(),
        &FirFitKnotSource::File("knots.txt".to_owned())
    );

    let inline = parse_effect_command(&["firfit", "20", "0", "10k", "-3"]).unwrap();
    assert_eq!(inline.render_tokens(), ["firfit", "20", "0", "10000", "-3"]);
}

#[test]
fn firfit_text_parser_accepts_comments_and_multiple_pairs_per_line() {
    let knots = FirFit::parse_knot_text("300 -100 3k 0\n# tail\n10k -6\n").unwrap();

    assert_eq!(
        knots,
        [
            FirFitKnot::new(300.0, -100.0).unwrap(),
            FirFitKnot::new(3_000.0, 0.0).unwrap(),
            FirFitKnot::new(10_000.0, -6.0).unwrap(),
        ]
    );
}

#[test]
fn firfit_chain_groups_inline_knots_before_next_effect() {
    let chain = parse_effect_chain(&["firfit", "20", "0", "10k", "0", "reverse"]).unwrap();

    assert_eq!(chain.len(), 2);
    assert_eq!(
        chain.commands()[0].render_tokens(),
        ["firfit", "20", "0", "10000", "0"]
    );
    assert_eq!(chain.commands()[1].render_tokens(), ["reverse"]);
}

#[test]
fn firfit_chain_groups_single_argument_as_file_path() {
    let chain = parse_effect_chain(&["firfit", "knots.txt", "reverse"]).unwrap();

    assert_eq!(chain.len(), 2);
    assert_eq!(chain.commands()[0].render_tokens(), ["firfit", "knots.txt"]);
    assert_eq!(chain.commands()[1].render_tokens(), ["reverse"]);
}

#[test]
fn firfit_flat_zero_db_response_preserves_audio() {
    let mut audio = mono_audio_buffer(vec![0.25, -0.5, 0.0, 0.75]);
    let original = audio.clone();

    parse_effect_chain(&["firfit", "20", "0", "10k", "0"])
        .unwrap()
        .process_buffer(&mut audio)
        .unwrap();

    assert_eq!(audio, original);
}

#[test]
fn firfit_chain_execution_loads_knots_from_file() {
    let path = temp_path("auralis-firfit-knots", "txt");
    fs::write(&path, "20 -6\n10000 -6\n").unwrap();
    let mut audio = mono_audio_buffer(vec![1.0, -1.0, 0.5]);

    parse_effect_chain(&["firfit", path.to_str().unwrap()])
        .unwrap()
        .process_buffer(&mut audio)
        .unwrap();

    fs::remove_file(path).unwrap();
    assert_samples_close(
        audio.as_planar_f32(),
        &[0.501_187_2, -0.501_187_2, 0.250_593_6],
    );
}

#[test]
fn firfit_rejects_invalid_inline_or_stdin_processing() {
    assert!(parse_effect_command(&["firfit", "20", "0", "10k"]).is_err());
    assert!(parse_effect_command(&["firfit", "20", "0", "10", "-3"]).is_err());

    let mut audio = mono_audio_buffer(vec![0.0]);
    let error = parse_effect_chain(&["firfit"])
        .unwrap()
        .process_buffer(&mut audio)
        .unwrap_err();

    assert!(matches!(
        error,
        EffectChainError::CommandFailed {
            source: EffectError::InvalidFirFit,
            ..
        }
    ));
}

#[test]
fn firfit_non_flat_design_stays_finite_and_length_preserving() {
    let firfit = FirFit::parse_sox_args(&["20", "0", "1000", "-3", "10000", "-9"]).unwrap();
    let source = mono_audio_buffer(vec![0.25, -0.5, 0.75, -0.25, 0.0, 0.5]);

    let filtered = firfit.process_buffer(&source).unwrap();

    assert_eq!(filtered.frames(), source.frames());
    assert_eq!(filtered.channels(), source.channels());
    assert!(
        filtered
            .as_planar_f32()
            .iter()
            .all(|sample| sample.is_finite())
    );
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

fn assert_samples_close(actual: &[f32], expected: &[f32]) {
    assert_eq!(actual.len(), expected.len());
    for (index, (&actual, &expected)) in actual.iter().zip(expected).enumerate() {
        let difference = (actual - expected).abs();
        assert!(
            difference <= 1.0e-6,
            "sample {index} differed by {difference}: {actual} != {expected}"
        );
    }
}

fn temp_path(prefix: &str, extension: &str) -> std::path::PathBuf {
    let nanos = SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .unwrap()
        .as_nanos();

    std::env::temp_dir().join(format!("{prefix}-{nanos}.{extension}"))
}

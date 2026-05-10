//! Integration coverage for SoX-ng-style FIR filtering.

use std::{fs, time::SystemTime};

use auralis_core::{AudioBuffer, AudioSpec, ChannelCount, FrameCount, SampleFormat, SampleRate};
use auralis_effects::{
    EffectChainError, EffectCommand, EffectError, Fir, FirCoefficientSource, FirCoefficients,
    FirState, parse_effect_chain, parse_effect_command,
};

#[test]
fn fir_command_parses_and_renders_stdin_file_and_inline_coefficients() {
    let default = parse_effect_command(&["fir"]).unwrap();
    assert_eq!(default.render_tokens(), ["fir", "-"]);
    assert_eq!(default, EffectCommand::Fir(Fir::stdin()));

    let file = parse_effect_command(&["fir", "coeffs.txt"]).unwrap();
    assert_eq!(file.render_tokens(), ["fir", "coeffs.txt"]);
    let EffectCommand::Fir(fir) = file else {
        panic!("expected fir command");
    };
    assert_eq!(
        fir.source(),
        &FirCoefficientSource::File("coeffs.txt".to_owned())
    );

    let inline = parse_effect_command(&["fir", "0.5", "0.25"]).unwrap();
    assert_eq!(inline.render_tokens(), ["fir", "0.5", "0.25"]);
}

#[test]
fn fir_chain_groups_inline_coefficients_before_next_effect() {
    let chain = parse_effect_chain(&["fir", "0.5", "0.25", "reverse"]).unwrap();

    assert_eq!(chain.len(), 2);
    assert_eq!(chain.commands()[0].render_tokens(), ["fir", "0.5", "0.25"]);
    assert_eq!(chain.commands()[1].render_tokens(), ["reverse"]);
}

#[test]
fn fir_chain_groups_single_argument_as_file_path() {
    let chain = parse_effect_chain(&["fir", "coeffs.txt", "reverse"]).unwrap();

    assert_eq!(chain.len(), 2);
    assert_eq!(chain.commands()[0].render_tokens(), ["fir", "coeffs.txt"]);
    assert_eq!(chain.commands()[1].render_tokens(), ["reverse"]);
}

#[test]
fn fir_chain_execution_filters_audio_with_inline_coefficients() {
    let mut audio = mono_audio_buffer(vec![1.0, 0.0, 0.0, 0.0]);

    parse_effect_chain(&["fir", "1", "2", "3"])
        .unwrap()
        .process_buffer(&mut audio)
        .unwrap();

    assert_eq!(audio.as_planar_f32(), &[2.0, 3.0, 0.0, 0.0]);
}

#[test]
fn fir_chain_execution_loads_coefficients_from_file() {
    let path = temp_path("auralis-fir-coefficients", "txt");
    fs::write(&path, "0.5 0.25 # trailing\n").unwrap();
    let mut audio = mono_audio_buffer(vec![1.0, 0.0, 0.0, 0.0]);

    parse_effect_chain(&["fir", path.to_str().unwrap()])
        .unwrap()
        .process_buffer(&mut audio)
        .unwrap();

    fs::remove_file(path).unwrap();
    assert_eq!(audio.as_planar_f32(), &[0.5, 0.25, 0.0, 0.0]);
}

#[test]
fn fir_rejects_invalid_inline_or_stdin_processing() {
    assert!(parse_effect_command(&["fir", "coeffs.txt", "extra"]).is_err());

    let mut audio = mono_audio_buffer(vec![0.0]);
    let error = parse_effect_chain(&["fir"])
        .unwrap()
        .process_buffer(&mut audio)
        .unwrap_err();

    assert!(matches!(
        error,
        EffectChainError::CommandFailed {
            source: EffectError::InvalidFirCoefficients,
            ..
        }
    ));
}

#[test]
fn fir_typed_processing_matches_chunked_state_convolution() {
    let coefficients = FirCoefficients::new([0.25, 0.5, 0.25]).unwrap();
    let fir = Fir::from_coefficients(coefficients.clone());
    let source = mono_audio_buffer(vec![0.0, 1.0, 0.5, -0.5, 0.0]);
    let whole = fir.process_buffer(&source).unwrap();
    let mut state = FirState::new(coefficients);
    let mut chunked = Vec::new();

    state.process_mono_samples(&source.as_planar_f32()[..2], &mut chunked);
    state.process_mono_samples(&source.as_planar_f32()[2..3], &mut chunked);
    state.process_mono_samples(&source.as_planar_f32()[3..], &mut chunked);
    state.finish(&mut chunked);

    assert_eq!(whole.as_planar_f32(), chunked.as_slice());
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

fn temp_path(prefix: &str, extension: &str) -> std::path::PathBuf {
    let nanos = SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .unwrap()
        .as_nanos();

    std::env::temp_dir().join(format!("{prefix}-{nanos}.{extension}"))
}

//! Integration coverage for SoX-ng-style RBJ band-pass filtering.

use auralis_core::{AudioBuffer, AudioSpec, ChannelCount, FrameCount, SampleFormat, SampleRate};
use auralis_effects::{
    BandPass, Biquad, BiquadState, BiquadWidth, EffectCommandParseError, EffectError,
    parse_effect_chain,
};

#[test]
fn bandpass_chain_parses_and_renders_supported_forms() {
    let constant_peak = parse_effect_chain(&["bandpass", "1k", "0.707q"]).unwrap();
    let constant_skirt = parse_effect_chain(&["bandpass", "-c", "1000", "500"]).unwrap();

    assert_eq!(
        constant_peak.render_tokens(),
        ["bandpass", "1000", "0.707q"]
    );
    assert_eq!(
        constant_skirt.render_tokens(),
        ["bandpass", "-c", "1000", "500h"]
    );
}

#[test]
fn bandpass_chain_matches_typed_processor() {
    let source = stereo_impulse();
    let band_pass = BandPass::new(1_000.0, BiquadWidth::q(0.707)).unwrap();
    let mut direct = source.clone();
    let mut parsed = source;

    band_pass.process_buffer(&mut direct).unwrap();
    parse_effect_chain(&["bandpass", "1000", "0.707q"])
        .unwrap()
        .process_buffer(&mut parsed)
        .unwrap();

    assert_sample_bits_eq(parsed.as_planar_f32(), direct.as_planar_f32());
}

#[test]
fn bandpass_supports_state_preserving_chunked_processing() {
    let source = stereo_impulse();
    let band_pass = BandPass::constant_skirt(1_000.0, BiquadWidth::q(2.0)).unwrap();
    let coefficients = band_pass
        .coefficients(source.spec().sample_rate())
        .expect("fixture design is valid");
    let mut whole = source.clone();
    let mut chunked = source;

    band_pass.process_buffer(&mut whole).unwrap();
    for channel_index in 0..chunked.channels().as_usize() {
        let channel = chunked
            .channel_mut(channel_index)
            .expect("channel index is within the fixture shape");
        let mut state = BiquadState::new(coefficients);
        for chunk in channel.chunks_mut(3) {
            state.process_mono_samples(chunk);
        }
    }

    assert_sample_bits_eq(chunked.as_planar_f32(), whole.as_planar_f32());
}

#[test]
fn bandpass_rejects_bad_command_shapes_and_runtime_designs() {
    let missing = parse_effect_chain(&["bandpass", "1000"]).unwrap_err();
    assert!(matches!(
        missing,
        auralis_effects::EffectChainParseError::CommandParseFailed {
            source: EffectCommandParseError::MissingArgument {
                effect: "bandpass",
                argument: "width",
            },
            ..
        }
    ));

    let invalid = parse_effect_chain(&["bandpass", "1000", "1s"]).unwrap_err();
    assert!(matches!(
        invalid,
        auralis_effects::EffectChainParseError::CommandParseFailed {
            source: EffectCommandParseError::InvalidEffectConfig {
                effect: "bandpass",
                argument: "width",
                source: EffectError::InvalidBiquadDesign,
            },
            ..
        }
    ));

    let mut audio = stereo_impulse();
    let error = BandPass::new(24_000.0, BiquadWidth::q(1.0))
        .unwrap()
        .process_buffer(&mut audio)
        .unwrap_err();
    assert_eq!(error, EffectError::InvalidBiquadDesign);
}

#[test]
fn bandpass_coefficients_can_be_processed_by_biquad_primitive() {
    let band_pass = BandPass::new(1_000.0, BiquadWidth::q(2.0)).unwrap();
    let coefficients = band_pass
        .coefficients(SampleRate::new(48_000).unwrap())
        .unwrap();
    let mut via_bandpass = [1.0, 0.0, 0.0, 0.0];
    let mut via_biquad = via_bandpass;

    Biquad::new(coefficients).process_mono_samples(&mut via_biquad);
    let mut audio = mono_audio_from_samples(&via_bandpass);
    band_pass.process_buffer(&mut audio).unwrap();
    via_bandpass.copy_from_slice(audio.as_planar_f32());

    assert_sample_bits_eq(&via_bandpass, &via_biquad);
}

fn stereo_impulse() -> AudioBuffer {
    let spec = AudioSpec::new(
        SampleRate::new(48_000).unwrap(),
        ChannelCount::new(2).unwrap(),
        SampleFormat::Float32,
    );

    AudioBuffer::from_planar_f32(
        spec,
        FrameCount::new(8),
        vec![
            0.5, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, -0.25, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0,
        ],
    )
    .unwrap()
}

fn mono_audio_from_samples(samples: &[f32]) -> AudioBuffer {
    let spec = AudioSpec::new(
        SampleRate::new(48_000).unwrap(),
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

fn assert_sample_bits_eq(actual: &[f32], expected: &[f32]) {
    assert_eq!(actual.len(), expected.len());
    for (index, (actual, expected)) in actual.iter().zip(expected).enumerate() {
        assert_eq!(
            actual.to_bits(),
            expected.to_bits(),
            "sample {index} differed: actual={actual} expected={expected}"
        );
    }
}

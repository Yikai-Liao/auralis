//! Integration coverage for SoX-ng-style dcshift limiter gain.

use auralis_core::{AudioBuffer, AudioSpec, ChannelCount, FrameCount, SampleFormat, SampleRate};
use auralis_effects::{DcShift, EffectCommand, parse_effect_chain};
use auralis_simd::BackendKind;

#[test]
fn dcshift_limiter_chain_parses_optional_second_argument() {
    let chain = parse_effect_chain(&["dcshift", "0.5", "0.05", "reverse"]).unwrap();

    assert_eq!(
        chain.commands(),
        &[
            EffectCommand::DcShift(DcShift::with_limiter_gain(0.5, 0.05).unwrap()),
            EffectCommand::Reverse(auralis_effects::Reverse::new()),
        ]
    );
    assert_eq!(chain.render_tokens(), ["dcshift", "0.5", "0.05", "reverse"]);
}

#[test]
fn dcshift_limiter_chain_matches_under_forced_scalar_and_requested_simd() {
    let chain = parse_effect_chain(&["dcshift", "0.5", "0.05"]).unwrap();
    let mut scalar = mono_buffer(vec![-1.0, -0.75, 0.0, 0.75, 1.0]);
    let mut simd = scalar.clone();

    chain
        .process_buffer_with_backend(&mut scalar, BackendKind::Scalar)
        .unwrap();
    chain
        .process_buffer_with_backend(&mut simd, BackendKind::Simd)
        .unwrap();

    assert_eq!(simd.as_planar_f32(), scalar.as_planar_f32());
    assert_eq!(scalar.as_planar_f32(), &[-0.5, -0.25, 0.5, 0.55, 0.55]);
}

fn mono_buffer(samples: Vec<f32>) -> AudioBuffer {
    AudioBuffer::from_planar_f32(
        AudioSpec::new(
            SampleRate::new(48_000).unwrap(),
            ChannelCount::new(1).unwrap(),
            SampleFormat::Float32,
        ),
        FrameCount::new(samples.len() as u64),
        samples,
    )
    .unwrap()
}

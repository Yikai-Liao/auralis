//! L4 property and metamorphic tests for implemented effects.

use auralis_core::{
    AudioBuffer, AudioSpec, ChannelCount, Decibels, FrameCount, SampleFormat, SampleRate,
};
use auralis_effects::{DcShift, Fade, Gain, Pad, Reverse, Trim};
use proptest::prelude::*;
use proptest::test_runner::TestCaseError;

#[derive(Clone, Debug)]
struct GeneratedAudio {
    channels: u16,
    frames: usize,
    samples: Vec<f32>,
}

impl GeneratedAudio {
    fn to_buffer(&self) -> AudioBuffer {
        let spec = AudioSpec::new(
            SampleRate::new(48_000).expect("sample rate fixture is valid"),
            ChannelCount::new(self.channels).expect("channel count strategy excludes zero"),
            SampleFormat::Float32,
        );
        let frames = FrameCount::new(u64::try_from(self.frames).expect("frame strategy fits u64"));

        AudioBuffer::from_planar_f32(spec, frames, self.samples.clone())
            .expect("sample strategy matches generated buffer shape")
    }
}

fn generated_audio() -> impl Strategy<Value = GeneratedAudio> {
    (1_u16..=4, 0_usize..=64).prop_flat_map(|(channels, frames)| {
        let sample_count = usize::from(channels) * frames;

        proptest::collection::vec(-0.45_f32..=0.45, sample_count).prop_map(move |samples| {
            GeneratedAudio {
                channels,
                frames,
                samples,
            }
        })
    })
}

fn prop_assert_sample_bits_eq(actual: &[f32], expected: &[f32]) -> Result<(), TestCaseError> {
    prop_assert_eq!(actual.len(), expected.len());

    for (index, (&actual, &expected)) in actual.iter().zip(expected).enumerate() {
        prop_assert_eq!(
            actual.to_bits(),
            expected.to_bits(),
            "sample {} differed: {} != {}",
            index,
            actual,
            expected
        );
    }

    Ok(())
}

fn prop_assert_samples_close(
    actual: &[f32],
    expected: &[f32],
    tolerance: f32,
) -> Result<(), TestCaseError> {
    prop_assert_eq!(actual.len(), expected.len());

    for (index, (&actual, &expected)) in actual.iter().zip(expected).enumerate() {
        let difference = (actual - expected).abs();
        prop_assert!(
            difference <= tolerance,
            "sample {} differed by {}: {} != {} within {}",
            index,
            difference,
            actual,
            expected,
            tolerance
        );
    }

    Ok(())
}

fn prop_assert_all_finite(audio: &AudioBuffer) -> Result<(), TestCaseError> {
    for (index, sample) in audio.as_planar_f32().iter().enumerate() {
        prop_assert!(
            sample.is_finite(),
            "sample {} was not finite: {}",
            index,
            sample
        );
    }

    Ok(())
}

proptest! {
    #[test]
    fn identity_parameters_preserve_samples(audio in generated_audio()) {
        let source = audio.to_buffer();

        let mut gained = source.clone();
        Gain::new(Decibels::new(0.0).expect("zero dB is valid"))
            .process_buffer(&mut gained);
        prop_assert_sample_bits_eq(gained.as_planar_f32(), source.as_planar_f32())?;

        let mut shifted = source.clone();
        DcShift::new(0.0).expect("zero shift is valid").process_buffer(&mut shifted);
        prop_assert_sample_bits_eq(shifted.as_planar_f32(), source.as_planar_f32())?;

        let mut faded = source.clone();
        Fade::new(FrameCount::new(0), FrameCount::new(0)).process_buffer(&mut faded);
        prop_assert_sample_bits_eq(faded.as_planar_f32(), source.as_planar_f32())?;

        let trimmed = Trim::new(FrameCount::new(0), source.frames())
            .expect("full-range trim is ordered")
            .process_buffer(&source)
            .expect("full-range trim is in bounds");
        prop_assert_sample_bits_eq(trimmed.as_planar_f32(), source.as_planar_f32())?;
        prop_assert_eq!(trimmed.frames(), source.frames());
        prop_assert_eq!(trimmed.channels(), source.channels());

        let padded = Pad::new(FrameCount::new(0), FrameCount::new(0))
            .process_buffer(&source)
            .expect("zero padding cannot overflow generated buffers");
        prop_assert_sample_bits_eq(padded.as_planar_f32(), source.as_planar_f32())?;
        prop_assert_eq!(padded.frames(), source.frames());
        prop_assert_eq!(padded.channels(), source.channels());
    }

    #[test]
    fn reverse_twice_returns_original_signal(audio in generated_audio()) {
        let source = audio.to_buffer();
        let mut actual = source.clone();

        Reverse::new().process_buffer(&mut actual);
        Reverse::new().process_buffer(&mut actual);

        prop_assert_sample_bits_eq(actual.as_planar_f32(), source.as_planar_f32())?;
        prop_assert_eq!(actual.frames(), source.frames());
        prop_assert_eq!(actual.channels(), source.channels());
    }

    #[test]
    fn gain_then_inverse_gain_approximately_returns_non_clipping_input(
        audio in generated_audio(),
        db in 0.0_f64..=6.0,
    ) {
        let source = audio.to_buffer();
        let mut actual = source.clone();

        Gain::new(Decibels::new(db).expect("generated dB is finite"))
            .process_buffer(&mut actual);
        Gain::new(Decibels::new(-db).expect("generated inverse dB is finite"))
            .process_buffer(&mut actual);

        prop_assert_samples_close(actual.as_planar_f32(), source.as_planar_f32(), 1.0e-6)?;
        prop_assert_eq!(actual.frames(), source.frames());
        prop_assert_eq!(actual.channels(), source.channels());
    }

    #[test]
    fn implemented_effects_preserve_finiteness_for_bounded_finite_input(
        audio in generated_audio(),
        db in -6.0_f64..=6.0,
        shift in -0.25_f32..=0.25,
    ) {
        let source = audio.to_buffer();

        let mut gained = source.clone();
        Gain::new(Decibels::new(db).expect("generated dB is finite"))
            .process_buffer(&mut gained);
        prop_assert_all_finite(&gained)?;

        let mut shifted = source.clone();
        DcShift::new(shift).expect("generated shift is valid").process_buffer(&mut shifted);
        prop_assert_all_finite(&shifted)?;

        let mut faded = source.clone();
        let fade_in = FrameCount::new(u64::try_from(audio.frames / 2).expect("frame strategy fits u64"));
        let fade_out = FrameCount::new(u64::try_from(audio.frames / 3).expect("frame strategy fits u64"));
        Fade::new(fade_in, fade_out).process_buffer(&mut faded);
        prop_assert_all_finite(&faded)?;

        let trim_start = FrameCount::new(u64::try_from(audio.frames / 3).expect("frame strategy fits u64"));
        let trim_end = FrameCount::new(u64::try_from(audio.frames - (audio.frames / 3)).expect("frame strategy fits u64"));
        let trimmed = Trim::new(trim_start, trim_end)
            .expect("generated trim range is ordered")
            .process_buffer(&source)
            .expect("generated trim range is in bounds");
        prop_assert_all_finite(&trimmed)?;

        let padded = Pad::new(FrameCount::new(1), FrameCount::new(2))
            .process_buffer(&source)
            .expect("small generated padding cannot overflow");
        prop_assert_all_finite(&padded)?;

        let mut reversed = source;
        Reverse::new().process_buffer(&mut reversed);
        prop_assert_all_finite(&reversed)?;
    }
}

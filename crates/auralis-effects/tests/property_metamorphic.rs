//! L4 property and metamorphic tests for implemented effects.

use auralis_core::{
    AudioBuffer, AudioSpec, ChannelCount, Decibels, FrameCount, SampleFormat, SampleRate,
};
use auralis_effects::{
    AllPass, Band, BandPass, BandReject, Bass, Biquad, BiquadCoefficients, BiquadWidth, Centercut,
    Channels, Contrast, DcShift, Deemph, Equalizer, Fade, Gain, HighPass, LowPass, Norm, Oops,
    Overdrive, Pad, Remix, RemixOutputSpec, RemixSource, Repeat, Reverse, Saturation,
    SaturationType, SoftVol, Swap, Treble, Tremolo, Trim, Vol,
};
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

        let mut volume_scaled = source.clone();
        Vol::amplitude(1.0).expect("unity vol gain is valid")
            .process_buffer(&mut volume_scaled);
        prop_assert_sample_bits_eq(volume_scaled.as_planar_f32(), source.as_planar_f32())?;

        let mut soft_volume_scaled = source.clone();
        SoftVol::default().process_buffer(&mut soft_volume_scaled);
        prop_assert_sample_bits_eq(
            soft_volume_scaled.as_planar_f32(),
            source.as_planar_f32(),
        )?;

        let mut unmodulated = source.clone();
        Tremolo::new(0.0, 40.0)
            .expect("zero-speed tremolo is valid")
            .process_buffer(&mut unmodulated);
        prop_assert_sample_bits_eq(unmodulated.as_planar_f32(), source.as_planar_f32())?;

        let mut biquad_filtered = source.clone();
        Biquad::default().process_buffer(&mut biquad_filtered);
        prop_assert_sample_bits_eq(biquad_filtered.as_planar_f32(), source.as_planar_f32())?;

        let mut normalized_silence = zero_audio_like(&source);
        Norm::zero_db().expect("zero dB is valid")
            .process_buffer(&mut normalized_silence)
            .expect("finite silence normalizes successfully");
        prop_assert_sample_bits_eq(
            normalized_silence.as_planar_f32(),
            zero_audio_like(&source).as_planar_f32(),
        )?;

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

        let repeated = Repeat::new(0)
            .expect("zero repeat count is valid")
            .process_buffer(&source)
            .expect("zero repeat cannot overflow generated buffers");
        prop_assert_sample_bits_eq(repeated.as_planar_f32(), source.as_planar_f32())?;
        prop_assert_eq!(repeated.frames(), source.frames());
        prop_assert_eq!(repeated.channels(), source.channels());

        let channel_converted = Channels::new(source.channels())
            .process_buffer(&source)
            .expect("matching channel conversion cannot fail");
        prop_assert_sample_bits_eq(channel_converted.as_planar_f32(), source.as_planar_f32())?;
        prop_assert_eq!(channel_converted.frames(), source.frames());
        prop_assert_eq!(channel_converted.channels(), source.channels());

        let remixed = identity_remix(source.channels().as_u16())
            .process_buffer(&source)
            .expect("identity remix cannot fail");
        prop_assert_sample_bits_eq(remixed.as_planar_f32(), source.as_planar_f32())?;
        prop_assert_eq!(remixed.frames(), source.frames());
        prop_assert_eq!(remixed.channels(), source.channels());
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
    fn swap_twice_returns_original_signal(audio in generated_audio()) {
        let source = audio.to_buffer();
        let mut actual = source.clone();

        Swap::new().process_buffer(&mut actual);
        Swap::new().process_buffer(&mut actual);

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

        let mut volume_scaled = source.clone();
        Vol::amplitude(0.5).expect("generated vol gain is valid")
            .process_buffer(&mut volume_scaled);
        prop_assert_all_finite(&volume_scaled)?;

        let mut all_passed = source.clone();
        AllPass::new(1_000.0, BiquadWidth::q(0.707))
            .expect("fixture all-pass design is valid")
            .process_buffer(&mut all_passed)
            .expect("fixture sample rate keeps frequency below Nyquist");
        prop_assert_all_finite(&all_passed)?;

        let mut band_passed = source.clone();
        Band::new(1_000.0, Some(BiquadWidth::q(2.0)))
            .expect("fixture band design is valid")
            .process_buffer(&mut band_passed)
            .expect("fixture sample rate keeps frequency below Nyquist");
        prop_assert_all_finite(&band_passed)?;

        let mut rbj_band_passed = source.clone();
        BandPass::new(1_000.0, BiquadWidth::q(2.0))
            .expect("fixture bandpass design is valid")
            .process_buffer(&mut rbj_band_passed)
            .expect("fixture sample rate keeps frequency below Nyquist");
        prop_assert_all_finite(&rbj_band_passed)?;

        let mut band_rejected = source.clone();
        BandReject::new(1_000.0, BiquadWidth::q(2.0))
            .expect("fixture bandreject design is valid")
            .process_buffer(&mut band_rejected)
            .expect("fixture sample rate keeps frequency below Nyquist");
        prop_assert_all_finite(&band_rejected)?;

        let mut bass_filtered = source.clone();
        Bass::with_width(6.0, 100.0, BiquadWidth::slope(0.5))
            .expect("fixture bass design is valid")
            .process_buffer(&mut bass_filtered)
            .expect("fixture sample rate keeps frequency below Nyquist");
        prop_assert_all_finite(&bass_filtered)?;

        let mut treble_filtered = source.clone();
        Treble::with_width(-6.0, 3000.0, BiquadWidth::slope(0.5))
            .expect("fixture treble design is valid")
            .process_buffer(&mut treble_filtered)
            .expect("fixture sample rate keeps frequency below Nyquist");
        prop_assert_all_finite(&treble_filtered)?;

        let mut equalized = source.clone();
        Equalizer::new(1_000.0, BiquadWidth::q(1.0), 6.0)
            .expect("fixture equalizer design is valid")
            .process_buffer(&mut equalized)
            .expect("fixture sample rate keeps frequency below Nyquist");
        prop_assert_all_finite(&equalized)?;

        let mut low_passed = source.clone();
        LowPass::with_width(1_000.0, BiquadWidth::q(0.707))
            .expect("fixture lowpass design is valid")
            .process_buffer(&mut low_passed)
            .expect("fixture sample rate keeps frequency below Nyquist");
        prop_assert_all_finite(&low_passed)?;

        let mut high_passed = source.clone();
        HighPass::with_width(1_000.0, BiquadWidth::q(0.707))
            .expect("fixture highpass design is valid")
            .process_buffer(&mut high_passed)
            .expect("fixture sample rate keeps frequency below Nyquist");
        prop_assert_all_finite(&high_passed)?;

        let mut deemphasized = source.clone();
        Deemph::new()
            .process_buffer(&mut deemphasized)
            .expect("fixture sample rate has a SoX-ng deemph preset");
        prop_assert_all_finite(&deemphasized)?;

        let mut normalized = source.clone();
        Norm::new(Decibels::new(db).expect("generated dB is finite"))
            .process_buffer(&mut normalized)
            .expect("generated samples are finite");
        prop_assert_all_finite(&normalized)?;

        let mut shifted = source.clone();
        DcShift::new(shift).expect("generated shift is valid").process_buffer(&mut shifted);
        prop_assert_all_finite(&shifted)?;

        let mut contrasted = source.clone();
        Contrast::new(75.0).expect("default contrast amount is valid")
            .process_buffer(&mut contrasted);
        prop_assert_all_finite(&contrasted)?;

        let mut soft_volume_scaled = source.clone();
        SoftVol::new(2.0, 1.0, 0.1)
            .expect("generated softvol settings are valid")
            .process_buffer(&mut soft_volume_scaled);
        prop_assert_all_finite(&soft_volume_scaled)?;

        let mut tremolo_modulated = source.clone();
        Tremolo::new(5.0, 75.0)
            .expect("generated tremolo settings are valid")
            .process_buffer(&mut tremolo_modulated);
        prop_assert_all_finite(&tremolo_modulated)?;

        let mut overdriven = source.clone();
        Overdrive::new(12.0, 25.0)
            .expect("generated overdrive settings are valid")
            .process_buffer(&mut overdriven);
        prop_assert_all_finite(&overdriven)?;

        let mut saturated = source.clone();
        Saturation::new(SaturationType::Sqrt, 0.75, 0.1, 0.25)
            .expect("generated saturation settings are valid")
            .process_buffer(&mut saturated);
        prop_assert_all_finite(&saturated)?;

        let mut biquad_filtered = source.clone();
        Biquad::new(
            BiquadCoefficients::normalized(0.5, 0.25, 0.125, -0.25, 0.0625)
                .expect("fixture coefficients are finite"),
        )
        .process_buffer(&mut biquad_filtered);
        prop_assert_all_finite(&biquad_filtered)?;

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

        let repeated = Repeat::new(2)
            .expect("small repeat count is valid")
            .process_buffer(&source)
            .expect("small generated repeat cannot overflow");
        prop_assert_all_finite(&repeated)?;

        let converted = Channels::new(ChannelCount::new(2).expect("fixture channel count is valid"))
            .process_buffer(&source)
            .expect("small generated channel conversion succeeds");
        prop_assert_all_finite(&converted)?;

        let remixed = Remix::new([
            RemixOutputSpec::new([RemixSource::channel(1).expect("channel one is valid")])
                .expect("single-source remix output is valid"),
            RemixOutputSpec::silent(),
        ])
        .expect("two-output remix is valid")
        .process_buffer(&source)
        .expect("generated audio always has channel one");
        prop_assert_all_finite(&remixed)?;

        if source.channels().as_usize() >= 2 {
            let out_of_phase = Oops::new()
                .process_buffer(&source)
                .expect("generated multichannel audio has channel two");
            prop_assert_all_finite(&out_of_phase)?;
            prop_assert_eq!(out_of_phase.channels(), ChannelCount::new(2).unwrap());
            prop_assert_eq!(out_of_phase.frames(), source.frames());
        }

        if source.channels().as_usize() == 2 {
            let centered = Centercut::new()
                .process_buffer(&source)
                .expect("generated stereo audio is valid for centercut");
            prop_assert_all_finite(&centered)?;
            prop_assert_eq!(centered.channels(), ChannelCount::new(3).unwrap());
            prop_assert_eq!(centered.frames(), source.frames());
        }

        let mut reversed = source;
        Reverse::new().process_buffer(&mut reversed);
        prop_assert_all_finite(&reversed)?;

        let mut swapped = reversed;
        Swap::new().process_buffer(&mut swapped);
        prop_assert_all_finite(&swapped)?;
    }
}

fn identity_remix(channels: u16) -> Remix {
    let outputs = (1..=channels)
        .map(|channel| {
            RemixOutputSpec::new([RemixSource::channel(channel).expect("channel is nonzero")])
                .expect("single-source output is valid")
        })
        .collect::<Vec<_>>();

    Remix::new(outputs).expect("generated audio has at least one channel")
}

fn zero_audio_like(source: &AudioBuffer) -> AudioBuffer {
    AudioBuffer::from_planar_f32(
        source.spec(),
        source.frames(),
        vec![0.0; source.as_planar_f32().len()],
    )
    .expect("zero samples match source shape")
}

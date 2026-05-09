//! L5 chunk-invariance matrix for streaming-capable effects.

use auralis_core::{
    AudioBuffer, AudioSpec, ChannelCount, Decibels, FrameCount, SampleFormat, SampleRate,
};
use auralis_effects::{
    AllPass, Band, BandPass, BandReject, Biquad, BiquadCoefficients, BiquadState, BiquadWidth,
    Contrast, DcShift, EffectChain, EffectCommand, Fade, Gain, Saturation, SaturationType, Tremolo,
    Vol,
};
use auralis_testkit::chunk_invariance::{ChunkSchedule, l5_chunk_schedules, process_chunks_mut};

#[test]
fn gain_matches_whole_buffer_for_l5_chunk_matrix() {
    let gain = Gain::new(Decibels::new(-3.0).expect("fixture dB is valid"));
    let source = stereo_source(1_105);
    let schedules = l5_chunk_schedules(source.as_planar_f32().len());

    for schedule in &schedules {
        let mut whole = source.clone();
        let mut chunked = source.clone();

        gain.process_buffer(&mut whole);
        process_chunks_mut(chunked.as_planar_f32_mut(), schedule, |chunk, _offset| {
            gain.process_samples(chunk);
        });

        assert_same_audio(&chunked, &whole, schedule);
    }
}

#[test]
fn dcshift_matches_whole_buffer_for_l5_chunk_matrix() {
    let dc_shift = DcShift::new(0.125).expect("fixture shift is valid");
    let source = stereo_source(1_105);
    let schedules = l5_chunk_schedules(source.as_planar_f32().len());

    for schedule in &schedules {
        let mut whole = source.clone();
        let mut chunked = source.clone();

        dc_shift.process_buffer(&mut whole);
        process_chunks_mut(chunked.as_planar_f32_mut(), schedule, |chunk, _offset| {
            dc_shift.process_samples(chunk);
        });

        assert_same_audio(&chunked, &whole, schedule);
    }
}

#[test]
fn vol_matches_whole_buffer_for_l5_chunk_matrix() {
    let vol = Vol::amplitude(0.5).expect("fixture vol gain is valid");
    let source = stereo_source(1_105);
    let schedules = l5_chunk_schedules(source.as_planar_f32().len());

    for schedule in &schedules {
        let mut whole = source.clone();
        let mut chunked = source.clone();

        vol.process_buffer(&mut whole);
        process_chunks_mut(chunked.as_planar_f32_mut(), schedule, |chunk, _offset| {
            vol.process_samples(chunk);
        });

        assert_same_audio(&chunked, &whole, schedule);
    }
}

#[test]
fn contrast_matches_whole_buffer_for_l5_chunk_matrix() {
    let contrast = Contrast::new(75.0).expect("fixture contrast amount is valid");
    let source = stereo_source(1_105);
    let schedules = l5_chunk_schedules(source.as_planar_f32().len());

    for schedule in &schedules {
        let mut whole = source.clone();
        let mut chunked = source.clone();

        contrast.process_buffer(&mut whole);
        process_chunks_mut(chunked.as_planar_f32_mut(), schedule, |chunk, _offset| {
            contrast.process_samples(chunk);
        });

        assert_same_audio(&chunked, &whole, schedule);
    }
}

#[test]
fn saturation_matches_whole_buffer_for_l5_chunk_matrix() {
    let saturation = Saturation::new(SaturationType::Sqrt, 0.75, 0.1, 0.25)
        .expect("fixture saturation settings are valid");
    let source = stereo_source(1_105);
    let schedules = l5_chunk_schedules(source.as_planar_f32().len());

    for schedule in &schedules {
        let mut whole = source.clone();
        let mut chunked = source.clone();

        saturation.process_buffer(&mut whole);
        process_chunks_mut(chunked.as_planar_f32_mut(), schedule, |chunk, _offset| {
            saturation.process_samples(chunk);
        });

        assert_same_audio(&chunked, &whole, schedule);
    }
}

#[test]
fn tremolo_matches_whole_buffer_for_l5_chunk_matrix() {
    let tremolo = Tremolo::new(7.0, 75.0).expect("fixture tremolo settings are valid");
    let source = stereo_source(1_105);
    let schedules = l5_chunk_schedules(frames_len(&source));

    for schedule in &schedules {
        let mut whole = source.clone();
        let mut chunked = source.clone();

        tremolo.process_buffer(&mut whole);
        process_tremolo_by_channel_chunks(&mut chunked, tremolo, schedule);

        assert_same_audio(&chunked, &whole, schedule);
    }
}

#[test]
fn biquad_matches_whole_buffer_for_l5_chunk_matrix_when_state_is_preserved() {
    let coefficients = BiquadCoefficients::normalized(0.5, 0.25, 0.125, -0.25, 0.0625)
        .expect("fixture coefficients are finite");
    let biquad = Biquad::new(coefficients);
    let source = stereo_source(1_105);
    let schedules = l5_chunk_schedules(frames_len(&source));

    for schedule in &schedules {
        let mut whole = source.clone();
        let mut chunked = source.clone();

        biquad.process_buffer(&mut whole);
        process_biquad_by_channel_chunks(&mut chunked, coefficients, schedule);

        assert_same_audio(&chunked, &whole, schedule);
    }
}

#[test]
fn allpass_matches_whole_buffer_for_l5_chunk_matrix_when_state_is_preserved() {
    let all_pass =
        AllPass::new(1_000.0, BiquadWidth::q(0.707)).expect("fixture all-pass design is valid");
    let source = stereo_source(1_105);
    let schedules = l5_chunk_schedules(frames_len(&source));

    for schedule in &schedules {
        let mut whole = source.clone();
        let mut chunked = source.clone();

        all_pass
            .process_buffer(&mut whole)
            .expect("fixture sample rate keeps frequency below Nyquist");
        process_allpass_by_channel_chunks(&mut chunked, all_pass, schedule);

        assert_same_audio(&chunked, &whole, schedule);
    }
}

#[test]
fn band_matches_whole_buffer_for_l5_chunk_matrix_when_state_is_preserved() {
    let band =
        Band::unpitched(1_000.0, Some(BiquadWidth::q(2.0))).expect("fixture band design is valid");
    let source = stereo_source(1_105);
    let schedules = l5_chunk_schedules(frames_len(&source));

    for schedule in &schedules {
        let mut whole = source.clone();
        let mut chunked = source.clone();

        band.process_buffer(&mut whole)
            .expect("fixture sample rate keeps frequency below Nyquist");
        process_band_by_channel_chunks(&mut chunked, band, schedule);

        assert_same_audio(&chunked, &whole, schedule);
    }
}

#[test]
fn bandpass_matches_whole_buffer_for_l5_chunk_matrix_when_state_is_preserved() {
    let band_pass =
        BandPass::constant_skirt(1_000.0, BiquadWidth::q(2.0)).expect("fixture design is valid");
    let source = stereo_source(1_105);
    let schedules = l5_chunk_schedules(frames_len(&source));

    for schedule in &schedules {
        let mut whole = source.clone();
        let mut chunked = source.clone();

        band_pass
            .process_buffer(&mut whole)
            .expect("fixture sample rate keeps frequency below Nyquist");
        process_bandpass_by_channel_chunks(&mut chunked, band_pass, schedule);

        assert_same_audio(&chunked, &whole, schedule);
    }
}

#[test]
fn bandreject_matches_whole_buffer_for_l5_chunk_matrix_when_state_is_preserved() {
    let band_reject =
        BandReject::new(1_000.0, BiquadWidth::q(2.0)).expect("fixture design is valid");
    let source = stereo_source(1_105);
    let schedules = l5_chunk_schedules(frames_len(&source));

    for schedule in &schedules {
        let mut whole = source.clone();
        let mut chunked = source.clone();

        band_reject
            .process_buffer(&mut whole)
            .expect("fixture sample rate keeps frequency below Nyquist");
        process_bandreject_by_channel_chunks(&mut chunked, band_reject, schedule);

        assert_same_audio(&chunked, &whole, schedule);
    }
}

#[test]
fn fade_matches_whole_buffer_for_l5_chunk_matrix() {
    let fade = Fade::new(FrameCount::new(257), FrameCount::new(383));
    let source = stereo_source(1_105);
    let schedules = l5_chunk_schedules(frames_len(&source));

    for schedule in &schedules {
        let mut whole = source.clone();
        let mut chunked = source.clone();

        fade.process_buffer(&mut whole);
        process_fade_by_channel_chunks(&mut chunked, fade, schedule);

        assert_same_audio(&chunked, &whole, schedule);
    }
}

#[test]
fn streaming_safe_chain_matches_whole_buffer_for_l5_chunk_matrix() {
    let chain = EffectChain::new(vec![
        EffectCommand::Biquad(Biquad::new(
            BiquadCoefficients::normalized(0.5, 0.25, 0.125, -0.25, 0.0625)
                .expect("fixture coefficients are finite"),
        )),
        EffectCommand::Gain(Gain::new(Decibels::new(-6.0).expect("fixture dB is valid"))),
        EffectCommand::DcShift(DcShift::new(0.125).expect("fixture shift is valid")),
        EffectCommand::Fade(Fade::new(FrameCount::new(257), FrameCount::new(383))),
        EffectCommand::Tremolo(
            Tremolo::new(7.0, 75.0).expect("fixture tremolo settings are valid"),
        ),
        EffectCommand::Saturation(
            Saturation::new(SaturationType::Sqrt, 0.75, 0.1, 0.25)
                .expect("fixture saturation settings are valid"),
        ),
    ]);
    let source = stereo_source(1_105);
    let schedules = l5_chunk_schedules(frames_len(&source));

    for schedule in &schedules {
        let mut whole = source.clone();
        let mut chunked = source.clone();

        chain
            .process_buffer(&mut whole)
            .expect("streaming-safe fixture chain succeeds");
        process_streaming_safe_chain_by_chunks(&chain, &mut chunked, schedule);

        assert_same_audio(&chunked, &whole, schedule);
    }
}

fn process_streaming_safe_chain_by_chunks(
    chain: &EffectChain,
    audio: &mut AudioBuffer,
    schedule: &ChunkSchedule,
) {
    for command in chain.commands() {
        match command {
            EffectCommand::AllPass(all_pass) => {
                process_allpass_by_channel_chunks(audio, *all_pass, schedule);
            }
            EffectCommand::Band(band) => {
                process_band_by_channel_chunks(audio, *band, schedule);
            }
            EffectCommand::BandPass(band_pass) => {
                process_bandpass_by_channel_chunks(audio, *band_pass, schedule);
            }
            EffectCommand::BandReject(band_reject) => {
                process_bandreject_by_channel_chunks(audio, *band_reject, schedule);
            }
            EffectCommand::Biquad(biquad) => {
                process_biquad_by_channel_chunks(audio, biquad.coefficients(), schedule);
            }
            EffectCommand::Gain(gain) => {
                process_chunks_mut(audio.as_planar_f32_mut(), schedule, |chunk, _offset| {
                    gain.process_samples(chunk);
                });
            }
            EffectCommand::DcShift(dc_shift) => {
                process_chunks_mut(audio.as_planar_f32_mut(), schedule, |chunk, _offset| {
                    dc_shift.process_samples(chunk);
                });
            }
            EffectCommand::Fade(fade) => {
                process_fade_by_channel_chunks(audio, *fade, schedule);
            }
            EffectCommand::Vol(vol) => {
                process_chunks_mut(audio.as_planar_f32_mut(), schedule, |chunk, _offset| {
                    vol.process_samples(chunk);
                });
            }
            EffectCommand::Contrast(contrast) => {
                process_chunks_mut(audio.as_planar_f32_mut(), schedule, |chunk, _offset| {
                    contrast.process_samples(chunk);
                });
            }
            EffectCommand::Saturation(saturation) => {
                process_chunks_mut(audio.as_planar_f32_mut(), schedule, |chunk, _offset| {
                    saturation.process_samples(chunk);
                });
            }
            EffectCommand::Tremolo(tremolo) => {
                process_tremolo_by_channel_chunks(audio, *tremolo, schedule);
            }
            EffectCommand::Centercut(_)
            | EffectCommand::Channels(_)
            | EffectCommand::Norm(_)
            | EffectCommand::Oops(_)
            | EffectCommand::Pad(_)
            | EffectCommand::Repeat(_)
            | EffectCommand::Remix(_)
            | EffectCommand::Reverse(_)
            | EffectCommand::SoftVol(_)
            | EffectCommand::Trim(_) => {
                panic!("L5 streaming-safe chain fixture contained a non-streaming command")
            }
            _ => panic!("L5 streaming-safe chain fixture contained an unknown command"),
        }
    }
}

fn process_allpass_by_channel_chunks(
    audio: &mut AudioBuffer,
    all_pass: AllPass,
    schedule: &ChunkSchedule,
) {
    let coefficients = all_pass
        .coefficients(audio.spec().sample_rate())
        .expect("fixture sample rate keeps frequency below Nyquist");
    process_biquad_by_channel_chunks(audio, coefficients, schedule);
}

fn process_band_by_channel_chunks(audio: &mut AudioBuffer, band: Band, schedule: &ChunkSchedule) {
    let coefficients = band
        .coefficients(audio.spec().sample_rate())
        .expect("fixture sample rate keeps frequency below Nyquist");
    process_biquad_by_channel_chunks(audio, coefficients, schedule);
}

fn process_bandpass_by_channel_chunks(
    audio: &mut AudioBuffer,
    band_pass: BandPass,
    schedule: &ChunkSchedule,
) {
    let coefficients = band_pass
        .coefficients(audio.spec().sample_rate())
        .expect("fixture sample rate keeps frequency below Nyquist");
    process_biquad_by_channel_chunks(audio, coefficients, schedule);
}

fn process_bandreject_by_channel_chunks(
    audio: &mut AudioBuffer,
    band_reject: BandReject,
    schedule: &ChunkSchedule,
) {
    let coefficients = band_reject
        .coefficients(audio.spec().sample_rate())
        .expect("fixture sample rate keeps frequency below Nyquist");
    process_biquad_by_channel_chunks(audio, coefficients, schedule);
}

fn process_biquad_by_channel_chunks(
    audio: &mut AudioBuffer,
    coefficients: BiquadCoefficients,
    schedule: &ChunkSchedule,
) {
    for channel_index in 0..audio.channels().as_usize() {
        let channel = audio
            .channel_mut(channel_index)
            .expect("channel index is within the audio shape");
        let mut state = BiquadState::new(coefficients);
        process_chunks_mut(channel, schedule, |chunk, _start_frame| {
            state.process_mono_samples(chunk);
        });
    }
}

fn process_fade_by_channel_chunks(audio: &mut AudioBuffer, fade: Fade, schedule: &ChunkSchedule) {
    let total_frames = audio.frames().as_u64();
    for channel_index in 0..audio.channels().as_usize() {
        let channel = audio
            .channel_mut(channel_index)
            .expect("channel index is within the audio shape");
        process_chunks_mut(channel, schedule, |chunk, start_frame| {
            fade.process_channel_segment(
                chunk,
                total_frames,
                FrameCount::new(u64::try_from(start_frame).expect("fixture offset fits u64")),
            );
        });
    }
}

fn process_tremolo_by_channel_chunks(
    audio: &mut AudioBuffer,
    tremolo: Tremolo,
    schedule: &ChunkSchedule,
) {
    let sample_rate = audio.spec().sample_rate();
    for channel_index in 0..audio.channels().as_usize() {
        let channel = audio
            .channel_mut(channel_index)
            .expect("channel index is within the audio shape");
        process_chunks_mut(channel, schedule, |chunk, start_frame| {
            tremolo.process_mono_samples(
                chunk,
                sample_rate,
                FrameCount::new(u64::try_from(start_frame).expect("fixture offset fits u64")),
            );
        });
    }
}

fn stereo_source(frames: usize) -> AudioBuffer {
    let mut samples = Vec::with_capacity(frames * 2);
    for frame in 0..frames {
        let phase = f32::from(u16::try_from(frame % 97).expect("phase index fits u16")) / 96.0;
        samples.push((phase * 0.8) - 0.4);
    }
    for frame in 0..frames {
        let phase = f32::from(u16::try_from(frame % 131).expect("phase index fits u16")) / 130.0;
        samples.push(0.35 - (phase * 0.7));
    }

    let spec = AudioSpec::new(
        SampleRate::new(48_000).expect("fixture sample rate is valid"),
        ChannelCount::new(2).expect("fixture channel count is valid"),
        SampleFormat::Float32,
    );
    AudioBuffer::from_planar_f32(
        spec,
        FrameCount::new(u64::try_from(frames).expect("fixture length fits u64")),
        samples,
    )
    .expect("fixture samples match the declared shape")
}

fn frames_len(audio: &AudioBuffer) -> usize {
    usize::try_from(audio.frames().as_u64()).expect("fixture frame count fits usize")
}

fn assert_same_audio(actual: &AudioBuffer, expected: &AudioBuffer, schedule: &ChunkSchedule) {
    assert_eq!(actual.spec(), expected.spec(), "schedule {schedule}");
    assert_eq!(actual.frames(), expected.frames(), "schedule {schedule}");
    assert_sample_bits_eq(
        actual.as_planar_f32(),
        expected.as_planar_f32(),
        &schedule.to_string(),
    );
}

fn assert_sample_bits_eq(actual: &[f32], expected: &[f32], context: &str) {
    assert_eq!(actual.len(), expected.len(), "{context}");
    for (index, (&actual, &expected)) in actual.iter().zip(expected).enumerate() {
        assert_eq!(
            actual.to_bits(),
            expected.to_bits(),
            "{context}: sample {index} differed: {actual} != {expected}"
        );
    }
}

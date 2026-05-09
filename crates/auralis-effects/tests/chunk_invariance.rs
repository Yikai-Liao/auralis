//! L5 chunk-invariance matrix for streaming-capable effects.

use auralis_core::{
    AudioBuffer, AudioSpec, ChannelCount, Decibels, FrameCount, SampleFormat, SampleRate,
};
use auralis_effects::{Contrast, DcShift, EffectChain, EffectCommand, Fade, Gain, Vol};
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
        EffectCommand::Gain(Gain::new(Decibels::new(-6.0).expect("fixture dB is valid"))),
        EffectCommand::DcShift(DcShift::new(0.125).expect("fixture shift is valid")),
        EffectCommand::Fade(Fade::new(FrameCount::new(257), FrameCount::new(383))),
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
            EffectCommand::Norm(_)
            | EffectCommand::Pad(_)
            | EffectCommand::Reverse(_)
            | EffectCommand::Trim(_) => {
                panic!("L5 streaming-safe chain fixture contained a non-streaming command")
            }
            _ => panic!("L5 streaming-safe chain fixture contained an unknown command"),
        }
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

//! Integration coverage for SoX-ng-style out-of-phase stereo extraction.

use auralis_core::{AudioSpec, ChannelCount, FrameCount, SampleFormat, SampleRate};
use auralis_effects::{
    EffectChainParseError, EffectCommand, EffectCommandParseError, Oops, parse_effect_chain,
};

#[test]
fn oops_chain_parses_and_renders_without_arguments() {
    let chain = parse_effect_chain(&["oops", "reverse"]).unwrap();

    assert_eq!(
        chain.commands(),
        &[
            EffectCommand::Oops(Oops::new()),
            EffectCommand::Reverse(auralis_effects::Reverse::new()),
        ]
    );
    assert_eq!(chain.render_tokens(), ["oops", "reverse"]);
}

#[test]
fn oops_extracts_out_of_phase_stereo_in_chain() {
    let mut audio = audio_buffer(2, vec![0.25, 0.75, -0.25, 0.5]);

    parse_effect_chain(&["oops"])
        .unwrap()
        .process_buffer(&mut audio)
        .unwrap();

    assert_eq!(audio.channels(), ChannelCount::new(2).unwrap());
    assert_eq!(audio.frames(), FrameCount::new(2));
    assert_eq!(audio.as_planar_f32(), &[0.5, 0.25, 0.5, 0.25]);
}

#[test]
fn oops_clips_difference_and_ignores_extra_channels() {
    let mut audio = audio_buffer(3, vec![0.75, -0.75, -0.75, 0.75, 0.5, 0.5]);

    parse_effect_chain(&["oops"])
        .unwrap()
        .process_buffer(&mut audio)
        .unwrap();

    assert_eq!(audio.channels(), ChannelCount::new(2).unwrap());
    assert_eq!(audio.as_planar_f32(), &[1.0, -1.0, 1.0, -1.0]);
}

#[test]
fn oops_requires_at_least_two_input_channels() {
    let mut audio = audio_buffer(1, vec![0.25, -0.5]);
    let error = parse_effect_chain(&["oops"])
        .unwrap()
        .process_buffer(&mut audio)
        .unwrap_err();

    assert!(matches!(
        error,
        auralis_effects::EffectChainError::CommandFailed {
            command: EffectCommand::Oops(_),
            argument: "channels",
            ..
        }
    ));
}

#[test]
fn oops_matches_whole_buffer_when_processed_in_frame_chunks() {
    let source = audio_buffer(
        3,
        vec![
            0.25, 0.5, 0.75, -0.25, 0.0, 0.25, 0.5, 0.25, 0.0, -0.5, -0.25, 0.0,
        ],
    );
    let whole = Oops::new().process_buffer(&source).unwrap();

    let chunked = process_oops_in_chunks(&source, &[1, 2, 1]);

    assert_eq!(chunked, whole);
}

#[test]
fn oops_rejects_arguments_and_options() {
    let extra = parse_effect_chain(&["oops", "1"]).unwrap_err();
    assert!(matches!(
        extra,
        EffectChainParseError::CommandParseFailed {
            source: EffectCommandParseError::UnexpectedArgument {
                effect: "oops",
                argument,
            },
            ..
        } if argument == "1"
    ));

    let option = parse_effect_chain(&["oops", "-x"]).unwrap_err();
    assert!(matches!(
        option,
        EffectChainParseError::CommandParseFailed {
            source: EffectCommandParseError::UnsupportedOption {
                effect: "oops",
                option,
            },
            ..
        } if option == "-x"
    ));
}

fn process_oops_in_chunks(
    source: &auralis_core::AudioBuffer,
    chunks: &[usize],
) -> auralis_core::AudioBuffer {
    let input_channels = source.channels().as_usize();
    let frames = usize::try_from(source.frames().as_u64()).unwrap();
    let mut output_channels = [Vec::<f32>::new(), Vec::<f32>::new()];
    let mut frame_start = 0_usize;

    for chunk_size in chunks.iter().copied() {
        let frame_end = (frame_start + chunk_size).min(frames);
        if frame_start == frame_end {
            continue;
        }

        let mut samples = Vec::with_capacity(input_channels * (frame_end - frame_start));
        for channel_index in 0..input_channels {
            let channel = source.channel(channel_index).unwrap();
            samples.extend_from_slice(&channel[frame_start..frame_end]);
        }

        let chunk = audio_buffer(u16::try_from(input_channels).unwrap(), samples);
        let output = Oops::new().process_buffer(&chunk).unwrap();
        for (channel_index, output_channel) in output_channels.iter_mut().enumerate() {
            output_channel.extend_from_slice(output.channel(channel_index).unwrap());
        }

        frame_start = frame_end;
    }

    assert_eq!(frame_start, frames);
    let samples = output_channels.into_iter().flatten().collect();
    audio_buffer(2, samples)
}

fn audio_buffer(channels: u16, samples: Vec<f32>) -> auralis_core::AudioBuffer {
    assert_eq!(samples.len() % usize::from(channels), 0);
    let channels = ChannelCount::new(channels).unwrap();
    let frames = samples.len() / channels.as_usize();
    let spec = AudioSpec::new(
        SampleRate::new(48_000).unwrap(),
        channels,
        SampleFormat::Float32,
    );
    auralis_core::AudioBuffer::from_planar_f32(
        spec,
        FrameCount::new(u64::try_from(frames).unwrap()),
        samples,
    )
    .unwrap()
}

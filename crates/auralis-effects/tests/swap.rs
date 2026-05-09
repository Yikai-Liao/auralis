//! Integration coverage for SoX-ng-style channel pair swapping.

use auralis_core::{AudioSpec, ChannelCount, FrameCount, SampleFormat, SampleRate};
use auralis_effects::{
    EffectChainParseError, EffectCommand, EffectCommandParseError, Swap, parse_effect_chain,
};

#[test]
fn swap_chain_parses_and_renders_without_arguments() {
    let chain = parse_effect_chain(&["swap", "reverse"]).unwrap();

    assert_eq!(
        chain.commands(),
        &[
            EffectCommand::Swap(Swap::new()),
            EffectCommand::Reverse(auralis_effects::Reverse::new()),
        ]
    );
    assert_eq!(chain.render_tokens(), ["swap", "reverse"]);
}

#[test]
fn swap_exchanges_adjacent_channel_pairs_in_chain() {
    let mut audio = audio_buffer(4, vec![0.1, 0.2, -0.1, -0.2, 0.3, 0.4, -0.3, -0.4]);

    parse_effect_chain(&["swap"])
        .unwrap()
        .process_buffer(&mut audio)
        .unwrap();

    assert_eq!(audio.channels(), ChannelCount::new(4).unwrap());
    assert_eq!(audio.frames(), FrameCount::new(2));
    assert_eq!(
        audio.as_planar_f32(),
        &[-0.1, -0.2, 0.1, 0.2, -0.3, -0.4, 0.3, 0.4]
    );
}

#[test]
fn swap_preserves_mono_and_odd_trailing_channels() {
    let mut mono = audio_buffer(1, vec![0.25, -0.5]);
    parse_effect_chain(&["swap"])
        .unwrap()
        .process_buffer(&mut mono)
        .unwrap();
    assert_eq!(mono.as_planar_f32(), &[0.25, -0.5]);

    let mut odd = audio_buffer(3, vec![1.0, 1.1, 2.0, 2.1, 3.0, 3.1]);
    parse_effect_chain(&["swap"])
        .unwrap()
        .process_buffer(&mut odd)
        .unwrap();
    assert_eq!(odd.as_planar_f32(), &[2.0, 2.1, 1.0, 1.1, 3.0, 3.1]);
}

#[test]
fn swap_matches_whole_buffer_when_processed_in_frame_chunks() {
    let source = audio_buffer(
        4,
        vec![
            0.1, 0.2, 0.3, 0.4, -0.1, -0.2, -0.3, -0.4, 0.5, 0.6, 0.7, 0.8, -0.5, -0.6, -0.7, -0.8,
        ],
    );
    let mut whole = source.clone();
    Swap::new().process_buffer(&mut whole);

    let chunked = process_swap_in_chunks(&source, &[1, 2, 1]);

    assert_eq!(chunked, whole);
}

#[test]
fn swap_rejects_arguments_and_options() {
    let extra = parse_effect_chain(&["swap", "1"]).unwrap_err();
    assert!(matches!(
        extra,
        EffectChainParseError::CommandParseFailed {
            source: EffectCommandParseError::UnexpectedArgument {
                effect: "swap",
                argument,
            },
            ..
        } if argument == "1"
    ));

    let option = parse_effect_chain(&["swap", "-x"]).unwrap_err();
    assert!(matches!(
        option,
        EffectChainParseError::CommandParseFailed {
            source: EffectCommandParseError::UnsupportedOption {
                effect: "swap",
                option,
            },
            ..
        } if option == "-x"
    ));
}

fn process_swap_in_chunks(
    source: &auralis_core::AudioBuffer,
    chunks: &[usize],
) -> auralis_core::AudioBuffer {
    let channels = source.channels().as_usize();
    let frames = usize::try_from(source.frames().as_u64()).unwrap();
    let mut output_channels = vec![Vec::<f32>::new(); channels];
    let mut frame_start = 0_usize;

    for chunk_size in chunks.iter().copied() {
        let frame_end = (frame_start + chunk_size).min(frames);
        if frame_start == frame_end {
            continue;
        }

        let mut samples = Vec::with_capacity(channels * (frame_end - frame_start));
        for channel_index in 0..channels {
            let channel = source.channel(channel_index).unwrap();
            samples.extend_from_slice(&channel[frame_start..frame_end]);
        }

        let mut chunk = audio_buffer(u16::try_from(channels).unwrap(), samples);
        Swap::new().process_buffer(&mut chunk);
        for (channel_index, output) in output_channels.iter_mut().enumerate() {
            output.extend_from_slice(chunk.channel(channel_index).unwrap());
        }

        frame_start = frame_end;
    }

    assert_eq!(frame_start, frames);
    let samples = output_channels.into_iter().flatten().collect();
    audio_buffer(u16::try_from(channels).unwrap(), samples)
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

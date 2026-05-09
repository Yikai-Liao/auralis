use auralis_core::{
    AudioBuffer, AudioSpec, ChannelCount, Decibels, FrameCount, SampleFormat, SampleRate,
};

pub(crate) fn audio_buffer(samples: Vec<f32>) -> AudioBuffer {
    let frames = samples.len().try_into().unwrap();
    let spec = AudioSpec::new(
        SampleRate::new(48_000).unwrap(),
        ChannelCount::new(1).unwrap(),
        SampleFormat::Float32,
    );

    AudioBuffer::from_planar_f32(spec, FrameCount::new(frames), samples).unwrap()
}

pub(crate) fn stereo_audio_buffer(samples: Vec<f32>) -> AudioBuffer {
    assert_eq!(samples.len() % 2, 0);
    let frames = u64::try_from(samples.len() / 2).unwrap();
    let spec = AudioSpec::new(
        SampleRate::new(48_000).unwrap(),
        ChannelCount::new(2).unwrap(),
        SampleFormat::Float32,
    );

    AudioBuffer::from_planar_f32(spec, FrameCount::new(frames), samples).unwrap()
}

pub(crate) fn db(value: f64) -> Decibels {
    Decibels::new(value).unwrap()
}

pub(crate) fn assert_samples_close(actual: &[f32], expected: &[f32]) {
    assert_eq!(actual.len(), expected.len());

    for (actual, expected) in actual.iter().zip(expected) {
        let tolerance = 1.0e-6;
        let difference = (actual - expected).abs();

        assert!(
            difference <= tolerance,
            "expected {actual} to be within {tolerance} of {expected}, difference was {difference}"
        );
    }
}

pub(crate) fn assert_sample_bits_eq(actual: &[f32], expected: &[f32]) {
    assert_eq!(actual.len(), expected.len());

    for (index, (&actual, &expected)) in actual.iter().zip(expected).enumerate() {
        assert_eq!(
            actual.to_bits(),
            expected.to_bits(),
            "sample {index} differed: {actual} != {expected}"
        );
    }
}

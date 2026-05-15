pub(super) fn parse_backend(value: &str) -> Result<auralis::BackendKind, String> {
    auralis::BackendKind::from_name(value)
        .ok_or_else(|| "backend must be `scalar` or `simd`".to_owned())
}

pub(super) fn parse_dbfs(value: &str) -> Result<f64, String> {
    let trimmed = value.trim();
    let number = trimmed
        .strip_suffix("dBFS")
        .or_else(|| trimmed.strip_suffix("dbfs"))
        .unwrap_or(trimmed);

    number
        .parse::<f64>()
        .map_err(|error| format!("invalid dBFS value `{value}`: {error}"))
}

pub(super) fn parse_filter_poles(value: &str) -> Result<u8, String> {
    match value {
        "1" => Ok(1),
        "2" => Ok(2),
        _ => Err(format!("invalid pole count `{value}`: expected 1 or 2")),
    }
}

pub(super) fn parse_combine_method(value: &str) -> Result<auralis::CombineMethod, String> {
    auralis::CombineMethod::from_name(value).ok_or_else(|| {
        "combine method must be `concatenate`, `sequence`, `mix`, `mix-power`, `merge`, or `multiply`"
            .to_owned()
    })
}

pub(super) fn parse_channel_count(value: &str) -> Result<auralis::ChannelCount, String> {
    let channels = value
        .parse::<u16>()
        .map_err(|_| "channels must be a positive integer no larger than 65535".to_owned())?;

    auralis::ChannelCount::new(channels)
        .map_err(|_| "channels must be a positive integer no larger than 65535".to_owned())
}

pub(super) fn parse_sample_rate(value: &str) -> Result<auralis::SampleRate, String> {
    let sample_rate = value
        .parse::<u32>()
        .map_err(|_| "rate must be a positive integer no larger than 4294967295".to_owned())?;

    auralis::SampleRate::new(sample_rate)
        .map_err(|_| "rate must be a positive integer no larger than 4294967295".to_owned())
}

pub(super) fn parse_wav_sample_format(value: &str) -> Result<auralis::WavSampleFormat, String> {
    match value {
        "pcm8" => Ok(auralis::WavSampleFormat::Pcm8),
        "pcm16" => Ok(auralis::WavSampleFormat::Pcm16),
        "pcm24" => Ok(auralis::WavSampleFormat::Pcm24),
        "pcm32" => Ok(auralis::WavSampleFormat::Pcm32),
        "float32" => Ok(auralis::WavSampleFormat::Float32),
        "float64" => Ok(auralis::WavSampleFormat::Float64),
        "ulaw" => Ok(auralis::WavSampleFormat::ULaw),
        "alaw" => Ok(auralis::WavSampleFormat::ALaw),
        _ => Err(
            "sample format must be `pcm8`, `pcm16`, `pcm24`, `pcm32`, `float32`, `float64`, `ulaw`, or `alaw`"
                .to_owned(),
        ),
    }
}

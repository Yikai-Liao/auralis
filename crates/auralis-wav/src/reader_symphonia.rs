use std::io::Cursor;

use auralis_codec::WavSampleFormat;
use auralis_core::{AudioBuffer, AudioSpec, ChannelCount, FrameCount, SampleFormat, SampleRate};
use auralis_simd::BackendKind;
use symphonia::{
    core::{
        audio::{AudioBuffer as SymphoniaAudioBuffer, AudioBufferRef, Signal},
        codecs::DecoderOptions,
        errors::Error as SymphoniaError,
        formats::FormatOptions,
        io::{MediaSourceStream, MediaSourceStreamOptions},
        meta::MetadataOptions,
        probe::Hint,
        sample::i24,
    },
    default::{get_codecs, get_probe},
};

use crate::{Result, WavError, WavSampleEncoding, sample_conversion::pcm16_to_f32_with_backend};

pub(crate) fn decode_symphonia_wav_bytes(
    bytes: &[u8],
    expected_format: Option<WavSampleFormat>,
    requested_backend: BackendKind,
) -> Result<Option<AudioBuffer>> {
    let source = Box::new(Cursor::new(bytes.to_vec()));
    let stream = MediaSourceStream::new(source, MediaSourceStreamOptions::default());
    let mut hint = Hint::new();
    hint.with_extension("wav");

    let probed = match get_probe().format(
        &hint,
        stream,
        &FormatOptions::default(),
        &MetadataOptions::default(),
    ) {
        Ok(probed) => probed,
        Err(SymphoniaError::Unsupported(_)) => return Ok(None),
        Err(error) => return Err(malformed(&error)),
    };
    let mut format = probed.format;
    let track = format.default_track().ok_or_else(|| WavError::Malformed {
        message: "missing default audio track".to_owned(),
    })?;
    let track_id = track.id;
    let mut decoder = match get_codecs().make(&track.codec_params, &DecoderOptions::default()) {
        Ok(decoder) => decoder,
        Err(SymphoniaError::Unsupported(_)) => return Ok(None),
        Err(error) => return Err(malformed(&error)),
    };

    let expected_frames = track
        .codec_params
        .n_frames
        .and_then(|frames| usize::try_from(frames).ok());
    let mut builder = PlanarBuilder::new(expected_format, requested_backend, expected_frames);
    loop {
        let packet = match format.next_packet() {
            Ok(packet) => packet,
            Err(SymphoniaError::IoError(error))
                if error.kind() == std::io::ErrorKind::UnexpectedEof =>
            {
                break;
            }
            Err(SymphoniaError::ResetRequired) => {
                decoder.reset();
                continue;
            }
            Err(SymphoniaError::Unsupported(_)) => return Ok(None),
            Err(error) => return Err(malformed(&error)),
        };

        if packet.track_id() != track_id {
            continue;
        }

        match decoder.decode(&packet) {
            Ok(audio_ref) => {
                if !builder.can_append(&audio_ref) {
                    return Ok(None);
                }
                builder.append(audio_ref)?;
            }
            Err(SymphoniaError::DecodeError(_) | SymphoniaError::Unsupported(_)) => {
                return Ok(None);
            }
            Err(SymphoniaError::ResetRequired) => {
                decoder.reset();
            }
            Err(error) => return Err(malformed(&error)),
        }
    }

    builder.finish().map(Some)
}

struct PlanarBuilder {
    expected_format: Option<WavSampleFormat>,
    requested_backend: BackendKind,
    expected_frames: Option<usize>,
    sample_rate: Option<SampleRate>,
    channels: Option<ChannelCount>,
    planar: Option<Vec<f32>>,
    planes: Vec<Vec<f32>>,
    frames: usize,
}

impl PlanarBuilder {
    fn new(
        expected_format: Option<WavSampleFormat>,
        requested_backend: BackendKind,
        expected_frames: Option<usize>,
    ) -> Self {
        Self {
            expected_format,
            requested_backend,
            expected_frames,
            sample_rate: None,
            channels: None,
            planar: None,
            planes: Vec::new(),
            frames: 0,
        }
    }

    fn append(&mut self, decoded: AudioBufferRef<'_>) -> Result<()> {
        let actual_format = sample_format(&decoded).ok_or_else(unsupported_unknown)?;
        self.ensure_expected_format(actual_format)?;
        let spec = *decoded.spec();
        let sample_rate = SampleRate::new(spec.rate).map_err(|_| WavError::InvalidSampleRate)?;
        let channels = ChannelCount::new(
            u16::try_from(spec.channels.count()).map_err(|_| WavError::InvalidChannelCount)?,
        )
        .map_err(|_| WavError::InvalidChannelCount)?;
        self.ensure_shape(sample_rate, channels)?;

        match decoded {
            AudioBufferRef::U8(buffer) => self.append_u8(&buffer),
            AudioBufferRef::S16(buffer) => self.append_i16(&buffer),
            AudioBufferRef::S24(buffer) => self.append_i24(&buffer),
            AudioBufferRef::S32(buffer) => self.append_i32(&buffer),
            AudioBufferRef::F32(buffer) => self.append_f32(&buffer),
            _ => Err(unsupported(actual_format)),
        }
    }

    fn can_append(&self, decoded: &AudioBufferRef<'_>) -> bool {
        self.expected_format.is_some() || sample_format(decoded).is_some()
    }

    fn ensure_expected_format(&self, actual_format: WavSampleFormat) -> Result<()> {
        if self
            .expected_format
            .is_some_and(|expected_format| expected_format != actual_format)
        {
            Err(unsupported(actual_format))
        } else {
            Ok(())
        }
    }

    fn ensure_shape(&mut self, sample_rate: SampleRate, channels: ChannelCount) -> Result<()> {
        match (self.sample_rate, self.channels) {
            (None, None) => {
                self.sample_rate = Some(sample_rate);
                self.channels = Some(channels);
                if let Some(expected_frames) = self.expected_frames {
                    let sample_count = expected_frames
                        .checked_mul(channels.as_usize())
                        .ok_or(WavError::InvalidBufferShape)?;
                    self.planar = Some(vec![0.0; sample_count]);
                } else {
                    self.planes = (0..channels.as_usize()).map(|_| Vec::new()).collect();
                }
                Ok(())
            }
            (Some(existing_rate), Some(existing_channels))
                if existing_rate == sample_rate && existing_channels == channels =>
            {
                Ok(())
            }
            _ => Err(WavError::Malformed {
                message: "decoded WAV packets changed audio shape".to_owned(),
            }),
        }
    }

    fn append_u8(&mut self, buffer: &SymphoniaAudioBuffer<u8>) -> Result<()> {
        let frame_offset = self.frames;
        for channel_index in 0..self.channel_count()? {
            let plane = buffer.chan(channel_index);
            if self.planar.is_some() {
                let output = self.output_slice(channel_index, plane.len())?;
                for (sample, destination) in plane.iter().zip(output) {
                    *destination = (f32::from(*sample) - 128.0) / 128.0;
                }
            } else {
                let output = &mut self.planes[channel_index];
                output.reserve(plane.len());
                for sample in plane {
                    output.push((f32::from(*sample) - 128.0) / 128.0);
                }
            }
        }
        self.advance_frames(buffer.frames(), frame_offset)
    }

    fn append_i16(&mut self, buffer: &SymphoniaAudioBuffer<i16>) -> Result<()> {
        let frame_offset = self.frames;
        for channel_index in 0..self.channel_count()? {
            let plane = buffer.chan(channel_index);
            if self.planar.is_some() {
                let requested_backend = self.requested_backend;
                let output = self.output_slice(channel_index, plane.len())?;
                pcm16_to_f32_with_backend(requested_backend, plane, output)?;
            } else {
                let output = &mut self.planes[channel_index];
                let start = output.len();
                output.resize(start + plane.len(), 0.0);
                pcm16_to_f32_with_backend(
                    self.requested_backend,
                    plane,
                    output
                        .get_mut(start..)
                        .ok_or(WavError::InvalidBufferShape)?,
                )?;
            }
        }
        self.advance_frames(buffer.frames(), frame_offset)
    }

    fn append_i24(&mut self, buffer: &SymphoniaAudioBuffer<i24>) -> Result<()> {
        let frame_offset = self.frames;
        for channel_index in 0..self.channel_count()? {
            let plane = buffer.chan(channel_index);
            #[allow(
                clippy::cast_precision_loss,
                reason = "PCM24 samples fit exactly in f32 before normalization."
            )]
            if self.planar.is_some() {
                let output = self.output_slice(channel_index, plane.len())?;
                for (sample, destination) in plane.iter().zip(output) {
                    *destination = (sample.0 as f32) / 8_388_608.0;
                }
            } else {
                let output = &mut self.planes[channel_index];
                output.reserve(plane.len());
                for sample in plane {
                    output.push((sample.0 as f32) / 8_388_608.0);
                }
            }
        }
        self.advance_frames(buffer.frames(), frame_offset)
    }

    fn append_i32(&mut self, buffer: &SymphoniaAudioBuffer<i32>) -> Result<()> {
        let frame_offset = self.frames;
        for channel_index in 0..self.channel_count()? {
            let plane = buffer.chan(channel_index);
            #[allow(
                clippy::cast_possible_truncation,
                reason = "PCM32 decode narrows into Auralis' f32 processing buffer."
            )]
            if self.planar.is_some() {
                let output = self.output_slice(channel_index, plane.len())?;
                for (sample, destination) in plane.iter().zip(output) {
                    *destination = (f64::from(*sample) / 2_147_483_648.0) as f32;
                }
            } else {
                let output = &mut self.planes[channel_index];
                output.reserve(plane.len());
                for sample in plane {
                    output.push((f64::from(*sample) / 2_147_483_648.0) as f32);
                }
            }
        }
        self.advance_frames(buffer.frames(), frame_offset)
    }

    fn append_f32(&mut self, buffer: &SymphoniaAudioBuffer<f32>) -> Result<()> {
        let frame_offset = self.frames;
        for channel_index in 0..self.channel_count()? {
            let plane = buffer.chan(channel_index);
            if self.planar.is_some() {
                let output = self.output_slice(channel_index, plane.len())?;
                for (frame_index, (sample, destination)) in
                    plane.iter().copied().zip(output).enumerate()
                {
                    if !sample.is_finite() {
                        return Err(WavError::NonFiniteSample {
                            channel_index,
                            frame_index: frame_offset + frame_index,
                        });
                    }
                    *destination = sample;
                }
            } else {
                let output = &mut self.planes[channel_index];
                output.reserve(plane.len());
                for (frame_index, sample) in plane.iter().copied().enumerate() {
                    if !sample.is_finite() {
                        return Err(WavError::NonFiniteSample {
                            channel_index,
                            frame_index: frame_offset + frame_index,
                        });
                    }
                    output.push(sample);
                }
            }
        }
        self.advance_frames(buffer.frames(), frame_offset)
    }

    fn advance_frames(&mut self, decoded_frames: usize, frame_offset: usize) -> Result<()> {
        if decoded_frames == 0 {
            return Ok(());
        }
        self.frames = frame_offset
            .checked_add(decoded_frames)
            .ok_or(WavError::InvalidBufferShape)?;
        Ok(())
    }

    fn channel_count(&self) -> Result<usize> {
        self.channels
            .map(ChannelCount::as_usize)
            .ok_or(WavError::InvalidBufferShape)
    }

    fn output_slice(&mut self, channel_index: usize, frames: usize) -> Result<&mut [f32]> {
        let expected_frames = self.expected_frames.ok_or(WavError::InvalidBufferShape)?;
        let start = channel_index
            .checked_mul(expected_frames)
            .and_then(|channel_start| channel_start.checked_add(self.frames))
            .ok_or(WavError::InvalidBufferShape)?;
        let end = start
            .checked_add(frames)
            .ok_or(WavError::InvalidBufferShape)?;
        if self
            .frames
            .checked_add(frames)
            .is_none_or(|end_frame| end_frame > expected_frames)
        {
            return Err(WavError::InvalidBufferShape);
        }
        self.planar
            .as_mut()
            .and_then(|planar| planar.get_mut(start..end))
            .ok_or(WavError::InvalidBufferShape)
    }

    fn finish(self) -> Result<AudioBuffer> {
        let sample_rate = self.sample_rate.ok_or_else(|| WavError::Malformed {
            message: "WAV stream did not contain audio packets".to_owned(),
        })?;
        let channels = self.channels.ok_or(WavError::InvalidBufferShape)?;
        let frames =
            FrameCount::new(u64::try_from(self.frames).map_err(|_| WavError::InvalidBufferShape)?);
        let sample_count = self
            .frames
            .checked_mul(channels.as_usize())
            .ok_or(WavError::InvalidBufferShape)?;

        let planar = if let Some(mut planar) = self.planar {
            if self.expected_frames != Some(self.frames) {
                for channel_index in 1..channels.as_usize() {
                    let source_start = channel_index
                        .checked_mul(self.expected_frames.ok_or(WavError::InvalidBufferShape)?)
                        .ok_or(WavError::InvalidBufferShape)?;
                    let destination_start = channel_index
                        .checked_mul(self.frames)
                        .ok_or(WavError::InvalidBufferShape)?;
                    planar.copy_within(source_start..source_start + self.frames, destination_start);
                }
                planar.truncate(sample_count);
            }
            planar
        } else {
            let mut planar = Vec::with_capacity(sample_count);
            for plane in self.planes {
                if plane.len() != self.frames {
                    return Err(WavError::InvalidBufferShape);
                }
                planar.extend(plane);
            }
            planar
        };
        if planar.len() != sample_count {
            return Err(WavError::InvalidBufferShape);
        }

        AudioBuffer::from_planar_f32(
            AudioSpec::new(sample_rate, channels, SampleFormat::Float32),
            frames,
            planar,
        )
        .map_err(|_| WavError::InvalidBufferShape)
    }
}

fn sample_format(decoded: &AudioBufferRef<'_>) -> Option<WavSampleFormat> {
    match decoded {
        AudioBufferRef::U8(_) => Some(WavSampleFormat::Pcm8),
        AudioBufferRef::S16(_) => Some(WavSampleFormat::Pcm16),
        AudioBufferRef::S24(_) => Some(WavSampleFormat::Pcm24),
        AudioBufferRef::S32(_) => Some(WavSampleFormat::Pcm32),
        AudioBufferRef::F32(_) => Some(WavSampleFormat::Float32),
        AudioBufferRef::F64(_) => Some(WavSampleFormat::Float64),
        AudioBufferRef::S8(_)
        | AudioBufferRef::U16(_)
        | AudioBufferRef::U24(_)
        | AudioBufferRef::U32(_) => None,
    }
}

fn unsupported(sample_format: WavSampleFormat) -> WavError {
    match sample_format {
        WavSampleFormat::Pcm8 => WavError::UnsupportedSampleFormat {
            bits_per_sample: 8,
            encoding: WavSampleEncoding::Integer,
        },
        WavSampleFormat::Pcm16 => WavError::UnsupportedSampleFormat {
            bits_per_sample: 16,
            encoding: WavSampleEncoding::Integer,
        },
        WavSampleFormat::Pcm24 => WavError::UnsupportedSampleFormat {
            bits_per_sample: 24,
            encoding: WavSampleEncoding::Integer,
        },
        WavSampleFormat::Pcm32 => WavError::UnsupportedSampleFormat {
            bits_per_sample: 32,
            encoding: WavSampleEncoding::Integer,
        },
        WavSampleFormat::Float32 => WavError::UnsupportedSampleFormat {
            bits_per_sample: 32,
            encoding: WavSampleEncoding::Float,
        },
        WavSampleFormat::Float64 => WavError::UnsupportedSampleFormat {
            bits_per_sample: 64,
            encoding: WavSampleEncoding::Float,
        },
        WavSampleFormat::ULaw | WavSampleFormat::ALaw => WavError::UnsupportedSampleFormat {
            bits_per_sample: 8,
            encoding: WavSampleEncoding::Companded,
        },
        _ => WavError::UnsupportedSampleFormat {
            bits_per_sample: 0,
            encoding: WavSampleEncoding::Integer,
        },
    }
}

fn unsupported_unknown() -> WavError {
    WavError::UnsupportedSampleFormat {
        bits_per_sample: 0,
        encoding: WavSampleEncoding::Integer,
    }
}

fn malformed(error: &SymphoniaError) -> WavError {
    WavError::Malformed {
        message: error.to_string(),
    }
}

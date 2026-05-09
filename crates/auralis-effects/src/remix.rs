use auralis_core::{AudioBuffer, AudioSpec, ChannelCount};

use crate::{EffectError, Result};

/// SoX-ng-style channel remixing with basic routing.
///
/// `Remix` selects input channels into explicitly declared output channels.
/// Basic routing supports 1-based input channel numbers, ascending or
/// descending ranges, open ranges, the `-` all-channel range, and standalone
/// `0` silent outputs. When multiple input channels feed one output channel,
/// each contribution uses SoX-ng's default `1 / n` scaling. Gain modifiers and
/// `-a`, `-m`, and `-p` options are intentionally left to the next remix
/// feature.
///
/// # Errors
///
/// [`Self::new`] rejects empty output lists and invalid source specifications.
/// [`Self::process_buffer`] returns [`EffectError::RemixInputChannelOutOfBounds`]
/// when a routing spec references a channel that is absent from the input.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Remix {
    /// Output-channel routing specifications in output order.
    pub outputs: Vec<RemixOutputSpec>,
}

impl Remix {
    /// Creates a basic remix processor.
    ///
    /// # Errors
    ///
    /// Returns [`EffectError::InvalidRemixRouting`] when no output channels are
    /// provided.
    pub fn new(outputs: impl Into<Vec<RemixOutputSpec>>) -> Result<Self> {
        let outputs = outputs.into();
        if outputs.is_empty() {
            return Err(EffectError::InvalidRemixRouting);
        }

        Ok(Self { outputs })
    }

    /// Applies basic channel routing and returns a new buffer.
    ///
    /// # Errors
    ///
    /// Returns [`EffectError::RemixOutputChannelsOverflow`] if the output
    /// channel count cannot be represented, [`EffectError::Core`] if the output
    /// buffer shape cannot be represented, or
    /// [`EffectError::RemixInputChannelOutOfBounds`] if any referenced input
    /// channel does not exist.
    pub fn process_buffer(&self, audio: &AudioBuffer) -> Result<AudioBuffer> {
        let channels = u16::try_from(self.outputs.len())
            .map_err(|_| EffectError::RemixOutputChannelsOverflow)
            .and_then(|channels| {
                ChannelCount::new(channels).map_err(|_| EffectError::RemixOutputChannelsOverflow)
            })?;
        let output_spec = AudioSpec::new(
            audio.spec().sample_rate(),
            channels,
            audio.spec().sample_format(),
        );
        let mut output = AudioBuffer::zeroed(output_spec, audio.frames())?;
        let input_channels = audio.channels().as_usize();

        for (output_index, output_spec) in self.outputs.iter().enumerate() {
            let sources = output_spec.expanded_sources(input_channels)?;
            if sources.is_empty() {
                continue;
            }

            let output_channel = output
                .channel_mut(output_index)
                .ok_or(auralis_core::AuralisError::InvalidAudioBufferShape)?;
            let multiplier = reciprocal_usize(sources.len());

            for source_index in sources {
                let input_channel = audio
                    .channel(source_index)
                    .ok_or(EffectError::RemixInputChannelOutOfBounds)?;
                for (output_sample, input_sample) in output_channel.iter_mut().zip(input_channel) {
                    *output_sample += *input_sample * multiplier;
                }
            }
        }

        Ok(output)
    }
}

/// One output channel in a basic remix command.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RemixOutputSpec {
    /// Input sources contributing to this output channel. Empty means silence.
    pub sources: Vec<RemixSource>,
}

impl RemixOutputSpec {
    /// Creates a silent output channel.
    #[must_use]
    pub const fn silent() -> Self {
        Self {
            sources: Vec::new(),
        }
    }

    /// Creates an output channel from one or more source specifications.
    ///
    /// # Errors
    ///
    /// Returns [`EffectError::InvalidRemixRouting`] if `sources` is empty.
    pub fn new(sources: impl Into<Vec<RemixSource>>) -> Result<Self> {
        let sources = sources.into();
        if sources.is_empty() {
            return Err(EffectError::InvalidRemixRouting);
        }

        Ok(Self { sources })
    }

    fn expanded_sources(&self, input_channels: usize) -> Result<Vec<usize>> {
        let mut expanded = Vec::new();

        for source in &self.sources {
            source.expand(input_channels, &mut expanded)?;
        }

        Ok(expanded)
    }
}

/// A 1-based basic remix source channel or range.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RemixSource {
    /// One 1-based input channel.
    Channel(u16),

    /// Inclusive 1-based input channel range. Missing bounds default to the
    /// first or last input channel at processing time.
    Range {
        /// Optional inclusive start channel.
        start: Option<u16>,

        /// Optional inclusive end channel.
        end: Option<u16>,
    },
}

impl RemixSource {
    /// Creates a 1-based input channel source.
    ///
    /// # Errors
    ///
    /// Returns [`EffectError::InvalidRemixRouting`] for channel `0`.
    pub fn channel(channel: u16) -> Result<Self> {
        if channel == 0 {
            return Err(EffectError::InvalidRemixRouting);
        }

        Ok(Self::Channel(channel))
    }

    /// Creates an inclusive 1-based input channel range.
    ///
    /// Missing bounds are resolved against the input channel count during
    /// processing. Reversed bounds are accepted and normalized, matching
    /// SoX-ng.
    ///
    /// # Errors
    ///
    /// Returns [`EffectError::InvalidRemixRouting`] when either provided bound
    /// is `0` or both bounds are missing.
    pub fn range(start: Option<u16>, end: Option<u16>) -> Result<Self> {
        if start.is_none() && end.is_none() {
            return Err(EffectError::InvalidRemixRouting);
        }
        if matches!(start, Some(0)) || matches!(end, Some(0)) {
            return Err(EffectError::InvalidRemixRouting);
        }

        Ok(Self::Range { start, end })
    }

    /// Creates the `-` all-channel range.
    #[must_use]
    pub const fn all() -> Self {
        Self::Range {
            start: None,
            end: None,
        }
    }

    fn expand(self, input_channels: usize, output: &mut Vec<usize>) -> Result<()> {
        match self {
            Self::Channel(channel) => {
                let index = usize::from(channel) - 1;
                if index >= input_channels {
                    return Err(EffectError::RemixInputChannelOutOfBounds);
                }
                output.push(index);
            }
            Self::Range { start, end } => {
                let mut start = usize::from(start.unwrap_or(1));
                let mut end = end.map_or(input_channels, usize::from);
                if start == 0 || end == 0 || start > input_channels || end > input_channels {
                    return Err(EffectError::RemixInputChannelOutOfBounds);
                }
                if end < start {
                    std::mem::swap(&mut start, &mut end);
                }

                output.extend((start - 1)..end);
            }
        }

        Ok(())
    }
}

fn reciprocal_usize(value: usize) -> f32 {
    #[allow(
        clippy::cast_precision_loss,
        reason = "Basic remix scales small channel groups as f32 sample arithmetic."
    )]
    {
        1.0 / value as f32
    }
}

#[cfg(test)]
mod tests {
    use super::{Remix, RemixOutputSpec, RemixSource};
    use crate::{EffectError, test_support::stereo_audio_buffer};
    use auralis_core::{ChannelCount, FrameCount};

    #[test]
    fn copies_and_reorders_channels() {
        let audio = stereo_audio_buffer(vec![0.25, -0.5, 0.75, 0.5]);
        let remix = Remix::new([
            RemixOutputSpec::new([RemixSource::channel(2).unwrap()]).unwrap(),
            RemixOutputSpec::new([RemixSource::channel(1).unwrap()]).unwrap(),
        ])
        .unwrap();

        let actual = remix.process_buffer(&audio).unwrap();

        assert_eq!(actual.channels(), ChannelCount::new(2).unwrap());
        assert_eq!(actual.frames(), FrameCount::new(2));
        assert_eq!(actual.as_planar_f32(), &[0.75, 0.5, 0.25, -0.5]);
    }

    #[test]
    fn mixes_multiple_sources_with_default_scaling() {
        let audio = stereo_audio_buffer(vec![0.25, 0.75, -0.5, 0.5]);
        let remix = Remix::new([RemixOutputSpec::new([RemixSource::all()]).unwrap()]).unwrap();

        let actual = remix.process_buffer(&audio).unwrap();

        assert_eq!(actual.channels(), ChannelCount::new(1).unwrap());
        assert_eq!(actual.as_planar_f32(), &[-0.125, 0.625]);
    }

    #[test]
    fn silent_output_emits_zero_channel() {
        let audio = stereo_audio_buffer(vec![0.25, 0.75, -0.5, 0.5]);
        let remix = Remix::new([
            RemixOutputSpec::silent(),
            RemixOutputSpec::new([RemixSource::channel(1).unwrap()]).unwrap(),
        ])
        .unwrap();

        let actual = remix.process_buffer(&audio).unwrap();

        assert_eq!(actual.channels(), ChannelCount::new(2).unwrap());
        assert_eq!(actual.as_planar_f32(), &[0.0, 0.0, 0.25, 0.75]);
    }

    #[test]
    fn rejects_out_of_bounds_input_channel_at_processing_time() {
        let audio = stereo_audio_buffer(vec![0.25, 0.75, -0.5, 0.5]);
        let remix = Remix::new([RemixOutputSpec::new([RemixSource::channel(3).unwrap()]).unwrap()])
            .unwrap();

        assert_eq!(
            remix.process_buffer(&audio).unwrap_err(),
            EffectError::RemixInputChannelOutOfBounds
        );
    }
}

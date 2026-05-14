use auralis_core::{AudioBuffer, AudioSpec, ChannelCount, Decibels};

use crate::{EffectError, Result};

/// SoX-ng-style channel remixing.
///
/// `Remix` selects input channels into explicitly declared output channels.
/// Basic routing supports 1-based input channel numbers, ascending or
/// descending ranges, open ranges, the `-` all-channel range, and standalone
/// `0` silent outputs. Source gain modifiers support SoX-ng's voltage (`v`),
/// power dB (`p`), and inverted power dB (`i`) forms. Automatic scaling uses
/// `1 / n` by default and `1 / sqrt(n)` when power scaling is enabled.
///
/// # Errors
///
/// [`Self::new`] rejects empty output lists and invalid source specifications.
/// [`Self::process_buffer`] returns [`EffectError::RemixInputChannelOutOfBounds`]
/// when a routing spec references a channel that is absent from the input.
#[derive(Debug, Clone, PartialEq)]
pub struct Remix {
    /// Output-channel routing specifications in output order.
    pub outputs: Vec<RemixOutputSpec>,

    /// Scaling mode for sources that do not carry an explicit gain modifier.
    pub level_mode: RemixLevelMode,

    /// Whether automatic scaling uses `1 / sqrt(n)` instead of `1 / n`.
    pub mix_power: bool,
}

impl Remix {
    /// Creates a remix processor with SoX-ng's default semi-automatic scaling.
    ///
    /// # Errors
    ///
    /// Returns [`EffectError::InvalidRemixRouting`] when no output channels are
    /// provided.
    pub fn new(outputs: impl Into<Vec<RemixOutputSpec>>) -> Result<Self> {
        Self::with_level_options(outputs, RemixLevelMode::SemiAutomatic, false)
    }

    /// Creates a remix processor with explicit level options.
    ///
    /// # Errors
    ///
    /// Returns [`EffectError::InvalidRemixRouting`] when no output channels are
    /// provided.
    pub fn with_level_options(
        outputs: impl Into<Vec<RemixOutputSpec>>,
        level_mode: RemixLevelMode,
        mix_power: bool,
    ) -> Result<Self> {
        let outputs = outputs.into();
        if outputs.is_empty() {
            return Err(EffectError::InvalidRemixRouting);
        }

        Ok(Self {
            outputs,
            level_mode,
            mix_power,
        })
    }

    /// Applies channel routing and returns a new buffer.
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
            let automatic_multiplier = if self.mix_power {
                reciprocal_sqrt_usize(sources.len())
            } else {
                reciprocal_usize(sources.len())
            };
            let has_explicit_gain = sources.iter().any(|source| source.gain.is_some());

            let first_source = sources[0];
            let input_channel = audio
                .channel(first_source.index)
                .ok_or(EffectError::RemixInputChannelOutOfBounds)?;
            let multiplier = first_source.gain.map_or_else(
                || {
                    self.level_mode
                        .default_multiplier(automatic_multiplier, has_explicit_gain)
                },
                RemixGain::multiplier,
            );
            for (output_sample, input_sample) in output_channel.iter_mut().zip(input_channel) {
                *output_sample = *input_sample * multiplier;
            }

            for source_index in &sources[1..] {
                let input_channel = audio
                    .channel(source_index.index)
                    .ok_or(EffectError::RemixInputChannelOutOfBounds)?;
                let multiplier = source_index.gain.map_or_else(
                    || {
                        self.level_mode
                            .default_multiplier(automatic_multiplier, has_explicit_gain)
                    },
                    RemixGain::multiplier,
                );
                for (output_sample, input_sample) in output_channel.iter_mut().zip(input_channel) {
                    *output_sample += *input_sample * multiplier;
                }
            }

            for output_sample in output_channel {
                *output_sample = output_sample.clamp(-1.0, 1.0);
            }
        }

        Ok(output)
    }
}

/// How `remix` scales sources that do not carry explicit gain modifiers.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RemixLevelMode {
    /// SoX-ng's default: auto-scale only when an output has no explicit gains.
    SemiAutomatic,

    /// Always auto-scale sources without explicit gains.
    Automatic,

    /// Never auto-scale sources without explicit gains.
    Manual,
}

impl RemixLevelMode {
    fn default_multiplier(self, automatic_multiplier: f32, has_explicit_gain: bool) -> f32 {
        match self {
            Self::SemiAutomatic if has_explicit_gain => 1.0,
            Self::SemiAutomatic | Self::Automatic => automatic_multiplier,
            Self::Manual => 1.0,
        }
    }
}

/// A per-source `remix` gain modifier.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum RemixGain {
    /// Voltage multiplier (`v`).
    Voltage(f64),

    /// Power adjustment in dB (`p`).
    PowerDb(Decibels),

    /// Inverted power adjustment in dB (`i`).
    InvertedPowerDb(Decibels),
}

impl RemixGain {
    /// Creates a finite voltage multiplier.
    ///
    /// # Errors
    ///
    /// Returns [`EffectError::InvalidRemixRouting`] when `multiplier` is not
    /// finite.
    pub fn voltage(multiplier: f64) -> Result<Self> {
        if !multiplier.is_finite() {
            return Err(EffectError::InvalidRemixRouting);
        }

        Ok(Self::Voltage(multiplier))
    }

    fn multiplier(self) -> f32 {
        let multiplier = match self {
            Self::Voltage(multiplier) => multiplier,
            Self::PowerDb(decibels) => db_to_linear(decibels),
            Self::InvertedPowerDb(decibels) => -db_to_linear(decibels),
        };

        #[allow(
            clippy::cast_possible_truncation,
            reason = "Remix gain modifiers are applied in the f32 sample domain."
        )]
        {
            multiplier as f32
        }
    }
}

/// One output channel in a remix command.
#[derive(Debug, Clone, PartialEq)]
pub struct RemixOutputSpec {
    /// Input sources contributing to this output channel. Empty means silence.
    pub sources: Vec<RemixSource>,

    /// Optional gain modifier for each source. This is kept parallel to
    /// [`Self::sources`].
    pub gains: Vec<Option<RemixGain>>,
}

impl RemixOutputSpec {
    /// Creates a silent output channel.
    #[must_use]
    pub const fn silent() -> Self {
        Self {
            sources: Vec::new(),
            gains: Vec::new(),
        }
    }

    /// Creates an output channel from one or more source specifications.
    ///
    /// # Errors
    ///
    /// Returns [`EffectError::InvalidRemixRouting`] if `sources` is empty.
    pub fn new(sources: impl Into<Vec<RemixSource>>) -> Result<Self> {
        let sources = sources.into();
        let gains = vec![None; sources.len()];
        Self::with_gains(sources, gains)
    }

    /// Creates an output channel from sources and aligned gain modifiers.
    ///
    /// # Errors
    ///
    /// Returns [`EffectError::InvalidRemixRouting`] if `sources` is empty or
    /// the source and gain lists do not have the same length.
    pub fn with_gains(
        sources: impl Into<Vec<RemixSource>>,
        gains: impl Into<Vec<Option<RemixGain>>>,
    ) -> Result<Self> {
        let sources = sources.into();
        let gains = gains.into();
        if sources.is_empty() {
            return Err(EffectError::InvalidRemixRouting);
        }
        if sources.len() != gains.len() {
            return Err(EffectError::InvalidRemixRouting);
        }

        Ok(Self { sources, gains })
    }

    fn expanded_sources(&self, input_channels: usize) -> Result<Vec<ExpandedRemixSource>> {
        let mut expanded = Vec::new();

        for (source, gain) in self.sources.iter().zip(&self.gains) {
            source.expand(*gain, input_channels, &mut expanded)?;
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

    fn expand(
        self,
        gain: Option<RemixGain>,
        input_channels: usize,
        output: &mut Vec<ExpandedRemixSource>,
    ) -> Result<()> {
        match self {
            Self::Channel(channel) => {
                let index = usize::from(channel) - 1;
                if index >= input_channels {
                    return Err(EffectError::RemixInputChannelOutOfBounds);
                }
                output.push(ExpandedRemixSource { index, gain });
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

                output.extend((start - 1..end).map(|index| ExpandedRemixSource { index, gain }));
            }
        }

        Ok(())
    }
}

#[derive(Debug, Clone, Copy)]
struct ExpandedRemixSource {
    index: usize,
    gain: Option<RemixGain>,
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

fn reciprocal_sqrt_usize(value: usize) -> f32 {
    #[allow(
        clippy::cast_precision_loss,
        reason = "Remix power scaling handles small channel groups."
    )]
    {
        1.0 / (value as f32).sqrt()
    }
}

fn db_to_linear(decibels: Decibels) -> f64 {
    10.0_f64.powf(decibels.as_f64() / 20.0)
}

#[cfg(test)]
mod tests {
    use super::{Remix, RemixGain, RemixLevelMode, RemixOutputSpec, RemixSource};
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
    fn explicit_gain_modifiers_disable_semi_automatic_scaling() {
        let audio = stereo_audio_buffer(vec![0.25, 0.75, 0.5, -0.5]);
        let remix = Remix::new([RemixOutputSpec::with_gains(
            [
                RemixSource::channel(1).unwrap(),
                RemixSource::channel(2).unwrap(),
            ],
            [Some(RemixGain::voltage(0.5).unwrap()), None],
        )
        .unwrap()])
        .unwrap();

        let actual = remix.process_buffer(&audio).unwrap();

        assert_eq!(actual.channels(), ChannelCount::new(1).unwrap());
        assert_eq!(actual.as_planar_f32(), &[0.625, -0.125]);
    }

    #[test]
    fn automatic_and_power_modes_scale_unspecified_sources() {
        let audio = stereo_audio_buffer(vec![0.25, 0.75, 0.5, -0.5]);
        let auto = Remix::with_level_options(
            [RemixOutputSpec::with_gains(
                [
                    RemixSource::channel(1).unwrap(),
                    RemixSource::channel(2).unwrap(),
                ],
                [Some(RemixGain::voltage(0.5).unwrap()), None],
            )
            .unwrap()],
            RemixLevelMode::Automatic,
            false,
        )
        .unwrap();
        let power = Remix::with_level_options(
            [RemixOutputSpec::new([
                RemixSource::channel(1).unwrap(),
                RemixSource::channel(2).unwrap(),
            ])
            .unwrap()],
            RemixLevelMode::SemiAutomatic,
            true,
        )
        .unwrap();

        let auto_actual = auto.process_buffer(&audio).unwrap();
        let power_actual = power.process_buffer(&audio).unwrap();

        assert_eq!(auto_actual.as_planar_f32(), &[0.375, 0.125]);
        assert!((power_actual.as_planar_f32()[0] - 0.530_330_06).abs() < 0.000_001);
        assert!((power_actual.as_planar_f32()[1] - 0.176_776_69).abs() < 0.000_001);
    }

    #[test]
    fn manual_mode_and_inverted_power_gains_can_clip() {
        let audio = stereo_audio_buffer(vec![0.75, 0.5, -0.75, -0.5]);
        let remix = Remix::with_level_options(
            [RemixOutputSpec::with_gains(
                [
                    RemixSource::channel(1).unwrap(),
                    RemixSource::channel(2).unwrap(),
                ],
                [
                    None,
                    Some(RemixGain::InvertedPowerDb(
                        auralis_core::Decibels::new(0.0).unwrap(),
                    )),
                ],
            )
            .unwrap()],
            RemixLevelMode::Manual,
            false,
        )
        .unwrap();

        let actual = remix.process_buffer(&audio).unwrap();

        assert_eq!(actual.as_planar_f32(), &[1.0, 1.0]);
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

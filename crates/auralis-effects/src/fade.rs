use std::f32::consts::{FRAC_PI_2, PI};

use auralis_core::{AudioBuffer, FrameCount};
use auralis_dsp::{fade_in_place, fade_in_place_with_backend};
use auralis_simd::{BackendKind, select_backend};

use crate::{EffectError, Result};

/// SoX-ng fade curve family.
///
/// Curves map a normalized fade position in `[0.0, 1.0]` to a gain
/// coefficient. Linear fades use a straight ramp, while the other variants
/// follow SoX-ng's `fade` coefficient formulas.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FadeCurve {
    /// Quarter-sine curve, selected by SoX-ng token `q`.
    QuarterSine,

    /// Half-sine curve, selected by SoX-ng token `h`.
    HalfSine,

    /// Logarithmic curve, selected by SoX-ng token `l` and by SoX-ng default.
    Logarithmic,

    /// Linear triangle ramp, selected by SoX-ng token `t`.
    Linear,

    /// Inverted-parabola curve, selected by SoX-ng token `p`.
    InvertedParabola,
}

impl FadeCurve {
    /// Returns the canonical SoX-ng one-letter curve token.
    #[must_use]
    pub const fn token(self) -> &'static str {
        match self {
            Self::QuarterSine => "q",
            Self::HalfSine => "h",
            Self::Logarithmic => "l",
            Self::Linear => "t",
            Self::InvertedParabola => "p",
        }
    }

    pub(crate) fn from_token(token: &str) -> Option<Self> {
        match token {
            "q" => Some(Self::QuarterSine),
            "h" => Some(Self::HalfSine),
            "l" => Some(Self::Logarithmic),
            "t" => Some(Self::Linear),
            "p" => Some(Self::InvertedParabola),
            _ => None,
        }
    }

    #[must_use]
    fn coefficient(self, index: u64, range: u64) -> f32 {
        let fade_index = ratio(index, range).clamp(0.0, 1.0);

        match self {
            Self::QuarterSine => (fade_index * FRAC_PI_2).sin(),
            Self::HalfSine => (1.0 - (fade_index * PI).cos()) / 2.0,
            Self::Logarithmic => 0.1_f32.powf((1.0 - fade_index) * 5.0),
            Self::Linear => fade_index,
            Self::InvertedParabola => 1.0 - ((1.0 - fade_index) * (1.0 - fade_index)),
        }
    }
}

/// Fade-in and fade-out effect processor.
///
/// `Fade` applies independent envelopes to the start and end of every
/// channel in a planar `f32` [`AudioBuffer`]. Fade lengths are measured in
/// frames. [`Fade::new`] preserves Auralis' original linear behavior: a
/// fade-in length of `4` uses coefficients `[0.0, 0.25, 0.5, 0.75]`; the first
/// frame after the fade reaches `1.0`. A fade-out length of `4` applies
/// `[0.75, 0.5, 0.25, 0.0]` to the final four frames. If fade-in and fade-out
/// overlap, their coefficients are multiplied. Zero-length fades are identity
/// transforms.
///
/// Command parsing can also create a SoX-ng positional fade with an explicit
/// stop position. That form truncates or pads the output to the stop frame and
/// uses SoX-ng's fade-out indexing, where the final retained frame has a
/// coefficient of `1 / fade-out-length` instead of zero.
///
/// # Examples
///
/// ```
/// use auralis_core::{
///     AudioBuffer, AudioSpec, ChannelCount, FrameCount, SampleFormat, SampleRate,
/// };
/// use auralis_effects::Fade;
///
/// let spec = AudioSpec::new(
///     SampleRate::new(48_000)?,
///     ChannelCount::new(1)?,
///     SampleFormat::Float32,
/// );
/// let mut audio = AudioBuffer::from_planar_f32(
///     spec,
///     FrameCount::new(4),
///     vec![1.0, 1.0, 1.0, 1.0],
/// )?;
///
/// Fade::new(FrameCount::new(2), FrameCount::new(2)).process_buffer(&mut audio);
///
/// assert_eq!(audio.as_planar_f32(), &[0.0, 0.5, 0.5, 0.0]);
/// # Ok::<(), auralis_core::AuralisError>(())
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Fade {
    /// SoX-ng fade curve family.
    pub curve: FadeCurve,

    /// Frames over which to ramp from silence to unity at the start.
    pub fade_in: FrameCount,

    /// Optional SoX-ng stop position for command-style fade-out processing.
    ///
    /// `None` preserves Auralis' direct API behavior. `Some(0)` means the end
    /// of the input audio, matching SoX-ng's historical `0` stop-position
    /// spelling.
    pub stop_position: Option<FrameCount>,

    /// Frames over which to ramp from unity to silence at the end.
    pub fade_out: FrameCount,
}

impl Fade {
    /// Creates a linear fade processor.
    #[must_use]
    pub const fn new(fade_in: FrameCount, fade_out: FrameCount) -> Self {
        Self::with_curve(FadeCurve::Linear, fade_in, fade_out)
    }

    /// Creates a fade processor with a specific SoX-ng curve family.
    #[must_use]
    pub const fn with_curve(curve: FadeCurve, fade_in: FrameCount, fade_out: FrameCount) -> Self {
        Self {
            curve,
            fade_in,
            stop_position: None,
            fade_out,
        }
    }

    /// Creates a SoX-ng positional fade processor.
    ///
    /// `stop_position` is the output frame count after truncation or padding.
    /// A value of zero means the input length. `fade_out` is measured backward
    /// from the resolved stop position.
    #[must_use]
    pub const fn with_stop_position(
        curve: FadeCurve,
        fade_in: FrameCount,
        stop_position: FrameCount,
        fade_out: FrameCount,
    ) -> Self {
        Self {
            curve,
            fade_in,
            stop_position: Some(stop_position),
            fade_out,
        }
    }

    /// Applies the fade envelope in place to every channel of an audio buffer.
    ///
    /// Processing is deterministic, non-allocating, and preserves channel
    /// grouping. Zero fade lengths are identity transforms.
    pub fn process_buffer(self, audio: &mut AudioBuffer) {
        self.process_buffer_with_backend(audio, BackendKind::Scalar);
    }

    /// Applies the fade envelope in place using the requested backend.
    ///
    /// Processing has the same channel-preserving behavior as
    /// [`Self::process_buffer`]. Requesting [`BackendKind::Simd`] uses the
    /// backend-dispatched fade kernel when available and deterministic scalar
    /// fallback otherwise.
    pub fn process_buffer_with_backend(
        self,
        audio: &mut AudioBuffer,
        requested_backend: BackendKind,
    ) {
        let total_frames = audio.frames().as_u64();
        for channel_index in 0..audio.channels().as_usize() {
            if let Some(channel) = audio.channel_mut(channel_index) {
                self.process_channel_segment_with_backend(
                    channel,
                    total_frames,
                    FrameCount::new(0),
                    requested_backend,
                );
            }
        }
    }

    /// Applies the fade and returns an output buffer.
    ///
    /// For ordinary direct fades, this clones the input and applies the same
    /// in-place envelope as [`Self::process_buffer_with_backend`]. For
    /// positional fades, this applies SoX-ng stop-position semantics, including
    /// output truncation, zero padding when the stop is past the input length,
    /// and SoX-ng's fade-out endpoint indexing.
    ///
    /// # Errors
    ///
    /// Returns [`EffectError::FadeRegionsOverlap`] when a positional fade-out
    /// starts before the fade-in has completed, allowing SoX-ng's one-frame
    /// rounding grace. Returns [`EffectError::FadeLengthOverflow`] when the
    /// requested output shape cannot be represented.
    pub fn process_buffer_to_output(
        self,
        audio: &AudioBuffer,
        requested_backend: BackendKind,
    ) -> Result<AudioBuffer> {
        let Some(stop_position) = self.stop_position else {
            let mut output = audio.clone();
            self.process_buffer_with_backend(&mut output, requested_backend);
            return Ok(output);
        };

        let plan = self.positioned_plan(audio.frames().as_u64(), stop_position)?;
        let output_frames = FrameCount::new(plan.output_frames);
        let output_frames_usize =
            usize::try_from(plan.output_frames).map_err(|_| EffectError::FadeLengthOverflow)?;
        let input_frames_usize = usize::try_from(audio.frames().as_u64())
            .map_err(|_| EffectError::FadeLengthOverflow)?;
        let copied_frames = input_frames_usize.min(output_frames_usize);
        let capacity = audio
            .channels()
            .as_usize()
            .checked_mul(output_frames_usize)
            .ok_or(EffectError::FadeLengthOverflow)?;
        let mut data = Vec::with_capacity(capacity);

        for channel_index in 0..audio.channels().as_usize() {
            let channel = audio
                .channel(channel_index)
                .ok_or(EffectError::FadeLengthOverflow)?;
            data.extend_from_slice(&channel[..copied_frames]);
            data.resize(data.len() + output_frames_usize - copied_frames, 0.0);
        }

        let mut output = AudioBuffer::from_planar_f32(audio.spec(), output_frames, data)?;
        for channel_index in 0..output.channels().as_usize() {
            if let Some(channel) = output.channel_mut(channel_index) {
                self.process_positioned_channel_segment(channel, plan, FrameCount::new(0));
            }
        }

        Ok(output)
    }

    /// Applies the fade to a contiguous channel segment with a known frame
    /// offset in the full signal.
    ///
    /// This method exists so streaming callers and tests can process chunks
    /// while still using full-signal frame positions. Passing every segment in
    /// order produces the same result as [`Self::process_buffer`].
    pub fn process_channel_segment(
        self,
        samples: &mut [f32],
        total_frames: u64,
        start_frame: FrameCount,
    ) {
        self.process_channel_segment_scalar(samples, total_frames, start_frame);
    }

    /// Applies the fade to a contiguous channel segment using the requested backend.
    ///
    /// Parameters and chunk-invariance behavior match
    /// [`Self::process_channel_segment`].
    pub fn process_channel_segment_with_backend(
        self,
        samples: &mut [f32],
        total_frames: u64,
        start_frame: FrameCount,
        requested_backend: BackendKind,
    ) {
        if self.curve != FadeCurve::Linear {
            self.process_channel_segment_scalar(samples, total_frames, start_frame);
            return;
        }

        fade_in_place_with_backend(
            select_backend(requested_backend),
            samples,
            total_frames,
            start_frame.as_u64(),
            self.fade_in.as_u64(),
            self.fade_out.as_u64(),
        );
    }

    fn process_channel_segment_scalar(
        self,
        samples: &mut [f32],
        total_frames: u64,
        start_frame: FrameCount,
    ) {
        if self.curve == FadeCurve::Linear {
            fade_in_place(
                samples,
                total_frames,
                start_frame.as_u64(),
                self.fade_in.as_u64(),
                self.fade_out.as_u64(),
            );
            return;
        }

        for (offset, sample) in samples.iter_mut().enumerate() {
            let Ok(offset) = u64::try_from(offset) else {
                return;
            };
            let Some(frame_index) = start_frame.as_u64().checked_add(offset) else {
                return;
            };

            *sample *= self.coefficient(frame_index, total_frames);
        }
    }

    fn coefficient(self, frame_index: u64, total_frames: u64) -> f32 {
        let mut coefficient = 1.0;

        if self.fade_in.as_u64() != 0 && frame_index < self.fade_in.as_u64() {
            coefficient *= self.curve.coefficient(frame_index, self.fade_in.as_u64());
        }

        if self.fade_out.as_u64() != 0 && frame_index < total_frames {
            let remaining = total_frames - frame_index - 1;
            if remaining < self.fade_out.as_u64() {
                coefficient *= self.curve.coefficient(remaining, self.fade_out.as_u64());
            }
        }

        coefficient
    }

    fn positioned_plan(
        self,
        input_frames: u64,
        stop_position: FrameCount,
    ) -> Result<PositionedFadePlan> {
        let output_frames = if stop_position.as_u64() == 0 {
            input_frames
        } else {
            stop_position.as_u64()
        };
        let fade_out_start = output_frames.saturating_sub(self.fade_out.as_u64());
        let mut fade_in = self.fade_in.as_u64();

        if fade_out_start != 0 && fade_in > fade_out_start {
            fade_in = fade_in.saturating_sub(1);
            if fade_in > fade_out_start {
                return Err(EffectError::FadeRegionsOverlap);
            }
        }

        Ok(PositionedFadePlan {
            output_frames,
            fade_in,
            fade_out_start,
        })
    }

    fn process_positioned_channel_segment(
        self,
        samples: &mut [f32],
        plan: PositionedFadePlan,
        start_frame: FrameCount,
    ) {
        for (offset, sample) in samples.iter_mut().enumerate() {
            let Ok(offset) = u64::try_from(offset) else {
                return;
            };
            let Some(frame_index) = start_frame.as_u64().checked_add(offset) else {
                return;
            };

            *sample *= self.positioned_coefficient(frame_index, plan);
        }
    }

    fn positioned_coefficient(self, frame_index: u64, plan: PositionedFadePlan) -> f32 {
        if plan.fade_in != 0 && frame_index < plan.fade_in {
            return self.curve.coefficient(frame_index, plan.fade_in);
        }

        if self.fade_out.as_u64() != 0
            && frame_index < plan.output_frames
            && frame_index >= plan.fade_out_start
        {
            let remaining = plan.output_frames - frame_index;
            let range = plan.output_frames - plan.fade_out_start;
            return self.curve.coefficient(remaining, range);
        }

        1.0
    }
}

#[derive(Debug, Clone, Copy)]
struct PositionedFadePlan {
    output_frames: u64,
    fade_in: u64,
    fade_out_start: u64,
}

fn ratio(numerator: u64, denominator: u64) -> f32 {
    #[allow(
        clippy::cast_precision_loss,
        reason = "Fade coefficients are applied at the f32 sample boundary; exact integer precision above f32 mantissa range is not meaningful for audio buffers."
    )]
    {
        numerator as f32 / denominator as f32
    }
}

#[cfg(test)]
mod tests {
    use super::{Fade, FadeCurve};
    use crate::EffectError;
    use crate::test_support::{
        assert_sample_bits_eq, assert_samples_close, audio_buffer, stereo_audio_buffer,
    };
    use auralis_core::{ChannelCount, FrameCount};
    use auralis_simd::BackendKind;

    #[test]
    fn fade_envelope_coefficients_are_linear() {
        let mut audio = audio_buffer(vec![1.0; 6]);

        Fade::new(FrameCount::new(4), FrameCount::new(3)).process_buffer(&mut audio);

        assert_samples_close(
            audio.as_planar_f32(),
            &[0.0, 0.25, 0.5, 0.5, 1.0 / 3.0, 0.0],
        );
    }

    #[test]
    fn zero_length_fade_is_identity() {
        let source = stereo_audio_buffer(vec![-0.5, 0.0, 0.5, 0.25, -0.25, 1.0]);
        let mut actual = source.clone();

        Fade::new(FrameCount::new(0), FrameCount::new(0)).process_buffer(&mut actual);

        assert_eq!(actual, source);
    }

    #[test]
    fn fade_in_only_and_fade_out_only_apply_one_side() {
        let mut fade_in = audio_buffer(vec![1.0; 5]);
        let mut fade_out = audio_buffer(vec![1.0; 5]);

        Fade::new(FrameCount::new(4), FrameCount::new(0)).process_buffer(&mut fade_in);
        Fade::new(FrameCount::new(0), FrameCount::new(4)).process_buffer(&mut fade_out);

        assert_samples_close(fade_in.as_planar_f32(), &[0.0, 0.25, 0.5, 0.75, 1.0]);
        assert_samples_close(fade_out.as_planar_f32(), &[1.0, 0.75, 0.5, 0.25, 0.0]);
    }

    #[test]
    fn positioned_fade_out_uses_sox_ng_stop_endpoint_indexing() {
        let audio = audio_buffer(vec![1.0; 6]);

        let faded = Fade::with_stop_position(
            FadeCurve::Linear,
            FrameCount::new(0),
            FrameCount::new(0),
            FrameCount::new(4),
        )
        .process_buffer_to_output(&audio, BackendKind::Scalar)
        .unwrap();

        assert_eq!(faded.frames(), FrameCount::new(6));
        assert_samples_close(faded.as_planar_f32(), &[1.0, 1.0, 1.0, 0.75, 0.5, 0.25]);
    }

    #[test]
    fn positioned_fade_truncates_or_pads_to_stop_position() {
        let audio = stereo_audio_buffer(vec![1.0, 1.0, 1.0, 1.0, -1.0, -1.0, -1.0, -1.0]);

        let truncated = Fade::with_stop_position(
            FadeCurve::Linear,
            FrameCount::new(0),
            FrameCount::new(3),
            FrameCount::new(2),
        )
        .process_buffer_to_output(&audio, BackendKind::Scalar)
        .unwrap();
        let padded = Fade::with_stop_position(
            FadeCurve::Linear,
            FrameCount::new(0),
            FrameCount::new(6),
            FrameCount::new(2),
        )
        .process_buffer_to_output(&audio, BackendKind::Scalar)
        .unwrap();

        assert_eq!(truncated.frames(), FrameCount::new(3));
        assert_samples_close(
            truncated.as_planar_f32(),
            &[1.0, 1.0, 0.5, -1.0, -1.0, -0.5],
        );
        assert_eq!(padded.frames(), FrameCount::new(6));
        assert_samples_close(
            padded.as_planar_f32(),
            &[
                1.0, 1.0, 1.0, 1.0, 0.0, 0.0, -1.0, -1.0, -1.0, -1.0, 0.0, 0.0,
            ],
        );
    }

    #[test]
    fn positioned_fade_rejects_overlapping_fade_regions() {
        let audio = audio_buffer(vec![1.0; 8]);

        let error = Fade::with_stop_position(
            FadeCurve::Linear,
            FrameCount::new(6),
            FrameCount::new(8),
            FrameCount::new(4),
        )
        .process_buffer_to_output(&audio, BackendKind::Scalar)
        .unwrap_err();

        assert_eq!(error, EffectError::FadeRegionsOverlap);
    }

    #[test]
    fn sox_ng_fade_curves_match_reference_coefficients() {
        let curves = [
            (
                FadeCurve::QuarterSine,
                [
                    0.0,
                    (1.0_f32 * std::f32::consts::FRAC_PI_2 / 4.0).sin(),
                    (2.0_f32 * std::f32::consts::FRAC_PI_2 / 4.0).sin(),
                    (3.0_f32 * std::f32::consts::FRAC_PI_2 / 4.0).sin(),
                    1.0,
                ],
            ),
            (
                FadeCurve::HalfSine,
                [
                    0.0,
                    (1.0 - (1.0_f32 * std::f32::consts::PI / 4.0).cos()) / 2.0,
                    0.5,
                    (1.0 - (3.0_f32 * std::f32::consts::PI / 4.0).cos()) / 2.0,
                    1.0,
                ],
            ),
            (
                FadeCurve::Logarithmic,
                [
                    0.000_01,
                    0.000_177_827_94,
                    0.003_162_277_6,
                    0.056_234_132,
                    1.0,
                ],
            ),
            (FadeCurve::Linear, [0.0, 0.25, 0.5, 0.75, 1.0]),
            (
                FadeCurve::InvertedParabola,
                [0.0, 0.4375, 0.75, 0.9375, 1.0],
            ),
        ];

        for (curve, expected) in curves {
            let mut audio = audio_buffer(vec![1.0; 5]);

            Fade::with_curve(curve, FrameCount::new(4), FrameCount::new(0))
                .process_buffer(&mut audio);

            assert_samples_close(audio.as_planar_f32(), &expected);
        }
    }

    #[test]
    fn stereo_fade_preserves_channel_grouping() {
        let mut audio = stereo_audio_buffer(vec![1.0, 1.0, 1.0, 1.0, -1.0, -1.0, -1.0, -1.0]);

        Fade::new(FrameCount::new(2), FrameCount::new(2)).process_buffer(&mut audio);

        assert_eq!(audio.frames(), FrameCount::new(4));
        assert_eq!(audio.channels(), ChannelCount::new(2).unwrap());
        assert_samples_close(
            audio.as_planar_f32(),
            &[0.0, 0.5, 0.5, 0.0, -0.0, -0.5, -0.5, -0.0],
        );
    }

    #[test]
    fn fade_segment_processing_matches_whole_channel() {
        let source = vec![1.0; 9];
        let fade = Fade::new(FrameCount::new(4), FrameCount::new(4));
        let mut whole = audio_buffer(source.clone());
        let mut chunked = source;
        let mut start = 0_u64;

        fade.process_buffer(&mut whole);
        for chunk in chunked.chunks_mut(3) {
            fade.process_channel_segment(chunk, 9, FrameCount::new(start));
            start += u64::try_from(chunk.len()).unwrap();
        }

        assert_samples_close(whole.as_planar_f32(), &chunked);
    }

    #[test]
    fn fade_effect_matches_under_forced_scalar_and_requested_simd() {
        let source = stereo_audio_buffer(vec![
            -1.0,
            -0.999_984_74,
            -0.5,
            -0.0,
            0.0,
            0.5,
            0.999_984_74,
            1.0,
            1.0,
            0.999_984_74,
            0.5,
            0.0,
            -0.0,
            -0.5,
            -0.999_984_74,
            -1.0,
        ]);
        let fade = Fade::with_curve(FadeCurve::HalfSine, FrameCount::new(5), FrameCount::new(7));
        let mut scalar = source.clone();
        let mut simd = source;

        fade.process_buffer_with_backend(&mut scalar, BackendKind::Scalar);
        fade.process_buffer_with_backend(&mut simd, BackendKind::Simd);

        assert_sample_bits_eq(simd.as_planar_f32(), scalar.as_planar_f32());
    }
}

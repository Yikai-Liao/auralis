use std::f32::consts::{FRAC_PI_2, PI};

use auralis_core::{AudioBuffer, FrameCount};
use auralis_dsp::{fade_in_place, fade_in_place_with_backend};
use auralis_simd::{BackendKind, select_backend};

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

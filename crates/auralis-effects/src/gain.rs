use auralis_core::{AudioBuffer, Decibels};
use auralis_dsp::{gain_in_place, gain_in_place_with_backend};
use auralis_simd::{BackendKind, select_backend};

/// Gain effect processor.
///
/// `Gain` multiplies every sample by `10^(db / 20)` using the scalar
/// [`auralis_dsp::gain_in_place`] reference kernel by default. Explicit backend
/// methods can request SIMD through Auralis' backend selection layer while
/// preserving the same numerical behavior and scalar fallback rules.
///
/// Plain gain does not clip, normalize, allocate, or inspect channel
/// boundaries, so processing a whole buffer and processing the same samples in
/// chunks produce identical results. SoX-ng headroom options are represented by
/// [`GainHeadroom`]; direct sample processing still applies only the configured
/// fixed gain, while [`crate::EffectChain`] uses the headroom mode to implement
/// `gain -h` and `gain -r` command semantics.
///
/// # Examples
///
/// ```
/// use auralis_core::{
///     AudioBuffer, AudioSpec, ChannelCount, Decibels, FrameCount, SampleFormat, SampleRate,
/// };
/// use auralis_effects::Gain;
///
/// let spec = AudioSpec::new(
///     SampleRate::new(48_000)?,
///     ChannelCount::new(1)?,
///     SampleFormat::Float32,
/// );
/// let mut audio = AudioBuffer::from_planar_f32(
///     spec,
///     FrameCount::new(3),
///     vec![0.25, -0.5, 1.0],
/// )?;
///
/// Gain::new(Decibels::new(6.0)?).process_buffer(&mut audio);
///
/// assert!(audio.as_planar_f32()[0] > 0.49);
/// # Ok::<(), auralis_core::AuralisError>(())
/// ```
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Gain {
    /// Gain amount in decibels.
    pub db: Decibels,

    /// SoX-ng headroom/reclaim mode for command-chain execution.
    pub headroom: GainHeadroom,
}

/// SoX-ng `gain` headroom/reclaim mode.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum GainHeadroom {
    /// Plain fixed gain with no headroom metadata.
    None,

    /// `gain -h`: reserve the fixed gain as reclaimable headroom.
    Reserve,

    /// `gain -r`: reclaim prior headroom as much as possible without clipping.
    Reclaim,

    /// `gain -rh` or `gain -hr`: reclaim prior headroom and reserve new headroom.
    ReclaimAndReserve,
}

impl Gain {
    /// Creates a gain processor from a validated decibel value.
    #[must_use]
    pub const fn new(db: Decibels) -> Self {
        Self {
            db,
            headroom: GainHeadroom::None,
        }
    }

    /// Creates a `gain -h` processor that reserves fixed-gain headroom.
    #[must_use]
    pub const fn reserve_headroom(db: Decibels) -> Self {
        Self {
            db,
            headroom: GainHeadroom::Reserve,
        }
    }

    /// Creates a `gain -r` processor that reclaims previously reserved headroom.
    #[must_use]
    pub const fn reclaim_headroom(db: Decibels) -> Self {
        Self {
            db,
            headroom: GainHeadroom::Reclaim,
        }
    }

    /// Creates a `gain -rh` processor.
    #[must_use]
    pub const fn reclaim_and_reserve_headroom(db: Decibels) -> Self {
        Self {
            db,
            headroom: GainHeadroom::ReclaimAndReserve,
        }
    }

    /// Returns true when this command should reserve headroom metadata.
    #[must_use]
    pub const fn reserves_headroom(self) -> bool {
        matches!(
            self.headroom,
            GainHeadroom::Reserve | GainHeadroom::ReclaimAndReserve
        )
    }

    /// Returns true when this command should reclaim prior headroom metadata.
    #[must_use]
    pub const fn reclaims_headroom(self) -> bool {
        matches!(
            self.headroom,
            GainHeadroom::Reclaim | GainHeadroom::ReclaimAndReserve
        )
    }

    /// Applies gain to all samples in an audio buffer.
    pub fn process_buffer(self, audio: &mut AudioBuffer) {
        self.process_samples(audio.as_planar_f32_mut());
    }

    /// Applies gain to all samples in an audio buffer using the requested backend.
    ///
    /// Requesting [`BackendKind::Scalar`] forces the scalar reference path.
    /// Requesting [`BackendKind::Simd`] uses SIMD when the build and target
    /// support it, otherwise it follows the documented scalar fallback.
    pub fn process_buffer_with_backend(
        self,
        audio: &mut AudioBuffer,
        requested_backend: BackendKind,
    ) {
        self.process_samples_with_backend(audio.as_planar_f32_mut(), requested_backend);
    }

    /// Applies gain to a planar sample slice.
    ///
    /// This method is suitable for streaming or chunked processing because each
    /// sample is transformed independently.
    pub fn process_samples(self, samples: &mut [f32]) {
        gain_in_place(samples, self.db);
    }

    /// Applies gain to a planar sample slice using the requested backend.
    ///
    /// This method is suitable for streaming or chunked processing because each
    /// sample is transformed independently. Unsupported SIMD requests follow
    /// Auralis backend fallback metadata before processing continues.
    pub fn process_samples_with_backend(self, samples: &mut [f32], requested_backend: BackendKind) {
        gain_in_place_with_backend(select_backend(requested_backend), samples, self.db);
    }
}

#[cfg(test)]
mod tests {
    use super::Gain;
    use crate::test_support::{assert_sample_bits_eq, assert_samples_close, audio_buffer, db};
    use auralis_dsp::gain_in_place;
    use auralis_simd::BackendKind;

    #[test]
    fn gain_effect_output_matches_scalar_kernel() {
        let mut actual = audio_buffer(vec![-1.0, -0.25, 0.0, 0.5, 1.0]);
        let mut expected = actual.as_planar_f32().to_vec();
        let db = db(-6.0);

        Gain::new(db).process_buffer(&mut actual);
        gain_in_place(&mut expected, db);

        assert_samples_close(actual.as_planar_f32(), &expected);
    }

    #[test]
    fn whole_buffer_and_chunked_processing_match() {
        let db = db(6.0);
        let source = vec![-1.0, -0.75, -0.5, -0.25, 0.0, 0.25, 0.5, 0.75, 1.0];
        let mut whole = audio_buffer(source.clone());
        let mut chunked = source;

        Gain::new(db).process_buffer(&mut whole);
        for chunk in chunked.chunks_mut(4) {
            Gain::new(db).process_samples(chunk);
        }

        assert_samples_close(whole.as_planar_f32(), &chunked);
    }

    #[test]
    fn gain_effect_matches_under_forced_scalar_and_requested_simd() {
        let source = vec![-1.0, -0.999_984_74, -0.5, -0.0, 0.0, 0.5, 0.999_984_74, 1.0];
        let mut scalar = audio_buffer(source.clone());
        let mut simd = audio_buffer(source);
        let gain = Gain::new(db(-3.0));

        gain.process_buffer_with_backend(&mut scalar, BackendKind::Scalar);
        gain.process_buffer_with_backend(&mut simd, BackendKind::Simd);

        assert_sample_bits_eq(simd.as_planar_f32(), scalar.as_planar_f32());
    }

    #[test]
    fn empty_buffer_is_accepted() {
        let mut audio = audio_buffer(Vec::new());

        Gain::new(db(12.0)).process_buffer(&mut audio);

        assert!(audio.as_planar_f32().is_empty());
    }

    #[test]
    fn headroom_constructors_preserve_fixed_gain_processing() {
        let source = vec![0.25, -0.5, 1.0];
        let mut plain = audio_buffer(source.clone());
        let mut headroom = audio_buffer(source);

        Gain::new(db(-6.0)).process_buffer(&mut plain);
        Gain::reserve_headroom(db(-6.0)).process_buffer(&mut headroom);

        assert!(Gain::reserve_headroom(db(-6.0)).reserves_headroom());
        assert!(Gain::reclaim_headroom(db(0.0)).reclaims_headroom());
        assert_samples_close(plain.as_planar_f32(), headroom.as_planar_f32());
    }
}

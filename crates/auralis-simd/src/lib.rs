//! SIMD backends for Auralis.
//!
//! This crate owns the backend trait skeleton used by future sample-processing
//! kernels. The scalar backend is the deterministic reference implementation;
//! optimized backends implement the same Auralis-owned traits while keeping
//! implementation crates such as `rten-simd` out of public API types.
//!
//! # Examples
//!
//! ```
//! use auralis_simd::{Backend, BackendKind, ScalarBackend};
//!
//! let descriptor = ScalarBackend::descriptor();
//!
//! assert_eq!(descriptor.kind(), BackendKind::Scalar);
//! assert_eq!(descriptor.name(), "scalar");
//! assert!(descriptor.is_available());
//! ```

#![deny(unsafe_code)]

use core::fmt;

#[cfg(feature = "simd")]
use rten_simd as _;

/// Stable identifier for an Auralis sample-processing backend.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum BackendKind {
    /// Deterministic scalar reference backend.
    Scalar,

    /// Optimized SIMD backend compiled behind the `simd` feature.
    Simd,
}

impl BackendKind {
    /// Returns the stable lowercase backend name used in reports and tests.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Scalar => "scalar",
            Self::Simd => "simd",
        }
    }

    /// Returns the backend kind for a stable lowercase backend name.
    ///
    /// The accepted names are `scalar` and `simd`. Backend name parsing is
    /// intentionally case-sensitive so manifests, reports, and test fixtures
    /// render the same spelling on every platform.
    #[must_use]
    pub fn from_name(name: &str) -> Option<Self> {
        match name {
            "scalar" => Some(Self::Scalar),
            "simd" => Some(Self::Simd),
            _ => None,
        }
    }
}

impl fmt::Display for BackendKind {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.as_str())
    }
}

/// Public metadata for an Auralis backend implementation.
///
/// Descriptors intentionally expose only Auralis-owned enums and primitive
/// values. Concrete implementation crate types remain private to the backend
/// implementation modules.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BackendDescriptor {
    kind: BackendKind,
    name: &'static str,
    available: bool,
}

impl BackendDescriptor {
    /// Creates a backend descriptor from Auralis-owned metadata.
    #[must_use]
    pub const fn new(kind: BackendKind, name: &'static str, available: bool) -> Self {
        Self {
            kind,
            name,
            available,
        }
    }

    /// Returns the backend kind.
    #[must_use]
    pub const fn kind(self) -> BackendKind {
        self.kind
    }

    /// Returns the stable backend name.
    #[must_use]
    pub const fn name(self) -> &'static str {
        self.name
    }

    /// Returns whether this backend is available in the active build.
    #[must_use]
    pub const fn is_available(self) -> bool {
        self.available
    }
}

/// Reason a requested backend was not selected.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum BackendFallbackReason {
    /// The `simd` Cargo feature was not enabled, so SIMD backend code is absent.
    SimdFeatureDisabled,

    /// The active target does not support Auralis' SIMD backend.
    SimdTargetUnsupported,
}

impl BackendFallbackReason {
    /// Returns a stable explanation for reports and tests.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::SimdFeatureDisabled => "simd feature is disabled",
            Self::SimdTargetUnsupported => "simd backend is unsupported on this target",
        }
    }
}

impl fmt::Display for BackendFallbackReason {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.as_str())
    }
}

/// Deterministic result of resolving a requested backend.
///
/// Selection never changes high-level effect APIs: callers receive Auralis-owned
/// backend metadata, and concrete backend implementation crates remain private
/// implementation details.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BackendSelection {
    requested: BackendKind,
    selected: BackendDescriptor,
    fallback_reason: Option<BackendFallbackReason>,
}

impl BackendSelection {
    const fn new(
        requested: BackendKind,
        selected: BackendDescriptor,
        fallback_reason: Option<BackendFallbackReason>,
    ) -> Self {
        Self {
            requested,
            selected,
            fallback_reason,
        }
    }

    /// Returns the backend kind requested by the caller or test harness.
    #[must_use]
    pub const fn requested_kind(self) -> BackendKind {
        self.requested
    }

    /// Returns the backend descriptor selected for execution.
    #[must_use]
    pub const fn selected_descriptor(self) -> BackendDescriptor {
        self.selected
    }

    /// Returns the selected backend kind.
    #[must_use]
    pub const fn selected_kind(self) -> BackendKind {
        self.selected.kind()
    }

    /// Returns the selected backend name.
    #[must_use]
    pub const fn selected_name(self) -> &'static str {
        self.selected.name()
    }

    /// Returns why the requested backend could not be used.
    #[must_use]
    pub const fn fallback_reason(self) -> Option<BackendFallbackReason> {
        self.fallback_reason
    }

    /// Returns whether backend selection fell back to a different backend.
    #[must_use]
    pub const fn is_fallback(self) -> bool {
        self.fallback_reason.is_some()
    }
}

/// Errors produced by sample conversion kernels.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum SampleConversionError {
    /// The source and destination buffers did not have the same length.
    BufferLengthMismatch {
        /// Number of input samples.
        input_len: usize,
        /// Number of output samples.
        output_len: usize,
    },

    /// A floating-point input sample was NaN or infinite.
    NonFiniteSample {
        /// Zero-based index of the first non-finite input sample.
        sample_index: usize,
    },
}

impl fmt::Display for SampleConversionError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::BufferLengthMismatch {
                input_len,
                output_len,
            } => write!(
                formatter,
                "sample conversion input length {input_len} did not match output length {output_len}"
            ),
            Self::NonFiniteSample { sample_index } => write!(
                formatter,
                "sample conversion input sample {sample_index} was NaN or infinite"
            ),
        }
    }
}

impl std::error::Error for SampleConversionError {}

/// Compile-time backend contract for sample-processing kernels.
///
/// The trait uses associated constants so kernel dispatch can stay statically
/// typed while deterministic runtime selection is added later. It is sealed so
/// downstream code cannot implement incompatible backend markers.
pub trait Backend: private::Sealed + Copy + fmt::Debug + Default + Send + Sync + 'static {
    /// Stable backend kind.
    const KIND: BackendKind;

    /// Stable lowercase backend name.
    const NAME: &'static str;

    /// Whether this backend can be used in the active build.
    const AVAILABLE: bool;

    /// Returns public metadata for this backend type.
    #[must_use]
    fn descriptor() -> BackendDescriptor {
        BackendDescriptor::new(Self::KIND, Self::NAME, Self::AVAILABLE)
    }
}

/// Deterministic scalar reference backend.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub struct ScalarBackend;

impl Backend for ScalarBackend {
    const KIND: BackendKind = BackendKind::Scalar;
    const NAME: &'static str = "scalar";
    const AVAILABLE: bool = true;
}

/// Placeholder optimized backend compiled when the optional `simd` feature is enabled.
///
/// This marker does not implement any kernels yet. It proves that the selected
/// SIMD dependency can be built behind an Auralis-owned backend boundary before
/// later features add concrete conversion and effect kernels.
#[cfg(feature = "simd")]
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub struct SimdBackend;

#[cfg(feature = "simd")]
impl Backend for SimdBackend {
    const KIND: BackendKind = BackendKind::Simd;
    const NAME: &'static str = "simd";
    const AVAILABLE: bool = simd_target_supported();
}

/// Returns public metadata for a backend kind in the active build.
#[must_use]
pub const fn backend_descriptor(kind: BackendKind) -> BackendDescriptor {
    match kind {
        BackendKind::Scalar => scalar_descriptor(),
        BackendKind::Simd => simd_descriptor(),
    }
}

/// Selects a backend by kind with deterministic fallback behavior.
///
/// Requesting [`BackendKind::Scalar`] always selects the scalar reference
/// backend. Requesting [`BackendKind::Simd`] selects SIMD only when the `simd`
/// feature is enabled and the active target is supported; otherwise selection
/// falls back to scalar and records a [`BackendFallbackReason`].
#[must_use]
pub const fn select_backend(requested: BackendKind) -> BackendSelection {
    select_backend_with_status(requested, simd_status())
}

/// Selects a backend by stable lowercase name.
///
/// Returns [`None`] when `name` is not one of the supported backend names:
/// `scalar` or `simd`.
#[must_use]
pub fn select_named_backend(name: &str) -> Option<BackendSelection> {
    BackendKind::from_name(name).map(select_backend)
}

/// Converts signed 16-bit PCM samples into normalized `f32` samples using the
/// scalar reference backend.
///
/// Samples are scaled by `1.0 / 32768.0`, so `i16::MIN` maps exactly to
/// `-1.0`, `0` maps to `0.0`, and `i16::MAX` maps to `0.9999695`. The
/// conversion is deterministic and exact for every PCM16 input because the
/// scale denominator is a power of two. The function allocates no memory and
/// accepts empty buffers.
///
/// # Errors
///
/// Returns [`SampleConversionError::BufferLengthMismatch`] when `input` and
/// `output` have different lengths.
///
/// # Examples
///
/// ```
/// let input = [i16::MIN, 0, i16::MAX];
/// let mut output = [0.0; 3];
///
/// auralis_simd::i16_to_f32_scalar(&input, &mut output)?;
///
/// assert_eq!(output[0].to_bits(), (-1.0_f32).to_bits());
/// assert_eq!(output[1].to_bits(), 0.0_f32.to_bits());
/// assert_eq!(output[2].to_bits(), (f32::from(i16::MAX) / 32768.0).to_bits());
/// # Ok::<(), auralis_simd::SampleConversionError>(())
/// ```
pub fn i16_to_f32_scalar(input: &[i16], output: &mut [f32]) -> Result<(), SampleConversionError> {
    validate_conversion_lengths(input.len(), output.len())?;
    i16_to_f32_scalar_unchecked(input, output);
    Ok(())
}

/// Converts signed 16-bit PCM samples into normalized `f32` samples using the
/// backend recorded by `selection`.
///
/// Callers that need deterministic tests can pass the result of
/// [`select_backend`] with either [`BackendKind::Scalar`] or
/// [`BackendKind::Simd`]. When a SIMD request falls back to scalar, this
/// function follows the selected backend recorded in the selection metadata.
/// The numerical mapping is identical to [`i16_to_f32_scalar`].
///
/// # Errors
///
/// Returns [`SampleConversionError::BufferLengthMismatch`] when `input` and
/// `output` have different lengths.
pub fn i16_to_f32_with_backend(
    selection: BackendSelection,
    input: &[i16],
    output: &mut [f32],
) -> Result<(), SampleConversionError> {
    validate_conversion_lengths(input.len(), output.len())?;

    match selection.selected_kind() {
        BackendKind::Scalar => i16_to_f32_scalar_unchecked(input, output),
        BackendKind::Simd => i16_to_f32_selected_simd(input, output),
    }

    Ok(())
}

/// Converts normalized `f32` samples into signed 16-bit PCM using the scalar
/// reference backend.
///
/// Samples must be finite. Each sample is clipped to `[-1.0, 1.0]`, scaled by
/// `32768.0`, rounded to the nearest integer with halfway cases rounded away
/// from zero, then clipped to the `i16` range. This maps `-1.0` to
/// [`i16::MIN`], `1.0` to [`i16::MAX`], and values that came from
/// [`i16_to_f32_scalar`] back to their original PCM16 value where possible.
/// The function allocates no memory and accepts empty buffers.
///
/// # Errors
///
/// Returns [`SampleConversionError::BufferLengthMismatch`] when `input` and
/// `output` have different lengths. Returns
/// [`SampleConversionError::NonFiniteSample`] for the first NaN or infinite
/// input sample.
///
/// # Examples
///
/// ```
/// let input = [-1.0, 0.0, 0.5, 1.0];
/// let mut output = [0; 4];
///
/// auralis_simd::f32_to_i16_scalar(&input, &mut output)?;
///
/// assert_eq!(output, [i16::MIN, 0, 16_384, i16::MAX]);
/// # Ok::<(), auralis_simd::SampleConversionError>(())
/// ```
pub fn f32_to_i16_scalar(input: &[f32], output: &mut [i16]) -> Result<(), SampleConversionError> {
    validate_conversion_lengths(input.len(), output.len())?;
    validate_finite_samples(input)?;
    f32_to_i16_scalar_unchecked(input, output);
    Ok(())
}

/// Converts normalized `f32` samples into signed 16-bit PCM using the backend
/// recorded by `selection`.
///
/// Callers that need deterministic tests can pass the result of
/// [`select_backend`] with either [`BackendKind::Scalar`] or
/// [`BackendKind::Simd`]. When a SIMD request falls back to scalar, this
/// function follows the selected backend recorded in the selection metadata.
/// The clipping, rounding, and NaN/infinity behavior are identical to
/// [`f32_to_i16_scalar`].
///
/// # Errors
///
/// Returns [`SampleConversionError::BufferLengthMismatch`] when `input` and
/// `output` have different lengths. Returns
/// [`SampleConversionError::NonFiniteSample`] for the first NaN or infinite
/// input sample.
pub fn f32_to_i16_with_backend(
    selection: BackendSelection,
    input: &[f32],
    output: &mut [i16],
) -> Result<(), SampleConversionError> {
    validate_conversion_lengths(input.len(), output.len())?;
    validate_finite_samples(input)?;

    match selection.selected_kind() {
        BackendKind::Scalar => f32_to_i16_scalar_unchecked(input, output),
        BackendKind::Simd => f32_to_i16_selected_simd(input, output),
    }

    Ok(())
}

const fn scalar_descriptor() -> BackendDescriptor {
    BackendDescriptor::new(BackendKind::Scalar, BackendKind::Scalar.as_str(), true)
}

const fn simd_descriptor() -> BackendDescriptor {
    BackendDescriptor::new(
        BackendKind::Simd,
        BackendKind::Simd.as_str(),
        matches!(simd_status(), SimdStatus::Available),
    )
}

const fn select_backend_with_status(
    requested: BackendKind,
    simd_status: SimdStatus,
) -> BackendSelection {
    match requested {
        BackendKind::Scalar => {
            BackendSelection::new(BackendKind::Scalar, scalar_descriptor(), None)
        }
        BackendKind::Simd => match simd_status {
            SimdStatus::Available => {
                BackendSelection::new(BackendKind::Simd, simd_descriptor(), None)
            }
            SimdStatus::FeatureDisabled => BackendSelection::new(
                BackendKind::Simd,
                scalar_descriptor(),
                Some(BackendFallbackReason::SimdFeatureDisabled),
            ),
            SimdStatus::TargetUnsupported => BackendSelection::new(
                BackendKind::Simd,
                scalar_descriptor(),
                Some(BackendFallbackReason::SimdTargetUnsupported),
            ),
        },
    }
}

const fn simd_status() -> SimdStatus {
    if !cfg!(feature = "simd") {
        SimdStatus::FeatureDisabled
    } else if !simd_target_supported() {
        SimdStatus::TargetUnsupported
    } else {
        SimdStatus::Available
    }
}

const fn simd_target_supported() -> bool {
    cfg!(any(
        target_arch = "x86_64",
        target_arch = "aarch64",
        all(target_arch = "wasm32", target_feature = "simd128")
    ))
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum SimdStatus {
    Available,
    FeatureDisabled,
    TargetUnsupported,
}

const PCM16_TO_F32_SCALE: f32 = 1.0 / 32768.0;
const F32_TO_PCM16_SCALE: f32 = 32768.0;

fn validate_conversion_lengths(
    input_len: usize,
    output_len: usize,
) -> Result<(), SampleConversionError> {
    if input_len == output_len {
        Ok(())
    } else {
        Err(SampleConversionError::BufferLengthMismatch {
            input_len,
            output_len,
        })
    }
}

fn validate_finite_samples(input: &[f32]) -> Result<(), SampleConversionError> {
    for (sample_index, sample) in input.iter().enumerate() {
        if !sample.is_finite() {
            return Err(SampleConversionError::NonFiniteSample { sample_index });
        }
    }

    Ok(())
}

#[inline]
fn i16_to_f32_scalar_unchecked(input: &[i16], output: &mut [f32]) {
    for (&input, output) in input.iter().zip(output) {
        *output = f32::from(input) * PCM16_TO_F32_SCALE;
    }
}

#[inline]
fn f32_to_i16_scalar_unchecked(input: &[f32], output: &mut [i16]) {
    for (&input, output) in input.iter().zip(output) {
        *output = f32_to_i16_scalar_sample(input);
    }
}

#[allow(
    clippy::cast_possible_truncation,
    reason = "the sample is rounded and clamped to the i16 range before casting"
)]
#[inline]
fn f32_to_i16_scalar_sample(sample: f32) -> i16 {
    let scaled = (sample.clamp(-1.0, 1.0) * F32_TO_PCM16_SCALE)
        .round()
        .clamp(f32::from(i16::MIN), f32::from(i16::MAX));

    scaled as i16
}

#[cfg(not(feature = "simd"))]
fn i16_to_f32_selected_simd(input: &[i16], output: &mut [f32]) {
    i16_to_f32_scalar_unchecked(input, output);
}

#[cfg(not(feature = "simd"))]
fn f32_to_i16_selected_simd(input: &[f32], output: &mut [i16]) {
    f32_to_i16_scalar_unchecked(input, output);
}

#[cfg(feature = "simd")]
fn i16_to_f32_selected_simd(input: &[i16], output: &mut [f32]) {
    use rten_simd::{
        Isa, SimdOp,
        ops::{Extend, NumOps, ToFloat},
    };

    struct Convert<'input, 'output> {
        input: &'input [i16],
        output: &'output mut [f32],
    }

    impl SimdOp for Convert<'_, '_> {
        type Output = ();

        #[expect(
            clippy::inline_always,
            reason = "rten-simd recommends inlining eval so target-feature intrinsics compile into the dispatched kernel body"
        )]
        #[inline(always)]
        fn eval<I: Isa>(self, isa: I) -> Self::Output {
            let i16_ops = isa.i16();
            let i32_ops = isa.i32();
            let f32_ops = isa.f32();
            let scale = f32_ops.splat(PCM16_TO_F32_SCALE);
            let input_vector_len = i16_ops.len();
            let output_half_vector_len = i32_ops.len();

            let mut input_chunks = self.input.chunks_exact(input_vector_len);
            let mut output_chunks = self.output.chunks_exact_mut(input_vector_len);

            for (input_chunk, output_chunk) in input_chunks.by_ref().zip(output_chunks.by_ref()) {
                let input_i16 = i16_ops.load(input_chunk);
                let (low_extended, high_extended) = i16_ops.extend(input_i16);
                let low_scaled = f32_ops.mul(i32_ops.to_float(low_extended), scale);
                let high_scaled = f32_ops.mul(i32_ops.to_float(high_extended), scale);
                let (low_output, high_output) = output_chunk.split_at_mut(output_half_vector_len);

                f32_ops.store(low_scaled, low_output);
                f32_ops.store(high_scaled, high_output);
            }

            i16_to_f32_scalar_unchecked(input_chunks.remainder(), output_chunks.into_remainder());
        }
    }

    Convert { input, output }.dispatch();
}

#[cfg(feature = "simd")]
fn f32_to_i16_selected_simd(input: &[f32], output: &mut [i16]) {
    use rten_simd::{
        Isa, SimdOp,
        ops::{FloatOps, NarrowSaturate, NumOps},
    };

    struct Convert<'input, 'output> {
        input: &'input [f32],
        output: &'output mut [i16],
    }

    impl SimdOp for Convert<'_, '_> {
        type Output = ();

        #[expect(
            clippy::inline_always,
            reason = "rten-simd recommends inlining eval so target-feature intrinsics compile into the dispatched kernel body"
        )]
        #[inline(always)]
        fn eval<I: Isa>(self, isa: I) -> Self::Output {
            let f32_ops = isa.f32();
            let i32_ops = isa.i32();
            let i16_ops = isa.i16();
            let f32_vector_len = f32_ops.len();
            let output_vector_len = i16_ops.len();
            let min_sample = f32_ops.splat(-1.0);
            let max_sample = f32_ops.splat(1.0);
            let scale = f32_ops.splat(F32_TO_PCM16_SCALE);
            let zero = f32_ops.zero();
            let positive_round_offset = f32_ops.splat(0.5);
            let negative_round_offset = f32_ops.splat(-0.5);

            let mut input_chunks = self.input.chunks_exact(output_vector_len);
            let mut output_chunks = self.output.chunks_exact_mut(output_vector_len);

            for (input_chunk, output_chunk) in input_chunks.by_ref().zip(output_chunks.by_ref()) {
                let (low_input, high_input) = input_chunk.split_at(f32_vector_len);
                let low_scaled = f32_ops.mul(
                    f32_ops.clamp(f32_ops.load(low_input), min_sample, max_sample),
                    scale,
                );
                let high_scaled = f32_ops.mul(
                    f32_ops.clamp(f32_ops.load(high_input), min_sample, max_sample),
                    scale,
                );
                let low_round_offset = f32_ops.select(
                    positive_round_offset,
                    negative_round_offset,
                    f32_ops.ge(low_scaled, zero),
                );
                let high_round_offset = f32_ops.select(
                    positive_round_offset,
                    negative_round_offset,
                    f32_ops.ge(high_scaled, zero),
                );
                let low_i32 = f32_ops.to_int_trunc(f32_ops.add(low_scaled, low_round_offset));
                let high_i32 = f32_ops.to_int_trunc(f32_ops.add(high_scaled, high_round_offset));
                let packed = i32_ops.narrow_saturate(low_i32, high_i32);

                i16_ops.store(packed, output_chunk);
            }

            f32_to_i16_scalar_unchecked(input_chunks.remainder(), output_chunks.into_remainder());
        }
    }

    Convert { input, output }.dispatch();
}

mod private {
    use super::ScalarBackend;

    pub trait Sealed {}

    impl Sealed for ScalarBackend {}

    #[cfg(feature = "simd")]
    impl Sealed for super::SimdBackend {}
}

#[cfg(test)]
mod tests {
    use core::any::type_name;

    use super::{
        Backend, BackendDescriptor, BackendFallbackReason, BackendKind, BackendSelection,
        SampleConversionError, ScalarBackend, SimdStatus, backend_descriptor, f32_to_i16_scalar,
        f32_to_i16_with_backend, i16_to_f32_scalar, i16_to_f32_with_backend, select_backend,
        select_backend_with_status, select_named_backend,
    };

    #[test]
    fn crate_is_linkable() {
        assert_eq!(env!("CARGO_PKG_NAME"), "auralis-simd");
    }

    #[test]
    fn scalar_backend_implements_backend_trait() {
        let descriptor = descriptor_for::<ScalarBackend>();

        assert_eq!(descriptor.kind(), BackendKind::Scalar);
        assert_eq!(descriptor.name(), "scalar");
        assert!(descriptor.is_available());
        assert_eq!(BackendKind::Scalar.to_string(), "scalar");
    }

    #[test]
    fn backend_names_are_stable_and_case_sensitive() {
        assert_eq!(BackendKind::from_name("scalar"), Some(BackendKind::Scalar));
        assert_eq!(BackendKind::from_name("simd"), Some(BackendKind::Simd));
        assert_eq!(BackendKind::from_name("SIMD"), None);
        assert_eq!(BackendKind::from_name(""), None);
    }

    #[test]
    fn scalar_backend_can_be_forced_by_kind_and_name() {
        let by_kind = select_backend(BackendKind::Scalar);
        let by_name = select_named_backend("scalar").expect("scalar backend name should resolve");

        assert_eq!(by_kind, by_name);
        assert_eq!(by_kind.requested_kind(), BackendKind::Scalar);
        assert_eq!(by_kind.selected_kind(), BackendKind::Scalar);
        assert_eq!(by_kind.selected_name(), "scalar");
        assert_eq!(by_kind.selected_descriptor(), scalar_descriptor());
        assert_eq!(
            backend_descriptor(BackendKind::Scalar),
            by_kind.selected_descriptor()
        );
        assert!(!by_kind.is_fallback());
        assert_eq!(by_kind.fallback_reason(), None);
    }

    #[test]
    fn simd_request_falls_back_when_feature_is_disabled() {
        let selection = select_backend_with_status(BackendKind::Simd, SimdStatus::FeatureDisabled);

        assert_eq!(selection.requested_kind(), BackendKind::Simd);
        assert_eq!(selection.selected_kind(), BackendKind::Scalar);
        assert_eq!(selection.selected_name(), "scalar");
        assert!(selection.is_fallback());
        assert_eq!(
            selection.fallback_reason(),
            Some(BackendFallbackReason::SimdFeatureDisabled)
        );
        assert_eq!(
            selection
                .fallback_reason()
                .expect("fallback reason should be recorded")
                .to_string(),
            "simd feature is disabled"
        );
    }

    #[test]
    fn simd_request_falls_back_on_unsupported_target() {
        let selection =
            select_backend_with_status(BackendKind::Simd, SimdStatus::TargetUnsupported);

        assert_eq!(selection.requested_kind(), BackendKind::Simd);
        assert_eq!(selection.selected_kind(), BackendKind::Scalar);
        assert!(selection.is_fallback());
        assert_eq!(
            selection.fallback_reason(),
            Some(BackendFallbackReason::SimdTargetUnsupported)
        );
        assert_eq!(
            selection
                .fallback_reason()
                .expect("fallback reason should be recorded")
                .as_str(),
            "simd backend is unsupported on this target"
        );
    }

    #[test]
    fn unknown_backend_names_are_rejected() {
        assert_eq!(select_named_backend("avx2"), None);
        assert_eq!(select_named_backend(""), None);
    }

    #[test]
    fn scalar_i16_to_f32_matches_known_pcm16_values_exactly() {
        let input = [i16::MIN, -16_384, -1, 0, 1, 16_384, i16::MAX];
        let mut output = [0.0; 7];

        i16_to_f32_scalar(&input, &mut output).unwrap();

        assert_sample_bits_eq(
            &output,
            &[
                -1.0,
                -0.5,
                -1.0 / 32768.0,
                0.0,
                1.0 / 32768.0,
                0.5,
                f32::from(i16::MAX) / 32768.0,
            ],
        );
    }

    #[test]
    fn i16_to_f32_rejects_mismatched_buffer_lengths() {
        let input = [0, 1, 2];
        let mut output = [0.0; 2];

        let error = i16_to_f32_scalar(&input, &mut output).unwrap_err();

        assert_eq!(
            error,
            SampleConversionError::BufferLengthMismatch {
                input_len: 3,
                output_len: 2,
            }
        );
        assert_eq!(
            error.to_string(),
            "sample conversion input length 3 did not match output length 2"
        );
    }

    #[test]
    fn i16_to_f32_handles_empty_one_sample_odd_and_tail_lengths() {
        for len in [0, 1, 3, 17, 33, 65] {
            let input = patterned_pcm16(len);

            assert_scalar_and_simd_conversion_match(&input);
        }
    }

    #[test]
    fn i16_to_f32_random_pcm16_values_match_scalar_under_requested_simd() {
        let mut input = seeded_pcm16(0x9e37_79b9_7f4a_7c15, 4099);
        input.extend([i16::MIN, i16::MAX, -1, 0, 1]);

        assert_scalar_and_simd_conversion_match(&input);
    }

    #[test]
    fn scalar_f32_to_i16_matches_known_values_exactly() {
        let input = [
            -1.5,
            -1.0,
            -0.5,
            -1.0 / 32768.0,
            -0.5 / 32768.0,
            0.0,
            0.5 / 32768.0,
            1.0 / 32768.0,
            0.5,
            f32::from(i16::MAX) / 32768.0,
            1.0,
            1.5,
        ];
        let mut output = [0; 12];

        f32_to_i16_scalar(&input, &mut output).unwrap();

        assert_eq!(
            output,
            [
                i16::MIN,
                i16::MIN,
                -16_384,
                -1,
                -1,
                0,
                1,
                1,
                16_384,
                i16::MAX,
                i16::MAX,
                i16::MAX,
            ]
        );
    }

    #[test]
    fn f32_to_i16_rejects_mismatched_buffer_lengths() {
        let input = [0.0, 0.25, 0.5];
        let mut output = [0; 2];

        let error = f32_to_i16_scalar(&input, &mut output).unwrap_err();

        assert_eq!(
            error,
            SampleConversionError::BufferLengthMismatch {
                input_len: 3,
                output_len: 2,
            }
        );
    }

    #[test]
    fn f32_to_i16_rejects_nan_and_infinity() {
        let input = [0.0, f32::INFINITY, f32::NAN, f32::NEG_INFINITY];
        let mut output = [0; 4];

        let error = f32_to_i16_scalar(&input, &mut output).unwrap_err();

        assert_eq!(
            error,
            SampleConversionError::NonFiniteSample { sample_index: 1 }
        );
        assert_eq!(
            error.to_string(),
            "sample conversion input sample 1 was NaN or infinite"
        );
    }

    #[test]
    fn f32_to_i16_handles_empty_one_sample_odd_and_tail_lengths() {
        for len in [0, 1, 3, 17, 33, 65] {
            let input = patterned_f32(len);

            assert_scalar_and_simd_f32_to_i16_match(&input);
        }
    }

    #[test]
    fn f32_to_i16_random_near_clipping_values_match_scalar_under_requested_simd() {
        let mut input = seeded_f32(0xd1b5_4a32_d192_ed03, 4099);
        input.extend([
            -1.5,
            -1.0,
            -0.999_984_74,
            -1.0 / 65536.0,
            1.0 / 65536.0,
            0.999_984_74,
            1.0,
            1.5,
        ]);

        assert_scalar_and_simd_f32_to_i16_match(&input);
    }

    #[cfg(all(
        feature = "simd",
        any(
            target_arch = "x86_64",
            target_arch = "aarch64",
            all(target_arch = "wasm32", target_feature = "simd128")
        )
    ))]
    #[test]
    fn simd_backend_can_be_forced_when_available() {
        let selection = select_named_backend("simd").expect("simd backend name should resolve");

        assert_eq!(selection.requested_kind(), BackendKind::Simd);
        assert_eq!(selection.selected_kind(), BackendKind::Simd);
        assert_eq!(selection.selected_name(), "simd");
        assert!(!selection.is_fallback());
        assert_eq!(selection.fallback_reason(), None);
        assert!(selection.selected_descriptor().is_available());
        assert_eq!(
            backend_descriptor(BackendKind::Simd),
            selection.selected_descriptor()
        );
    }

    #[test]
    fn public_backend_types_are_auralis_owned() {
        assert_public_type_name::<BackendDescriptor>();
        assert_public_type_name::<BackendFallbackReason>();
        assert_public_type_name::<BackendKind>();
        assert_public_type_name::<BackendSelection>();
        assert_public_type_name::<SampleConversionError>();
        assert_public_type_name::<ScalarBackend>();

        #[cfg(feature = "simd")]
        assert_public_type_name::<super::SimdBackend>();
    }

    #[cfg(feature = "simd")]
    #[test]
    fn placeholder_simd_backend_compiles_with_selected_crate() {
        let descriptor = descriptor_for::<super::SimdBackend>();

        assert_eq!(descriptor.kind(), BackendKind::Simd);
        assert_eq!(descriptor.name(), "simd");
        assert_eq!(descriptor.is_available(), super::simd_target_supported());
        assert_eq!(BackendKind::Simd.to_string(), "simd");
    }

    fn scalar_descriptor() -> BackendDescriptor {
        ScalarBackend::descriptor()
    }

    fn descriptor_for<B>() -> BackendDescriptor
    where
        B: Backend,
    {
        B::descriptor()
    }

    fn assert_public_type_name<T>() {
        let name = type_name::<T>();

        assert!(
            name.starts_with("auralis_simd::"),
            "public backend type {name} should be owned by auralis-simd"
        );
        assert!(
            !name.contains("rten"),
            "public backend type {name} must not expose rten-simd"
        );
    }

    fn assert_scalar_and_simd_conversion_match(input: &[i16]) {
        let scalar = convert_with_backend(BackendKind::Scalar, input);
        let simd = convert_with_backend(BackendKind::Simd, input);

        assert_sample_bits_eq(&scalar, &reference_conversion(input));
        assert_sample_bits_eq(&simd, &scalar);
    }

    fn assert_scalar_and_simd_f32_to_i16_match(input: &[f32]) {
        let scalar = convert_f32_to_i16_with_backend(BackendKind::Scalar, input);
        let simd = convert_f32_to_i16_with_backend(BackendKind::Simd, input);

        assert_eq!(scalar, reference_f32_to_i16(input));
        assert_eq!(simd, scalar);
    }

    fn convert_with_backend(kind: BackendKind, input: &[i16]) -> Vec<f32> {
        let selection = select_backend(kind);
        let mut output = vec![0.0; input.len()];

        i16_to_f32_with_backend(selection, input, &mut output).unwrap();

        output
    }

    fn convert_f32_to_i16_with_backend(kind: BackendKind, input: &[f32]) -> Vec<i16> {
        let selection = select_backend(kind);
        let mut output = vec![0; input.len()];

        f32_to_i16_with_backend(selection, input, &mut output).unwrap();

        output
    }

    fn reference_conversion(input: &[i16]) -> Vec<f32> {
        input
            .iter()
            .map(|&sample| f32::from(sample) / 32768.0)
            .collect()
    }

    fn reference_f32_to_i16(input: &[f32]) -> Vec<i16> {
        input
            .iter()
            .map(|&sample| reference_f32_sample(sample))
            .collect()
    }

    #[allow(
        clippy::cast_possible_truncation,
        reason = "the reference sample is rounded and clamped to the i16 range before casting"
    )]
    fn reference_f32_sample(sample: f32) -> i16 {
        let scaled = (sample.clamp(-1.0, 1.0) * 32768.0)
            .round()
            .clamp(f32::from(i16::MIN), f32::from(i16::MAX));

        scaled as i16
    }

    fn patterned_pcm16(len: usize) -> Vec<i16> {
        (0..len)
            .map(|index| {
                let value = (index.wrapping_mul(977).wrapping_add(12_345)) & 0xffff;

                #[allow(
                    clippy::cast_possible_truncation,
                    reason = "test values are intentionally wrapped to the full 16-bit PCM domain"
                )]
                {
                    let sample_bits =
                        u16::try_from(value).expect("masked test value should fit in u16");
                    sample_bits.cast_signed()
                }
            })
            .collect()
    }

    fn patterned_f32(len: usize) -> Vec<f32> {
        (0..len)
            .map(|index| {
                let pcm16 = patterned_pcm16_value(index);
                let offset_bits = u16::try_from((index.wrapping_mul(37).wrapping_add(11)) & 0x03ff)
                    .expect("masked test offset should fit in u16");
                let offset = (f32::from(offset_bits) / 1024.0) - 0.5;

                (f32::from(pcm16) + offset) / 32768.0
            })
            .collect()
    }

    fn patterned_pcm16_value(index: usize) -> i16 {
        let value = (index.wrapping_mul(977).wrapping_add(12_345)) & 0xffff;
        let sample_bits = u16::try_from(value).expect("masked test value should fit in u16");

        sample_bits.cast_signed()
    }

    fn seeded_pcm16(seed: u64, len: usize) -> Vec<i16> {
        let mut state = seed;

        (0..len)
            .map(|_| {
                state = state
                    .wrapping_mul(6_364_136_223_846_793_005)
                    .wrapping_add(1);
                let sample_bits =
                    u16::try_from(state >> 48).expect("shifted test state should fit in u16");

                sample_bits.cast_signed()
            })
            .collect()
    }

    fn seeded_f32(seed: u64, len: usize) -> Vec<f32> {
        let mut state = seed;

        (0..len)
            .map(|_| {
                state = state
                    .wrapping_mul(6_364_136_223_846_793_005)
                    .wrapping_add(1);
                let sample_bits =
                    u16::try_from(state >> 48).expect("shifted test state should fit in u16");
                state = state
                    .wrapping_mul(6_364_136_223_846_793_005)
                    .wrapping_add(1);
                let offset_bits = u16::try_from((state >> 54) & 0x03ff)
                    .expect("shifted test offset should fit in u16");
                let offset = (f32::from(offset_bits) / 512.0) - 1.0;

                (f32::from(sample_bits.cast_signed()) + offset) / 32768.0
            })
            .collect()
    }

    fn assert_sample_bits_eq(actual: &[f32], expected: &[f32]) {
        assert_eq!(actual.len(), expected.len());

        for (index, (&actual, &expected)) in actual.iter().zip(expected).enumerate() {
            assert_eq!(
                actual.to_bits(),
                expected.to_bits(),
                "sample {index} differed: {actual} != {expected}"
            );
        }
    }
}

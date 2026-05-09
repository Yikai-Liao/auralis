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
        ScalarBackend, SimdStatus, backend_descriptor, select_backend, select_backend_with_status,
        select_named_backend,
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
}

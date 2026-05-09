use core::fmt;

#[cfg(feature = "simd")]
use crate::selection::simd_target_supported;

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
    pub(crate) const fn new(
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

/// Optimized backend marker compiled when the optional `simd` feature is enabled.
///
/// Concrete kernels are exposed through Auralis-owned free functions so
/// `rten-simd` types remain private implementation details.
#[cfg(feature = "simd")]
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub struct SimdBackend;

#[cfg(feature = "simd")]
impl Backend for SimdBackend {
    const KIND: BackendKind = BackendKind::Simd;
    const NAME: &'static str = "simd";
    const AVAILABLE: bool = simd_target_supported();
}

mod private {
    use super::ScalarBackend;

    pub trait Sealed {}

    impl Sealed for ScalarBackend {}

    #[cfg(feature = "simd")]
    impl Sealed for super::SimdBackend {}
}

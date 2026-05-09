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
    const AVAILABLE: bool = true;
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

    use super::{Backend, BackendDescriptor, BackendKind, ScalarBackend};

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
    fn public_backend_types_are_auralis_owned() {
        assert_public_type_name::<BackendDescriptor>();
        assert_public_type_name::<BackendKind>();
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
        assert!(descriptor.is_available());
        assert_eq!(BackendKind::Simd.to_string(), "simd");
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

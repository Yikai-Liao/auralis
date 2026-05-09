use core::any::type_name;

use crate::{
    Backend, BackendDescriptor, BackendFallbackReason, BackendKind, BackendSelection, MixError,
    MultiplyError, SampleConversionError, ScalarBackend, backend_descriptor, select_backend,
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
    assert_public_type_name::<MixError>();
    assert_public_type_name::<MultiplyError>();
    assert_public_type_name::<SampleConversionError>();
    assert_public_type_name::<ScalarBackend>();

    #[cfg(feature = "simd")]
    assert_public_type_name::<crate::SimdBackend>();
}

#[cfg(feature = "simd")]
#[test]
fn placeholder_simd_backend_compiles_with_selected_crate() {
    let descriptor = descriptor_for::<crate::SimdBackend>();

    assert_eq!(descriptor.kind(), BackendKind::Simd);
    assert_eq!(descriptor.name(), "simd");
    assert_eq!(
        descriptor.is_available(),
        crate::selection::simd_target_supported()
    );
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

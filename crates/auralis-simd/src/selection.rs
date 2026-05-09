use crate::{BackendDescriptor, BackendFallbackReason, BackendKind, BackendSelection};

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

pub(crate) const fn select_backend_with_status(
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

pub(crate) const fn simd_target_supported() -> bool {
    cfg!(any(
        target_arch = "x86_64",
        target_arch = "aarch64",
        all(target_arch = "wasm32", target_feature = "simd128")
    ))
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum SimdStatus {
    Available,
    FeatureDisabled,
    TargetUnsupported,
}

#[cfg(test)]
mod tests {
    use super::{SimdStatus, select_backend_with_status};
    use crate::{BackendFallbackReason, BackendKind};

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
}

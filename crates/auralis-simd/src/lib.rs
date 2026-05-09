//! SIMD backends for Auralis.
//!
//! This crate owns the backend trait skeleton used by future sample-processing
//! kernels. The scalar backend is the deterministic reference implementation;
//! optimized backends implement the same Auralis-owned traits while keeping
//! implementation crates such as `rten-simd` out of public API types.
//!
//! Gain kernels accept a linear amplitude multiplier. DC shift kernels accept a
//! normalized full-scale offset. Fade kernels apply linear envelope
//! multiplication over frame-indexed channel segments. Mix kernels combine
//! same-channel input slices with a caller-provided balancing scale and treat
//! short inputs as trailing silence. Multiply kernels multiply corresponding
//! same-channel input samples and treat short inputs as silence. Higher-level
//! DSP APIs validate effect configuration before dispatching here.
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

mod backend;
mod convert;
mod dcshift;
mod fade;
mod gain;
mod mix;
mod multiply;
mod selection;

#[cfg(test)]
mod backend_tests;
#[cfg(test)]
mod convert_tests;
#[cfg(test)]
mod dcshift_tests;
#[cfg(test)]
mod fade_tests;
#[cfg(test)]
mod gain_tests;
#[cfg(test)]
mod mix_tests;
#[cfg(test)]
mod multiply_tests;
#[cfg(test)]
mod test_support;

#[cfg(feature = "simd")]
pub use backend::SimdBackend;
pub use backend::{
    Backend, BackendDescriptor, BackendFallbackReason, BackendKind, BackendSelection, ScalarBackend,
};
pub use convert::{
    SampleConversionError, f32_to_i16_scalar, f32_to_i16_with_backend, i16_to_f32_scalar,
    i16_to_f32_with_backend,
};
pub use dcshift::{dc_shift_f32_in_place_scalar, dc_shift_f32_in_place_with_backend};
pub use fade::{fade_f32_in_place_scalar, fade_f32_in_place_with_backend};
pub use gain::{gain_f32_in_place_scalar, gain_f32_in_place_with_backend};
pub use mix::{MixError, mix_f32_scalar, mix_f32_with_backend};
pub use multiply::{MultiplyError, multiply_f32_scalar, multiply_f32_with_backend};
pub use selection::{backend_descriptor, select_backend, select_named_backend};

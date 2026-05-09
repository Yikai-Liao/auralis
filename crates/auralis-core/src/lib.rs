//! Core vocabulary for Auralis.
//!
//! This crate is intentionally empty in the workspace skeleton. Core audio
//! error and type definitions are introduced by the next feature.

#[cfg(test)]
mod tests {
    #[test]
    fn crate_is_linkable() {
        assert_eq!(env!("CARGO_PKG_NAME"), "auralis-core");
    }
}

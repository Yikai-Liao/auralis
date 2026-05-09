//! Test utilities for Auralis.
//!
//! This crate is a placeholder in the workspace skeleton. Metrics and corpus
//! helpers are introduced by later test infrastructure features.

#[cfg(test)]
mod tests {
    #[test]
    fn crate_is_linkable() {
        assert_eq!(env!("CARGO_PKG_NAME"), "auralis-testkit");
    }
}

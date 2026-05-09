//! Codec boundaries for Auralis.
//!
//! This crate is a placeholder in the workspace skeleton. Reader and writer
//! traits are introduced by the WAV codec trait boundary feature.

#[cfg(test)]
mod tests {
    #[test]
    fn crate_is_linkable() {
        assert_eq!(env!("CARGO_PKG_NAME"), "auralis-codec");
    }
}

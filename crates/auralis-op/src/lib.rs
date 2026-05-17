//! Operation contracts shared by graph, effects, CLI, and future bindings.
//!
//! This crate owns metadata-only operation descriptors and deterministic
//! catalog validation. Concrete DSP implementations, graph planning, codec I/O,
//! and CLI parsing stay in their own crates.

use std::{collections::BTreeMap, error::Error, fmt};

/// Static metadata for one graph operation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct OpDescriptor {
    /// Stable canonical operation name.
    pub name: &'static str,
    /// Alternate names accepted during frontend lowering.
    pub aliases: &'static [&'static str],
    /// High-level operation family.
    pub category: OpCategory,
    /// Input ports accepted by this operation.
    pub inputs: &'static [PortDescriptor],
    /// Output ports produced by this operation.
    pub outputs: &'static [PortDescriptor],
    /// Named parameter descriptors.
    pub params: &'static [ParamDescriptor],
    /// Execution and planning capabilities.
    pub capabilities: OpCapabilities,
    /// Example fragments for help, docs, and validation fixtures.
    pub examples: &'static [OpExample],
}

impl OpDescriptor {
    /// Returns true when `name` is the canonical name or one of the aliases.
    #[must_use]
    pub fn matches_name(self, name: &str) -> bool {
        self.name == name || self.aliases.contains(&name)
    }
}

/// High-level operation family used for discovery and help output.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum OpCategory {
    /// Operation transforms audio into audio.
    AudioTransform,
    /// Operation combines multiple audio inputs.
    Mixer,
    /// Operation analyzes audio and may produce reports.
    Analyzer,
    /// Operation reads, writes, or adapts structured data.
    Utility,
}

/// Static metadata for one operation input or output port.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PortDescriptor {
    /// Stable port name.
    pub name: &'static str,
    /// Kind of value accepted or produced by this port.
    pub kind: PortKind,
    /// Whether this port accepts multiple bindings.
    pub variadic: bool,
    /// Short human-readable summary.
    pub summary: &'static str,
}

/// Value kind for an operation port.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum PortKind {
    /// Whole-buffer audio.
    Audio,
    /// Structured analysis report.
    Report,
}

/// Static metadata for one named operation parameter.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ParamDescriptor {
    /// Stable parameter name.
    pub name: &'static str,
    /// Accepted value kind.
    pub kind: ParamKind,
    /// Whether the parameter is required before preparation.
    pub required: bool,
    /// Short human-readable summary.
    pub summary: &'static str,
}

/// Value kind accepted by a named parameter.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum ParamKind {
    /// Raw string or typed unit value.
    String,
    /// Floating-point number.
    Number,
    /// Integer number.
    Integer,
    /// Boolean flag.
    Bool,
}

/// Execution and planning capabilities for an operation.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub struct OpCapabilities {
    /// Whether repeated evaluation with the same inputs is deterministic.
    pub deterministic: bool,
    /// Whether this operation needs complete input buffers before producing
    /// output.
    pub whole_buffer: bool,
}

/// Example operation fragment attached to a descriptor.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct OpExample {
    /// Frontend or format label, such as `cli` or `toml`.
    pub surface: &'static str,
    /// Example text.
    pub text: &'static str,
}

/// Deterministically validated operation catalog.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OpCatalog {
    descriptors: Vec<OpDescriptor>,
}

impl OpCatalog {
    /// Builds a catalog sorted by canonical operation name.
    ///
    /// Validation rejects empty names, duplicate canonical names, duplicate
    /// aliases, alias-to-canonical collisions, and descriptors without output
    /// ports.
    ///
    /// # Errors
    ///
    /// Returns [`OpCatalogError`] when descriptor metadata violates the shared
    /// catalog contract.
    pub fn new(
        descriptors: impl IntoIterator<Item = OpDescriptor>,
    ) -> Result<Self, OpCatalogError> {
        let mut descriptors = descriptors.into_iter().collect::<Vec<_>>();
        descriptors.sort_by_key(|descriptor| descriptor.name);
        validate_descriptors(&descriptors)?;
        Ok(Self { descriptors })
    }

    /// Returns descriptors in deterministic canonical-name order.
    #[must_use]
    pub fn descriptors(&self) -> &[OpDescriptor] {
        &self.descriptors
    }

    /// Resolves a canonical name or alias.
    #[must_use]
    pub fn resolve(&self, name: &str) -> Option<OpDescriptor> {
        self.descriptors
            .iter()
            .copied()
            .find(|descriptor| descriptor.matches_name(name))
    }
}

fn validate_descriptors(descriptors: &[OpDescriptor]) -> Result<(), OpCatalogError> {
    let mut names = BTreeMap::new();
    let mut aliases = BTreeMap::new();

    for descriptor in descriptors {
        let name = descriptor.name.trim();
        if name.is_empty() {
            return Err(OpCatalogError::EmptyName);
        }
        if let Some(first) = names.insert(descriptor.name, descriptor.name) {
            return Err(OpCatalogError::DuplicateName {
                name: descriptor.name,
                first,
                second: descriptor.name,
            });
        }
    }

    for descriptor in descriptors {
        if descriptor.outputs.is_empty() {
            return Err(OpCatalogError::MissingOutputs {
                name: descriptor.name,
            });
        }
        for alias in descriptor.aliases {
            if alias.trim().is_empty() {
                return Err(OpCatalogError::EmptyAlias {
                    name: descriptor.name,
                });
            }
            if names.contains_key(alias) {
                return Err(OpCatalogError::AliasCollidesWithName {
                    alias,
                    name: descriptor.name,
                });
            }
            if let Some(first) = aliases.insert(*alias, descriptor.name) {
                return Err(OpCatalogError::DuplicateAlias {
                    alias,
                    first,
                    second: descriptor.name,
                });
            }
        }
    }

    Ok(())
}

/// Catalog validation failure.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum OpCatalogError {
    /// A descriptor has an empty canonical name.
    EmptyName,
    /// A descriptor has an empty alias.
    EmptyAlias {
        /// Canonical name of the descriptor carrying the empty alias.
        name: &'static str,
    },
    /// Two descriptors use the same canonical name.
    DuplicateName {
        /// Duplicated canonical name.
        name: &'static str,
        /// First descriptor canonical name.
        first: &'static str,
        /// Second descriptor canonical name.
        second: &'static str,
    },
    /// Two descriptors use the same alias.
    DuplicateAlias {
        /// Duplicated alias.
        alias: &'static str,
        /// Canonical name that first claimed the alias.
        first: &'static str,
        /// Canonical name that also claimed the alias.
        second: &'static str,
    },
    /// An alias collides with a canonical name.
    AliasCollidesWithName {
        /// Colliding alias.
        alias: &'static str,
        /// Canonical name of the descriptor carrying the alias.
        name: &'static str,
    },
    /// A descriptor has no output port metadata.
    MissingOutputs {
        /// Canonical name missing outputs.
        name: &'static str,
    },
}

impl fmt::Display for OpCatalogError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::EmptyName => formatter.write_str("op descriptor name cannot be empty"),
            Self::EmptyAlias { name } => {
                write!(formatter, "op descriptor `{name}` has an empty alias")
            }
            Self::DuplicateName { name, .. } => {
                write!(formatter, "duplicate op descriptor name `{name}`")
            }
            Self::DuplicateAlias {
                alias,
                first,
                second,
            } => write!(
                formatter,
                "duplicate op alias `{alias}` for descriptors `{first}` and `{second}`"
            ),
            Self::AliasCollidesWithName { alias, name } => write!(
                formatter,
                "op descriptor `{name}` alias `{alias}` collides with a canonical op name"
            ),
            Self::MissingOutputs { name } => {
                write!(
                    formatter,
                    "op descriptor `{name}` must declare output ports"
                )
            }
        }
    }
}

impl Error for OpCatalogError {}

#[cfg(test)]
mod tests {
    use super::{
        OpCapabilities, OpCatalog, OpCatalogError, OpCategory, OpDescriptor, ParamDescriptor,
        ParamKind, PortDescriptor, PortKind,
    };

    const AUDIO_IN: &[PortDescriptor] = &[PortDescriptor {
        name: "in",
        kind: PortKind::Audio,
        variadic: false,
        summary: "input audio",
    }];
    const AUDIO_OUT: &[PortDescriptor] = &[PortDescriptor {
        name: "audio",
        kind: PortKind::Audio,
        variadic: false,
        summary: "output audio",
    }];
    const GAIN_PARAMS: &[ParamDescriptor] = &[ParamDescriptor {
        name: "by",
        kind: ParamKind::String,
        required: true,
        summary: "gain amount",
    }];

    const fn descriptor(name: &'static str, aliases: &'static [&'static str]) -> OpDescriptor {
        OpDescriptor {
            name,
            aliases,
            category: OpCategory::AudioTransform,
            inputs: AUDIO_IN,
            outputs: AUDIO_OUT,
            params: GAIN_PARAMS,
            capabilities: OpCapabilities {
                deterministic: true,
                whole_buffer: true,
            },
            examples: &[],
        }
    }

    #[test]
    fn catalog_sorts_and_resolves_names_and_aliases() {
        let catalog = OpCatalog::new([
            descriptor("trim", &[]),
            descriptor("gain", &["volume"]),
            descriptor("fade", &[]),
        ])
        .unwrap();

        let names = catalog
            .descriptors()
            .iter()
            .map(|descriptor| descriptor.name)
            .collect::<Vec<_>>();
        assert_eq!(names, ["fade", "gain", "trim"]);
        assert_eq!(catalog.resolve("gain").unwrap().name, "gain");
        assert_eq!(catalog.resolve("volume").unwrap().name, "gain");
        assert!(catalog.resolve("missing").is_none());
    }

    #[test]
    fn catalog_rejects_duplicate_names() {
        let error = OpCatalog::new([descriptor("gain", &[]), descriptor("gain", &[])]).unwrap_err();

        assert_eq!(
            error,
            OpCatalogError::DuplicateName {
                name: "gain",
                first: "gain",
                second: "gain",
            }
        );
    }

    #[test]
    fn catalog_rejects_duplicate_aliases() {
        let error = OpCatalog::new([descriptor("gain", &["vol"]), descriptor("trim", &["vol"])])
            .unwrap_err();

        assert_eq!(
            error,
            OpCatalogError::DuplicateAlias {
                alias: "vol",
                first: "gain",
                second: "trim",
            }
        );
    }

    #[test]
    fn catalog_rejects_alias_colliding_with_name() {
        let error =
            OpCatalog::new([descriptor("gain", &[]), descriptor("trim", &["gain"])]).unwrap_err();

        assert_eq!(
            error,
            OpCatalogError::AliasCollidesWithName {
                alias: "gain",
                name: "trim",
            }
        );
    }

    #[test]
    fn catalog_rejects_descriptors_without_outputs() {
        let error = OpCatalog::new([OpDescriptor {
            outputs: &[],
            ..descriptor("gain", &[])
        }])
        .unwrap_err();

        assert_eq!(error, OpCatalogError::MissingOutputs { name: "gain" });
    }
}

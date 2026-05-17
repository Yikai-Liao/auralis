//! Operation descriptors exported by implemented effects.

use auralis_op::{
    OpCapabilities, OpCatalog, OpCatalogError, OpCategory, OpDescriptor, ParamDescriptor,
    ParamKind, PortDescriptor, PortKind,
};

const AUDIO_INPUT: &[PortDescriptor] = &[PortDescriptor {
    name: "in",
    kind: PortKind::Audio,
    variadic: false,
    summary: "input audio",
}];

const AUDIO_OUTPUT: &[PortDescriptor] = &[PortDescriptor {
    name: "audio",
    kind: PortKind::Audio,
    variadic: false,
    summary: "output audio",
}];

const WHOLE_BUFFER_DETERMINISTIC: OpCapabilities = OpCapabilities {
    deterministic: true,
    whole_buffer: true,
};

const FADE_PARAMS: &[ParamDescriptor] = &[
    ParamDescriptor {
        name: "curve",
        kind: ParamKind::String,
        required: false,
        summary: "fade curve token or name",
    },
    ParamDescriptor {
        name: "in",
        kind: ParamKind::Integer,
        required: false,
        summary: "fade-in frame count",
    },
    ParamDescriptor {
        name: "stop",
        kind: ParamKind::Integer,
        required: false,
        summary: "output stop frame for positioned fade-out",
    },
    ParamDescriptor {
        name: "out",
        kind: ParamKind::Integer,
        required: false,
        summary: "fade-out frame count",
    },
];

const GAIN_PARAMS: &[ParamDescriptor] = &[
    ParamDescriptor {
        name: "by",
        kind: ParamKind::String,
        required: false,
        summary: "gain amount in decibels",
    },
    ParamDescriptor {
        name: "normalize",
        kind: ParamKind::Bool,
        required: false,
        summary: "normalize peak level before fixed gain",
    },
    ParamDescriptor {
        name: "limiter",
        kind: ParamKind::Bool,
        required: false,
        summary: "apply SoX-ng simple limiter after gain",
    },
    ParamDescriptor {
        name: "headroom",
        kind: ParamKind::String,
        required: false,
        summary: "headroom reserve or reclaim mode",
    },
    ParamDescriptor {
        name: "channel_mode",
        kind: ParamKind::String,
        required: false,
        summary: "channel-aware gain scan mode",
    },
];

const TRIM_PARAMS: &[ParamDescriptor] = &[ParamDescriptor {
    name: "position",
    kind: ParamKind::String,
    required: true,
    summary: "SoX-ng-style trim position or range",
}];

/// Initial graph-operation descriptors owned by `auralis-effects`.
pub const SUPPORTED_OPS: &[OpDescriptor] = &[
    OpDescriptor {
        name: "gain",
        aliases: &["gain-db", "gain_db"],
        category: OpCategory::AudioTransform,
        inputs: AUDIO_INPUT,
        outputs: AUDIO_OUTPUT,
        params: GAIN_PARAMS,
        capabilities: WHOLE_BUFFER_DETERMINISTIC,
        examples: &[],
    },
    OpDescriptor {
        name: "trim",
        aliases: &[],
        category: OpCategory::AudioTransform,
        inputs: AUDIO_INPUT,
        outputs: AUDIO_OUTPUT,
        params: TRIM_PARAMS,
        capabilities: WHOLE_BUFFER_DETERMINISTIC,
        examples: &[],
    },
    OpDescriptor {
        name: "fade",
        aliases: &[],
        category: OpCategory::AudioTransform,
        inputs: AUDIO_INPUT,
        outputs: AUDIO_OUTPUT,
        params: FADE_PARAMS,
        capabilities: WHOLE_BUFFER_DETERMINISTIC,
        examples: &[],
    },
];

/// Builds the validated operation catalog exported by this crate.
///
/// # Errors
///
/// Returns [`OpCatalogError`] when effect-owned operation descriptors violate
/// the shared operation catalog contract.
pub fn operation_catalog() -> Result<OpCatalog, OpCatalogError> {
    OpCatalog::new(SUPPORTED_OPS.iter().copied())
}

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

const DC_SHIFT_PARAMS: &[ParamDescriptor] = &[
    ParamDescriptor {
        name: "shift",
        kind: ParamKind::Number,
        required: true,
        summary: "normalized full-scale offset",
    },
    ParamDescriptor {
        name: "limiter_gain",
        kind: ParamKind::Number,
        required: false,
        summary: "optional limiter gain",
    },
];

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
        name: "dcshift",
        aliases: &["dc-shift", "dc_shift"],
        category: OpCategory::AudioTransform,
        inputs: AUDIO_INPUT,
        outputs: AUDIO_OUTPUT,
        params: DC_SHIFT_PARAMS,
        capabilities: WHOLE_BUFFER_DETERMINISTIC,
        examples: &[],
    },
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
        name: "reverse",
        aliases: &[],
        category: OpCategory::AudioTransform,
        inputs: AUDIO_INPUT,
        outputs: AUDIO_OUTPUT,
        params: &[],
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

#[cfg(test)]
mod tests {
    use super::{ParamKind, SUPPORTED_OPS, operation_catalog};

    #[test]
    fn operation_catalog_resolves_new_effect_owned_ops() {
        let catalog = operation_catalog().unwrap();

        assert_eq!(catalog.resolve("dc-shift").unwrap().name, "dcshift");
        assert_eq!(catalog.resolve("dc_shift").unwrap().name, "dcshift");
        assert_eq!(catalog.resolve("reverse").unwrap().name, "reverse");
    }

    #[test]
    fn dc_shift_descriptor_exposes_required_shift_and_optional_limiter() {
        let catalog = operation_catalog().unwrap();
        let dc_shift = catalog.resolve("dcshift").unwrap();

        assert_eq!(dc_shift.inputs, super::AUDIO_INPUT);
        assert_eq!(dc_shift.outputs, super::AUDIO_OUTPUT);
        assert!(dc_shift.capabilities.deterministic);
        assert!(dc_shift.capabilities.whole_buffer);
        assert_eq!(dc_shift.params.len(), 2);
        assert_eq!(dc_shift.params[0].name, "shift");
        assert_eq!(dc_shift.params[0].kind, ParamKind::Number);
        assert!(dc_shift.params[0].required);
        assert_eq!(dc_shift.params[1].name, "limiter_gain");
        assert_eq!(dc_shift.params[1].kind, ParamKind::Number);
        assert!(!dc_shift.params[1].required);
    }

    #[test]
    fn reverse_descriptor_is_parameterless_audio_transform() {
        let catalog = operation_catalog().unwrap();
        let reverse = catalog.resolve("reverse").unwrap();

        assert_eq!(reverse.inputs, super::AUDIO_INPUT);
        assert_eq!(reverse.outputs, super::AUDIO_OUTPUT);
        assert!(reverse.params.is_empty());
        assert!(reverse.capabilities.deterministic);
        assert!(reverse.capabilities.whole_buffer);
    }

    #[test]
    fn exported_ops_stay_covered_by_validated_catalog() {
        let catalog = operation_catalog().unwrap();

        assert_eq!(catalog.descriptors().len(), SUPPORTED_OPS.len());
    }
}

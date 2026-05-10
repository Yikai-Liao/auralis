//! Integration coverage for the public effect registry.

use auralis_effects::{EffectKind, EffectNameError, EffectRegistry, SUPPORTED_EFFECTS};

#[test]
fn supported_canonical_names_resolve_to_descriptors() {
    let expected = [
        ("allpass", EffectKind::AllPass),
        ("band", EffectKind::Band),
        ("bandpass", EffectKind::BandPass),
        ("bandreject", EffectKind::BandReject),
        ("bass", EffectKind::Bass),
        ("biquad", EffectKind::Biquad),
        ("centercut", EffectKind::Centercut),
        ("channels", EffectKind::Channels),
        ("chorus", EffectKind::Chorus),
        ("compand", EffectKind::Compand),
        ("contrast", EffectKind::Contrast),
        ("dcshift", EffectKind::DcShift),
        ("delay", EffectKind::Delay),
        ("dither", EffectKind::Dither),
        ("earwax", EffectKind::Earwax),
        ("echo", EffectKind::Echo),
        ("echos", EffectKind::Echos),
        ("deemph", EffectKind::Deemph),
        ("equalizer", EffectKind::Equalizer),
        ("fade", EffectKind::Fade),
        ("fir", EffectKind::Fir),
        ("firfit", EffectKind::FirFit),
        ("flanger", EffectKind::Flanger),
        ("gain", EffectKind::Gain),
        ("highpass", EffectKind::HighPass),
        ("hilbert", EffectKind::Hilbert),
        ("loudness", EffectKind::Loudness),
        ("lowpass", EffectKind::LowPass),
        ("mcompand", EffectKind::MCompand),
        ("noiseprof", EffectKind::NoiseProf),
        ("norm", EffectKind::Norm),
        ("overdrive", EffectKind::Overdrive),
        ("pad", EffectKind::Pad),
        ("pitch", EffectKind::Pitch),
        ("rate", EffectKind::Rate),
        ("repeat", EffectKind::Repeat),
        ("reverb", EffectKind::Reverb),
        ("remix", EffectKind::Remix),
        ("reverse", EffectKind::Reverse),
        ("riaa", EffectKind::Riaa),
        ("saturation", EffectKind::Saturation),
        ("silence", EffectKind::Silence),
        ("sinc", EffectKind::Sinc),
        ("softvol", EffectKind::SoftVol),
        ("stat", EffectKind::Stat),
        ("stats", EffectKind::Stats),
        ("synth", EffectKind::Synth),
        ("swap", EffectKind::Swap),
        ("treble", EffectKind::Treble),
        ("tremolo", EffectKind::Tremolo),
        ("trim", EffectKind::Trim),
        ("vad", EffectKind::Vad),
        ("vol", EffectKind::Vol),
    ];
    for (name, kind) in expected {
        let descriptor = EffectRegistry::resolve(name).unwrap();
        assert_eq!(descriptor.kind(), kind);
        assert_eq!(descriptor.canonical_name(), name);
    }
}

#[test]
fn supported_aliases_resolve_to_canonical_descriptors() {
    let expected = [
        ("dc-shift", "dcshift", EffectKind::DcShift),
        ("dc_shift", "dcshift", EffectKind::DcShift),
        ("eq", "equalizer", EffectKind::Equalizer),
        ("gain-db", "gain", EffectKind::Gain),
        ("gain_db", "gain", EffectKind::Gain),
        ("normalize", "norm", EffectKind::Norm),
        ("normalise", "norm", EffectKind::Norm),
        ("soft-volume", "softvol", EffectKind::SoftVol),
        ("soft_volume", "softvol", EffectKind::SoftVol),
        ("volume", "vol", EffectKind::Vol),
    ];
    for (alias, canonical, kind) in expected {
        let descriptor = EffectRegistry::resolve(alias).unwrap();
        assert_eq!(descriptor.kind(), kind);
        assert_eq!(descriptor.canonical_name(), canonical);
    }
}

#[test]
fn unknown_names_return_deterministic_suggestions() {
    let error = EffectRegistry::resolve("gian").unwrap_err();
    assert_eq!(
        error,
        EffectNameError::UnknownEffect {
            name: "gian".to_owned(),
            suggestions: vec!["gain", "riaa"],
        }
    );
    assert_eq!(error.suggestions(), &["gain", "riaa"]);
    assert_eq!(
        error.to_string(),
        "unknown effect `gian`; did you mean one of `gain`, `riaa`?"
    );
}

#[test]
fn empty_names_are_rejected_without_suggestions() {
    let error = EffectRegistry::resolve("").unwrap_err();

    assert_eq!(error, EffectNameError::EmptyName);
    assert!(error.suggestions().is_empty());
    assert_eq!(error.to_string(), "effect name cannot be empty");
}

#[test]
fn blocked_known_effects_return_actionable_diagnostics() {
    let cases = [
        (
            "dolbyb",
            "known SoX-ng effect `dolbyb` is blocked in Auralis: SoX-ng uses GPLv2 libdolbyb C code while Auralis is MIT and pure Rust; use `sox_ng ... dolbyb ...` for Dolby B processing or provide a compatible pure-Rust/public-domain spec",
        ),
        (
            "dop",
            "known SoX-ng effect `dop` is not planned in the Auralis effect registry: DoP is DSD-over-PCM transport packing from 1-bit DSD into 24-bit PCM samples, while Auralis currently processes PCM16 WAV audio effects; use `sox_ng ... dop ...` for DoP transport or wait for future DSD/DoP format support",
        ),
        (
            "ladspa",
            "known SoX-ng effect `ladspa` is blocked in Auralis: LADSPA support requires loading native external plugins through LADSPA_PATH and a plugin-host ABI, while Auralis currently accepts only MIT-compatible pure Rust effects; use `sox_ng ... ladspa ...` for LADSPA plugins or wait for a future external-host boundary",
        ),
        (
            "sdm",
            "known SoX-ng effect `sdm` is not planned in the Auralis effect registry: SDM is a DSD-oriented sigma-delta modulator that emits 1-bit output using SoX-ng's LGPL filter tables and trellis behavior, while Auralis currently processes PCM16 WAV audio effects; use `sox_ng ... sdm ...` for SDM processing or wait for future DSD/1-bit format support",
        ),
    ];

    for (name, message) in cases {
        let error = EffectRegistry::resolve(name).unwrap_err();

        assert_eq!(
            error,
            EffectNameError::UnsupportedSoxNgEffect {
                name: name.to_owned(),
            }
        );
        assert!(error.suggestions().is_empty());
        assert_eq!(error.to_string(), message);
    }
}

#[test]
fn implemented_sox_ng_names_are_supported_effect_descriptors() {
    for descriptor in SUPPORTED_EFFECTS {
        assert!(EffectRegistry::is_known_sox_ng_effect(
            descriptor.canonical_name()
        ));
        assert!(EffectRegistry::resolve(descriptor.canonical_name()).is_ok());
    }
}

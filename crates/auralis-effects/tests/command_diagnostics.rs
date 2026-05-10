//! Diagnostics coverage for the public typed effect command parser.

use auralis_effects::parse_effect_command;

#[test]
fn unsupported_and_unknown_effect_names_use_registry_diagnostics() {
    let blocked = parse_effect_command(&["dolbyb"]).unwrap_err();
    assert_eq!(
        blocked.to_string(),
        "known SoX-ng effect `dolbyb` is blocked in Auralis: SoX-ng uses GPLv2 libdolbyb C code while Auralis is MIT and pure Rust; use `sox_ng ... dolbyb ...` for Dolby B processing or provide a compatible pure-Rust/public-domain spec"
    );

    let not_planned = parse_effect_command(&["dop"]).unwrap_err();
    assert_eq!(
        not_planned.to_string(),
        "known SoX-ng effect `dop` is not planned in the Auralis effect registry: DoP is DSD-over-PCM transport packing from 1-bit DSD into 24-bit PCM samples, while Auralis currently processes PCM16 WAV audio effects; use `sox_ng ... dop ...` for DoP transport or wait for future DSD/DoP format support"
    );

    let external_host = parse_effect_command(&["ladspa"]).unwrap_err();
    assert_eq!(
        external_host.to_string(),
        "known SoX-ng effect `ladspa` is blocked in Auralis: LADSPA support requires loading native external plugins through LADSPA_PATH and a plugin-host ABI, while Auralis currently accepts only MIT-compatible pure Rust effects; use `sox_ng ... ladspa ...` for LADSPA plugins or wait for a future external-host boundary"
    );

    let unknown = parse_effect_command(&["gian"]).unwrap_err();
    assert_eq!(
        unknown.to_string(),
        "unknown effect `gian`; did you mean one of `gain`, `riaa`?"
    );
}

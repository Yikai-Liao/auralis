//! Diagnostics coverage for the public typed effect command parser.

use auralis_effects::parse_effect_command;

#[test]
fn unsupported_and_unknown_effect_names_use_registry_diagnostics() {
    let blocked = parse_effect_command(&["dolbyb"]).unwrap_err();
    assert_eq!(
        blocked.to_string(),
        "known SoX-ng effect `dolbyb` is blocked in Auralis: SoX-ng uses GPLv2 libdolbyb C code while Auralis is MIT and pure Rust; use `sox_ng ... dolbyb ...` for Dolby B processing or provide a compatible pure-Rust/public-domain spec"
    );

    let unknown = parse_effect_command(&["gian"]).unwrap_err();
    assert_eq!(
        unknown.to_string(),
        "unknown effect `gian`; did you mean one of `gain`, `riaa`?"
    );
}

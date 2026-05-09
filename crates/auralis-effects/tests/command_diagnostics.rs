//! Diagnostics coverage for the public typed effect command parser.

use auralis_effects::parse_effect_command;

#[test]
fn unsupported_and_unknown_effect_names_use_registry_diagnostics() {
    let unsupported = parse_effect_command(&["lowpass"]).unwrap_err();
    assert!(
        unsupported
            .to_string()
            .contains("known SoX-ng effect `lowpass`")
    );

    let unknown = parse_effect_command(&["gian"]).unwrap_err();
    assert_eq!(
        unknown.to_string(),
        "unknown effect `gian`; did you mean `gain`?"
    );
}

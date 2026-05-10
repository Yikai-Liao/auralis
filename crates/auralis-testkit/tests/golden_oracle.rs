#![allow(missing_docs)]

use std::env;
use std::path::{Path, PathBuf};

#[test]
fn golden_gate_requires_sox_ng_oracle() {
    if !is_golden_gate_invocation() && !env_flag_enabled("AURALIS_REQUIRE_SOX_NG") {
        return;
    }

    let executable = if env::var_os("AURALIS_SOX_NG_BIN").is_some() {
        configured_or_path_oracle()
    } else {
        find_on_path("sox_ng")
    };

    assert!(
        executable.as_deref().is_some_and(Path::is_file),
        "release/gnhf golden validation requires a real sox_ng oracle; \
         set AURALIS_SOX_NG_BIN or install sox_ng on PATH"
    );
}

fn is_golden_gate_invocation() -> bool {
    env::args()
        .skip(1)
        .any(|argument| argument.contains("golden"))
}

fn env_flag_enabled(name: &str) -> bool {
    env::var(name).is_ok_and(|value| {
        let normalized = value.trim().to_ascii_lowercase();
        !matches!(normalized.as_str(), "" | "0" | "false" | "no" | "off")
    })
}

fn configured_or_path_oracle() -> Option<PathBuf> {
    let configured = env::var_os("AURALIS_SOX_NG_BIN")?;
    let configured_path = PathBuf::from(&configured);
    if configured_path.is_file() {
        return Some(configured_path);
    }

    configured
        .to_str()
        .filter(|binary| !binary.contains(std::path::MAIN_SEPARATOR))
        .and_then(find_on_path)
}

fn find_on_path(binary: &str) -> Option<PathBuf> {
    env::var_os("PATH").and_then(|paths| {
        env::split_paths(&paths)
            .map(|directory| directory.join(binary))
            .find(|candidate| candidate.is_file())
    })
}

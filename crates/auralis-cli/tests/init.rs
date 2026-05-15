//! Integration tests for `auralis init`.

mod support;

use support::*;

#[test]
fn init_creates_checkable_graph_spec_without_overwriting() {
    let dir = temp_path("auralis-cli-init", "dir");
    fs::create_dir_all(&dir).unwrap();
    let spec = dir.join("Auralis.toml");

    let init = Command::new(env!("CARGO_BIN_EXE_auralis"))
        .current_dir(&dir)
        .args(["init"])
        .output()
        .unwrap();

    assert!(init.status.success(), "stderr: {}", stderr(&init));
    let stdout = stdout(&init);
    assert!(stdout.contains("created Auralis.toml"), "{stdout}");
    let spec_text = fs::read_to_string(&spec).unwrap();
    assert!(spec_text.contains(r#"version = "auralis.graph/v1""#));
    assert!(spec_text.contains(r#"id = "master""#));

    let check = Command::new(env!("CARGO_BIN_EXE_auralis"))
        .args(["check", spec.to_str().unwrap()])
        .output()
        .unwrap();
    assert!(check.status.success(), "stderr: {}", stderr(&check));

    let second_init = Command::new(env!("CARGO_BIN_EXE_auralis"))
        .current_dir(&dir)
        .args(["init"])
        .output()
        .unwrap();
    assert!(!second_init.status.success());
    assert!(stderr(&second_init).contains("File exists"));

    fs::remove_dir_all(dir).unwrap();
}

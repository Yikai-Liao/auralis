//! Integration tests for `auralis cache`.

mod support;

use support::*;

#[test]
fn cache_status_reports_absent_default_cache() {
    let dir = temp_path("auralis-cli-cache-status-absent", "dir");
    fs::create_dir_all(&dir).unwrap();

    let output = Command::new(env!("CARGO_BIN_EXE_auralis"))
        .current_dir(&dir)
        .args(["cache", "status"])
        .output()
        .unwrap();

    fs::remove_dir_all(dir).unwrap();
    assert!(output.status.success(), "stderr: {}", stderr(&output));
    let stdout = stdout(&output);
    assert!(stdout.contains("Cache: .auralis/cache"), "{stdout}");
    assert!(stdout.contains("Status: absent"), "{stdout}");
    assert!(stdout.contains("Entries: 0"), "{stdout}");
}

#[test]
fn cache_status_json_counts_files_under_custom_root() {
    let dir = temp_path("auralis-cli-cache-status-json", "dir");
    let cache = dir.join("cache");
    let nested = cache.join("nested");
    fs::create_dir_all(&nested).unwrap();
    fs::write(cache.join("a.bin"), [1_u8, 2, 3]).unwrap();
    fs::write(nested.join("b.bin"), [4_u8, 5]).unwrap();

    let output = Command::new(env!("CARGO_BIN_EXE_auralis"))
        .args([
            "cache",
            "status",
            "--root",
            cache.to_str().unwrap(),
            "--json",
        ])
        .output()
        .unwrap();

    fs::remove_dir_all(dir).unwrap();
    assert!(output.status.success(), "stderr: {}", stderr(&output));
    let status: serde_json::Value = serde_json::from_str(&stdout(&output)).unwrap();
    assert_eq!(status["exists"], true);
    assert_eq!(status["entries"], 2);
    assert_eq!(status["bytes"], 5);
}

#[test]
fn cache_clear_requires_confirmation_and_removes_files() {
    let dir = temp_path("auralis-cli-cache-clear", "dir");
    let cache = dir.join("cache");
    let nested = cache.join("nested");
    fs::create_dir_all(&nested).unwrap();
    fs::write(cache.join("a.bin"), [1_u8, 2, 3]).unwrap();
    fs::write(nested.join("b.bin"), [4_u8, 5]).unwrap();

    let rejected = Command::new(env!("CARGO_BIN_EXE_auralis"))
        .args(["cache", "clear", "--root", cache.to_str().unwrap()])
        .output()
        .unwrap();
    assert!(!rejected.status.success());
    assert!(
        stderr(&rejected).contains("cache clear requires --yes"),
        "{}",
        stderr(&rejected)
    );
    assert!(cache.join("a.bin").exists());

    let cleared = Command::new(env!("CARGO_BIN_EXE_auralis"))
        .args(["cache", "clear", "--root", cache.to_str().unwrap(), "--yes"])
        .output()
        .unwrap();

    assert!(cleared.status.success(), "stderr: {}", stderr(&cleared));
    let stdout = stdout(&cleared);
    assert!(stdout.contains("Removed entries: 2"), "{stdout}");
    assert!(stdout.contains("Removed bytes: 5"), "{stdout}");
    assert!(cache.exists());
    assert_eq!(fs::read_dir(&cache).unwrap().count(), 0);

    fs::remove_dir_all(dir).unwrap();
}

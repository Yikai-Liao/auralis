//! Cache inspection commands.

use std::{
    fs,
    path::{Path, PathBuf},
};

use serde::Serialize;

use crate::CliError;

#[derive(Debug, Serialize)]
struct CacheStatus {
    path: PathBuf,
    exists: bool,
    entries: usize,
    bytes: u64,
}

pub(super) fn print_cache_status(root: &Path, json: bool) -> Result<(), CliError> {
    let status = read_cache_status(root)?;

    if json {
        println!("{}", serde_json::to_string_pretty(&status)?);
    } else {
        println!("Cache: {}", status.path.display());
        println!(
            "Status: {}",
            if status.exists { "present" } else { "absent" }
        );
        println!("Entries: {}", status.entries);
        println!("Bytes: {}", status.bytes);
    }

    Ok(())
}

fn read_cache_status(root: &Path) -> Result<CacheStatus, CliError> {
    if !root.exists() {
        return Ok(CacheStatus {
            path: root.to_path_buf(),
            exists: false,
            entries: 0,
            bytes: 0,
        });
    }

    let mut status = CacheStatus {
        path: root.to_path_buf(),
        exists: true,
        entries: 0,
        bytes: 0,
    };
    accumulate_cache_stats(root, &mut status)?;
    Ok(status)
}

fn accumulate_cache_stats(path: &Path, status: &mut CacheStatus) -> Result<(), CliError> {
    for entry in fs::read_dir(path)? {
        let entry = entry?;
        let metadata = entry.metadata()?;

        if metadata.is_dir() {
            accumulate_cache_stats(&entry.path(), status)?;
        } else if metadata.is_file() {
            status.entries += 1;
            status.bytes += metadata.len();
        }
    }

    Ok(())
}

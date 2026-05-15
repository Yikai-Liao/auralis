//! Cache inspection commands.

use std::{
    fs,
    path::{Path, PathBuf},
};

use serde::Serialize;

use crate::{
    CliError,
    command_args::{CacheArgs, CacheCommand},
};

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

pub(super) fn run_cache_command(args: CacheArgs) -> Result<(), CliError> {
    match args.command {
        CacheCommand::Status(status) => print_cache_status(&status.root, status.json),
        CacheCommand::Clear(clear) => clear_cache(&clear.root, clear.yes),
    }
}

fn clear_cache(root: &Path, confirmed: bool) -> Result<(), CliError> {
    if !confirmed {
        return Err(CliError::CacheClearNeedsConfirmation);
    }

    let status = read_cache_status(root)?;

    if root.exists() {
        for entry in fs::read_dir(root)? {
            let entry = entry?;
            let path = entry.path();
            if entry.file_type()?.is_dir() {
                fs::remove_dir_all(path)?;
            } else {
                fs::remove_file(path)?;
            }
        }
    }

    println!("Cache: {}", status.path.display());
    println!("Removed entries: {}", status.entries);
    println!("Removed bytes: {}", status.bytes);

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

// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 ddavef/KinteLiX retouched-server

use std::fs::{File, OpenOptions, TryLockError};
use std::path::PathBuf;

pub enum InstanceLock {
    Held(#[allow(dead_code)] File),
    AlreadyRunning,
    Unenforced,
}

pub fn acquire() -> InstanceLock {
    let path = lock_path();
    if let Some(parent) = path.parent() {
        let _ = std::fs::create_dir_all(parent);
    }
    let file = match OpenOptions::new().create(true).write(true).open(&path) {
        Ok(f) => f,
        Err(e) => {
            log::warn!(
                "single-instance: could not open lock file {}: {}; allowing startup",
                path.display(),
                e
            );
            return InstanceLock::Unenforced;
        }
    };
    match file.try_lock() {
        Ok(()) => InstanceLock::Held(file),
        Err(TryLockError::WouldBlock) => InstanceLock::AlreadyRunning,
        Err(TryLockError::Error(e)) => {
            log::warn!(
                "single-instance: lock error on {}: {}; allowing startup",
                path.display(),
                e
            );
            InstanceLock::Unenforced
        }
    }
}

fn lock_path() -> PathBuf {
    directories::ProjectDirs::from("com", "retouched", "retouched-server")
        .map(|d| d.cache_dir().join("instance.lock"))
        .unwrap_or_else(|| std::env::temp_dir().join("retouched-server-instance.lock"))
}

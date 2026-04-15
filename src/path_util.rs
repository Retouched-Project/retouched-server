// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 ddavef/KinteLiX retouched-server

use std::path::PathBuf;

pub fn to_native_path(input: &str) -> PathBuf {
    let trimmed = input.trim();
    if trimmed.starts_with("file://") {
        if let Ok(url) = url::Url::parse(trimmed) {
            if let Ok(path) = url.to_file_path() {
                return path;
            }
        }
    }
    let p = PathBuf::from(trimmed);
    #[cfg(target_os = "windows")]
    {
        PathBuf::from(p.to_string_lossy().replace('/', "\\"))
    }
    #[cfg(not(target_os = "windows"))]
    {
        p
    }
}

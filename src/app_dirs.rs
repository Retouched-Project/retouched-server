// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 ddavef/KinteLiX retouched-server

use directories::ProjectDirs;
use std::path::{Path, PathBuf};

fn project_dirs() -> Option<ProjectDirs> {
    ProjectDirs::from("com", "retouched", "retouched-server")
}

/// Linux:   ~/.cache/retouched-server/tools
/// macOS:   ~/Library/Caches/com.retouched.retouched-server/tools
/// Windows: C:\Users\<user>\AppData\Local\retouched\retouched-server\cache\tools
pub fn tools_cache_dir(override_dir: Option<&Path>) -> PathBuf {
    if let Some(dir) = override_dir {
        return dir.to_path_buf();
    }
    project_dirs()
        .map(|d| d.cache_dir().join("tools"))
        .unwrap_or_else(|| PathBuf::from("cache").join("tools"))
}

/// Linux:   ~/.local/share/retouched-server
/// macOS:   ~/Library/Application Support/com.retouched.retouched-server
/// Windows: C:\Users\<user>\AppData\Roaming\retouched\retouched-server\data
pub fn app_data_dir(override_dir: Option<&Path>) -> PathBuf {
    if let Some(dir) = override_dir {
        return dir.to_path_buf();
    }
    project_dirs()
        .map(|d| d.data_dir().to_path_buf())
        .unwrap_or_else(|| PathBuf::from("data"))
}

/// Linux:   ~/.cache/retouched-server/icons
/// macOS:   ~/Library/Caches/com.retouched.retouched-server/icons
/// Windows: C:\Users\<user>\AppData\Local\retouched\retouched-server\cache\icons
pub fn icons_cache_dir(override_dir: Option<&Path>) -> PathBuf {
    if let Some(dir) = override_dir {
        return dir.join("cache").join("icons");
    }
    project_dirs()
        .map(|d| d.cache_dir().join("icons"))
        .unwrap_or_else(|| PathBuf::from("cache").join("icons"))
}

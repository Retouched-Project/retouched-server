// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 ddavef/KinteLiX retouched-server

// Handle Flash Player's idiotic security model

use std::path::PathBuf;

const TRUST_FILENAME: &str = "retouched.cfg";

pub fn trust_directory() -> PathBuf {
    #[cfg(target_os = "windows")]
    {
        if let Some(appdata) = std::env::var_os("APPDATA") {
            PathBuf::from(appdata)
                .join("Macromedia")
                .join("Flash Player")
                .join("#Security")
                .join("FlashPlayerTrust")
        } else {
            PathBuf::from("FlashPlayerTrust")
        }
    }
    #[cfg(target_os = "macos")]
    {
        dirs::home_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join("Library")
            .join("Preferences")
            .join("Macromedia")
            .join("Flash Player")
            .join("#Security")
            .join("FlashPlayerTrust")
    }
    #[cfg(target_os = "linux")]
    {
        dirs::home_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join(".macromedia")
            .join("Flash_Player")
            .join("#Security")
            .join("FlashPlayerTrust")
    }
    #[cfg(not(any(target_os = "windows", target_os = "macos", target_os = "linux")))]
    {
        PathBuf::from("FlashPlayerTrust")
    }
}

fn cfg_path() -> PathBuf {
    trust_directory().join(TRUST_FILENAME)
}

pub fn read_trusted_dirs() -> Vec<String> {
    let path = cfg_path();
    match std::fs::read_to_string(&path) {
        Ok(content) => content
            .lines()
            .map(|l| l.trim().to_string())
            .filter(|l| !l.is_empty())
            .collect(),
        Err(_) => Vec::new(),
    }
}

fn write_dirs(dirs: &[String]) -> Result<(), Box<dyn std::error::Error>> {
    let trust_dir = trust_directory();
    std::fs::create_dir_all(&trust_dir)?;
    let path = cfg_path();
    let content = dirs.join("\n");
    std::fs::write(&path, content)?;
    Ok(())
}

pub fn add_trusted_dir(dir: &str) -> Result<(), Box<dyn std::error::Error>> {
    let mut dirs = read_trusted_dirs();
    let normalized = dir.trim().to_string();
    if normalized.is_empty() {
        return Ok(());
    }
    if dirs.iter().any(|d| d == &normalized) {
        log::info!("Directory already in trust config: {}", normalized);
        return Ok(());
    }
    dirs.push(normalized);
    write_dirs(&dirs)?;
    log::info!("Added to trust config: {}", dir);
    Ok(())
}

pub fn remove_trusted_dir(dir: &str) -> Result<(), Box<dyn std::error::Error>> {
    let mut dirs = read_trusted_dirs();
    let before = dirs.len();
    dirs.retain(|d| d != dir);
    if dirs.len() < before {
        write_dirs(&dirs)?;
        log::info!("Removed from trust config: {}", dir);
    }
    Ok(())
}

pub fn write_trust_config(games_directory: &str) -> Result<(), Box<dyn std::error::Error>> {
    add_trusted_dir(games_directory)
}

pub fn remove_trust_config() -> Result<(), Box<dyn std::error::Error>> {
    let path = cfg_path();
    if path.exists() {
        std::fs::remove_file(&path)?;
        log::info!("Removed trust config: {}", path.display());
    }
    Ok(())
}

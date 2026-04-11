// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 ddavef/KinteLiX retouched-server

use std::path::{Path, PathBuf};

pub const RETOUCHED_WEB_RELEASES_URL: &str =
    "https://github.com/TODO/retouched_web/releases/latest/download/retouched_web_release.zip";

pub const WEB_APP_DIR_NAME: &str = "retouched_web";

pub fn web_app_dir(data_dir: &Path) -> PathBuf {
    data_dir.join(WEB_APP_DIR_NAME)
}

pub fn download_web_app(dir: &Path) -> Result<String, Box<dyn std::error::Error + Send + Sync>> {
    let client = reqwest::blocking::Client::builder()
        .user_agent("retouched-server")
        .build()?;

    let download_url = RETOUCHED_WEB_RELEASES_URL;

    log::info!("Downloading retouched_web from {}", download_url);

    let bytes = client
        .get(download_url)
        .send()?
        .error_for_status()?
        .bytes()?;

    if dir.exists() {
        std::fs::remove_dir_all(dir)?;
    }
    std::fs::create_dir_all(dir)?;

    let mut archive = zip::ZipArchive::new(std::io::Cursor::new(&bytes))?;
    for i in 0..archive.len() {
        let mut file = archive.by_index(i)?;
        let name = file.name().to_string();
        let out_path = match strip_zip_prefix(&name) {
            Some(stripped) => dir.join(stripped),
            None => dir.join(&name),
        };
        if file.is_dir() {
            std::fs::create_dir_all(&out_path)?;
        } else {
            if let Some(parent) = out_path.parent() {
                std::fs::create_dir_all(parent)?;
            }
            let mut out = std::fs::File::create(&out_path)?;
            std::io::copy(&mut file, &mut out)?;
        }
    }

    log::info!("Extracted retouched_web to {}", dir.display());
    Ok("latest".to_string())
}

fn strip_zip_prefix(name: &str) -> Option<&str> {
    let trimmed = name.trim_start_matches('/');
    if let Some(pos) = trimmed.find('/') {
        let rest = &trimmed[pos + 1..];
        if rest.is_empty() { None } else { Some(rest) }
    } else {
        None
    }
}

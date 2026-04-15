// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 ddavef/KinteLiX retouched-server

use std::path::{Path, PathBuf};

pub const RETOUCHED_WEB_RELEASES_URL: &str = "https://github.com/Retouched-Project/retouched_web/releases/latest/download/retouched_web_release.zip";

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
        let dir_name = dir.file_name().and_then(|n| n.to_str()).unwrap_or("");
        if dir_name != WEB_APP_DIR_NAME {
            return Err(format!(
                "Refusing to delete '{}': expected directory named '{}'",
                dir.display(),
                WEB_APP_DIR_NAME
            )
            .into());
        }
        if dir.parent().is_none() {
            return Err("Refusing to delete a root-level directory".into());
        }
        let dominated = std::fs::read_dir(dir)?.count() == 0 || dir.join("dist").join("index.html").exists();
        if !dominated {
            return Err(format!(
                "Refusing to delete '{}': does not look like a retouched_web directory",
                dir.display()
            )
            .into());
        }
        std::fs::remove_dir_all(dir)?;
    }
    std::fs::create_dir_all(dir)?;

    let canonical_dir = std::fs::canonicalize(dir)?;
    let mut archive = zip::ZipArchive::new(std::io::Cursor::new(&bytes))?;
    for i in 0..archive.len() {
        let mut file = archive.by_index(i)?;
        let name = file.name().to_string();
        let out_path = dir.join(&name);
        let canonical_out = canonical_dir.join(&name);
        if !canonical_out.starts_with(&canonical_dir) {
            log::warn!("Skipping zip entry with path traversal: {}", name);
            continue;
        }
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

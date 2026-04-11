// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 ddavef/KinteLiX retouched-server

use std::collections::HashMap;
use std::path::{Path, PathBuf};

pub struct IconCache {
    cache_dir: PathBuf,
}

impl IconCache {
    pub fn new(custom_data_dir: Option<&Path>) -> Self {
        let cache_dir = crate::app_dirs::icons_cache_dir(custom_data_dir);
        std::fs::create_dir_all(&cache_dir).ok();
        Self { cache_dir }
    }

    pub async fn download_icons(&self) {
        let config = match self.load_config() {
            Some(c) => c,
            None => return,
        };

        let client = match reqwest::Client::builder().build() {
            Ok(c) => c,
            Err(e) => {
                log::error!("[ICONS] Failed to create HTTP client: {}", e);
                return;
            }
        };

        for (app_id, url) in &config {
            let cached_path = self.cache_dir.join(format!("{}.png", app_id));
            if cached_path.exists() {
                log::debug!("[ICONS] Cached: {}.png", app_id);
                continue;
            }

            log::info!("[ICONS] Downloading {} from {}", app_id, url);
            match client.get(url).send().await {
                Ok(resp) if resp.status().is_success() => match resp.bytes().await {
                    Ok(bytes) => {
                        if let Err(e) = std::fs::write(&cached_path, &bytes) {
                            log::error!("[ICONS] Failed to save {}: {}", app_id, e);
                        } else {
                            log::info!("[ICONS] Saved: {}.png", app_id);
                        }
                    }
                    Err(e) => log::error!("[ICONS] Failed to read body for {}: {}", app_id, e),
                },
                Ok(resp) => log::error!("[ICONS] HTTP {} for {}", resp.status(), app_id),
                Err(e) => log::error!("[ICONS] Failed to download {}: {}", app_id, e),
            }
        }
    }

    pub fn get_icon(&self, app_id: &str) -> Option<Vec<u8>> {
        let cached_path = self.cache_dir.join(format!("{}.png", app_id));
        std::fs::read(&cached_path).ok()
    }

    fn load_config(&self) -> Option<HashMap<String, String>> {
        let content = include_str!("../icon_config.json");

        match serde_json::from_str(content) {
            Ok(map) => Some(map),
            Err(e) => {
                log::error!("[ICONS] Failed to parse internal icon_config.json: {}", e);
                None
            }
        }
    }
}

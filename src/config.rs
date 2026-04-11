// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 ddavef/KinteLiX retouched-server

use serde::{Deserialize, Serialize};
use std::path::Path;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    #[serde(default = "default_host")]
    pub server_host: String,
    #[serde(default = "default_port")]
    pub server_port: u16,
    #[serde(default = "default_http_port")]
    pub http_port: u16,
    #[serde(default = "default_max_connections")]
    pub max_connections: usize,
    #[serde(default = "default_socket_timeout")]
    pub socket_timeout_secs: f64,
    #[serde(default = "default_buffer_size")]
    pub buffer_size: usize,
    #[serde(default = "default_max_packet_size")]
    pub max_packet_size: usize,
    #[serde(default = "default_log_level")]
    pub log_level: String,
    #[serde(default)]
    pub debug: bool,
    #[serde(default)]
    pub verbose_logging: bool,
    #[serde(default = "default_webrtc_port")]
    pub webrtc_port: u16,
    #[serde(default)]
    pub custom_web_dir: Option<String>,
}

fn default_host() -> String {
    "0.0.0.0".into()
}
fn default_port() -> u16 {
    8088
}
fn default_http_port() -> u16 {
    8080
}
fn default_max_connections() -> usize {
    100
}
fn default_socket_timeout() -> f64 {
    30.0
}
fn default_buffer_size() -> usize {
    4096
}
fn default_max_packet_size() -> usize {
    1024 * 1024
}
fn default_log_level() -> String {
    "INFO".into()
}
fn default_webrtc_port() -> u16 {
    8443
}

impl Default for Config {
    fn default() -> Self {
        Self {
            server_host: default_host(),
            server_port: default_port(),
            http_port: default_http_port(),
            max_connections: default_max_connections(),
            socket_timeout_secs: default_socket_timeout(),
            buffer_size: default_buffer_size(),
            max_packet_size: default_max_packet_size(),
            log_level: default_log_level(),
            debug: false,
            verbose_logging: false,
            webrtc_port: default_webrtc_port(),
            custom_web_dir: None,
        }
    }
}

impl Config {
    pub fn from_file(path: &Path) -> Result<Self, Box<dyn std::error::Error>> {
        let content = std::fs::read_to_string(path)?;
        let cfg: Config = serde_json::from_str(&content)?;
        Ok(cfg)
    }

    pub fn save_to_file(&self, path: &Path) -> Result<(), Box<dyn std::error::Error>> {
        let content = serde_json::to_string_pretty(self)?;
        std::fs::write(path, content)?;
        Ok(())
    }
}

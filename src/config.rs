// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 ddavef/KinteLiX retouched-server

use serde::{Deserialize, Serialize};
use std::path::Path;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    #[serde(default = "default_host")]
    pub server_host: String,

    #[serde(rename = "server_port", default = "default_port")]
    pub custom_registry_port: u16,

    #[serde(rename = "http_port", default = "default_http_port")]
    pub custom_http_port: u16,

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

    #[serde(default = "default_registry_retry_secs")]
    pub registry_retry_secs: u64,

    #[serde(default = "default_true")]
    pub bridge_uses_local_registry: bool,

    #[serde(default = "default_true")]
    pub serves_original_apps: bool,

    #[serde(default)]
    pub bridge_registry_host: String,

    #[serde(default)]
    pub custom_web_dir: Option<String>,

    #[serde(default)]
    pub allow_multiple_instances: bool,

    #[serde(default)]
    pub window_width: Option<u32>,

    #[serde(default)]
    pub window_height: Option<u32>,
}

pub const EXPECTED_REGISTRY_PORT: u16 = 8088;
pub const EXPECTED_HTTP_PORT: u16 = 8080;

fn default_host() -> String {
    "0.0.0.0".into()
}
fn default_port() -> u16 {
    EXPECTED_REGISTRY_PORT
}
fn default_http_port() -> u16 {
    EXPECTED_HTTP_PORT
}
fn default_max_connections() -> usize {
    DEFAULT_MAX_CONNECTIONS
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

pub const DEFAULT_BRIDGE_PORT: u16 = 8443;
pub const DEFAULT_MAX_CONNECTIONS: usize = 100;

fn default_webrtc_port() -> u16 {
    DEFAULT_BRIDGE_PORT
}
fn default_registry_retry_secs() -> u64 {
    5
}
fn default_true() -> bool {
    true
}

impl Default for Config {
    fn default() -> Self {
        Self {
            server_host: default_host(),
            custom_registry_port: default_port(),
            custom_http_port: default_http_port(),
            max_connections: default_max_connections(),
            socket_timeout_secs: default_socket_timeout(),
            buffer_size: default_buffer_size(),
            max_packet_size: default_max_packet_size(),
            log_level: default_log_level(),
            debug: false,
            verbose_logging: false,
            webrtc_port: default_webrtc_port(),
            registry_retry_secs: default_registry_retry_secs(),
            bridge_uses_local_registry: true,
            bridge_registry_host: String::new(),
            serves_original_apps: true,
            custom_web_dir: None,
            allow_multiple_instances: false,
            window_width: None,
            window_height: None,
        }
    }
}

impl Config {
    pub fn registry_port(&self) -> u16 {
        if self.serves_original_apps {
            EXPECTED_REGISTRY_PORT
        } else {
            self.custom_registry_port
        }
    }

    pub fn http_port(&self) -> u16 {
        if self.serves_original_apps {
            EXPECTED_HTTP_PORT
        } else {
            self.custom_http_port
        }
    }

    pub fn custom_ports_apply(&self) -> bool {
        !self.serves_original_apps
    }

    pub fn restore_default_ports(&mut self) {
        self.custom_registry_port = EXPECTED_REGISTRY_PORT;
        self.custom_http_port = EXPECTED_HTTP_PORT;
    }

    pub fn from_file(path: &Path) -> Result<Self, Box<dyn std::error::Error>> {
        let content = std::fs::read_to_string(path)?;
        let mut cfg: Config = serde_json::from_str(&content)?;
        // treat an empty custom web dir as unset so it falls back to the managed dir
        if cfg.custom_web_dir.as_deref() == Some("") {
            cfg.custom_web_dir = None;
        }
        Ok(cfg)
    }

    pub fn save_to_file(&self, path: &Path) -> Result<(), Box<dyn std::error::Error>> {
        let content = serde_json::to_string_pretty(self)?;
        std::fs::write(path, content)?;
        Ok(())
    }
}

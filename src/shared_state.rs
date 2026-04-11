// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 ddavef/KinteLiX retouched-server

use std::collections::{HashMap, VecDeque};
use std::sync::atomic::AtomicBool;
use std::sync::{Arc, Mutex};
use std::time::Instant;

const MAX_LOG_ENTRIES: usize = 2000;

#[derive(Clone, Debug)]
pub struct LogEntry {
    pub timestamp: Instant,
    pub level: log::Level,
    pub message: String,
}

#[derive(Default)]
pub struct LogBuffer {
    entries: VecDeque<LogEntry>,
}

impl LogBuffer {
    pub fn push(&mut self, level: log::Level, message: String) {
        if self.entries.len() >= MAX_LOG_ENTRIES {
            self.entries.pop_front();
        }
        self.entries.push_back(LogEntry {
            timestamp: Instant::now(),
            level,
            message,
        });
    }

    pub fn entries(&self) -> &VecDeque<LogEntry> {
        &self.entries
    }

    pub fn clear(&mut self) {
        self.entries.clear();
    }
}

#[derive(Clone, Debug)]
pub struct ConnectedClient {
    pub device_id: Option<String>,
    pub device_name: Option<String>,
    pub device_type_code: Option<i32>,
    pub addr: String,
    pub connected_at: Instant,
    pub domain: Option<String>,
    pub app_id: Option<String>,
    pub slot_id: Option<i16>,
    pub current_players: Option<i16>,
    pub max_players: Option<i16>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ServerStatus {
    Stopped,
    Starting,
    Running,
    Stopping,
}

impl Default for ServerStatus {
    fn default() -> Self {
        Self::Stopped
    }
}

pub struct SharedState {
    pub server_status: Mutex<ServerStatus>,
    pub server_started_at: Mutex<Option<Instant>>,
    pub connected_clients: Mutex<Vec<ConnectedClient>>,
    pub pending_connections: Mutex<HashMap<String, String>>,
    pub metrics_connections: Mutex<HashMap<String, String>>,
    pub request_server_start: AtomicBool,
    pub request_server_stop: AtomicBool,

    pub log_buffer: Mutex<LogBuffer>,

    pub detected_lan_ip: Mutex<String>,

    pub request_quit: AtomicBool,
}

impl SharedState {
    pub fn new() -> Arc<Self> {
        let lan_ip = local_ip_address::local_ip()
            .map(|ip| ip.to_string())
            .unwrap_or_else(|_| "127.0.0.1".to_string());

        Arc::new(Self {
            server_status: Mutex::new(ServerStatus::Stopped),
            server_started_at: Mutex::new(None),
            connected_clients: Mutex::new(Vec::new()),
            pending_connections: Mutex::new(HashMap::new()),
            metrics_connections: Mutex::new(HashMap::new()),
            request_server_start: AtomicBool::new(false),
            request_server_stop: AtomicBool::new(false),
            log_buffer: Mutex::new(LogBuffer::default()),
            detected_lan_ip: Mutex::new(lan_ip),
            request_quit: AtomicBool::new(false),
        })
    }

    pub fn set_server_status(&self, status: ServerStatus) {
        *self.server_status.lock().unwrap() = status;
    }

    pub fn server_status(&self) -> ServerStatus {
        self.server_status.lock().unwrap().clone()
    }

    pub fn push_log(&self, level: log::Level, message: String) {
        self.log_buffer.lock().unwrap().push(level, message);
    }

    pub fn detected_lan_ip(&self) -> String {
        self.detected_lan_ip.lock().unwrap().clone()
    }

    pub fn set_clients(&self, clients: Vec<ConnectedClient>) {
        *self.connected_clients.lock().unwrap() = clients;
    }

    pub fn clients(&self) -> Vec<ConnectedClient> {
        self.connected_clients.lock().unwrap().clone()
    }

    pub fn metrics_connections(&self) -> HashMap<String, String> {
        self.metrics_connections.lock().unwrap().clone()
    }
}

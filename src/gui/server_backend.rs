// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 ddavef/KinteLiX retouched-server

use crate::ServerCommand;
use crate::config::Config;
use crate::shared_state::{ServerStatus, SharedState};
use core::pin::Pin;
use cxx_qt_lib::QString;
use std::collections::HashMap;
use std::sync::Arc;
use std::sync::{Mutex, OnceLock};
use std::time::Instant;

pub struct BackendInit {
    pub shared: Arc<SharedState>,
    pub server_tx: std::sync::mpsc::SyncSender<ServerCommand>,
    pub config: Mutex<Config>,
    pub config_path: std::path::PathBuf,
    pub data_dir: Option<std::path::PathBuf>,
    pub show_wizard: bool,
}

pub static BACKEND_INIT: OnceLock<BackendInit> = OnceLock::new();

fn device_type_name(code: Option<i32>) -> &'static str {
    match code {
        Some(0) => "Any",
        Some(1) => "Unity",
        Some(2) => "iPhone",
        Some(3) => "Flash",
        Some(4) => "Android",
        Some(5) => "Native",
        Some(6) => "Palm",
        Some(7) => "Server",
        _ => "Unknown",
    }
}

fn app_label(device_type_code: Option<i32>, domain: Option<&str>) -> &'static str {
    let is_game = matches!(device_type_code, Some(1) | Some(3) | Some(5));
    let is_mobile = matches!(device_type_code, Some(0) | Some(2) | Some(4));
    match domain.map(|d| d.to_lowercase()).as_deref() {
        Some("nitrome") if is_game => "Nitrome",
        Some("nitrome") if is_mobile => "Touchy",
        Some("nitrome") => "Nitrome",
        Some(d) if d.starts_with("retouchedflutter") => "Retouched Flutter",
        Some(d) if d.starts_with("retouchedweb") => "Retouched Web",
        Some(d) if d.starts_with("retouched") => "Retouched",
        None if is_game => "Brass Monkey",
        None if is_mobile => "Brass Monkey",
        None => "Brass Monkey",
        Some(_) => "Brass Monkey",
    }
}

fn is_game(code: Option<i32>) -> bool {
    matches!(code, Some(1) | Some(3))
}

fn is_controller(code: Option<i32>) -> bool {
    matches!(code, Some(0) | Some(2) | Some(4) | Some(5))
}

fn device_type_color(code: Option<i32>) -> &'static str {
    match code {
        Some(1) => "#64b4ff",
        Some(3) => "#ff8c32",
        Some(2) => "#c8c8c8",
        Some(4) => "#a0d25a",
        Some(0) => "#00c864",
        Some(7) => "#c8a0ff",
        _ => "#b4b4b4",
    }
}

fn slot_color(slot_id: i16) -> &'static str {
    const COLORS: &[&str] = &[
        "#666666", "#FF6900", "#FED000", "#FF2C9B", "#FF0066", "#D500FF", "#969C00", "#9B96CE",
        "#00CD97", "#009B00", "#00C9FF", "#112F68", "#8AFF00", "#D01300", "#76D061", "#7400FF",
    ];
    COLORS.get(slot_id as usize).copied().unwrap_or("#666666")
}

fn is_retouched_domain(domain: Option<&str>) -> bool {
    domain
        .map(|d| d.to_lowercase().starts_with("retouched"))
        .unwrap_or(false)
}

fn is_in_metrics(device_id: Option<&String>, metrics: &HashMap<String, String>) -> bool {
    if let Some(dev_id) = device_id {
        metrics.contains_key(dev_id) || metrics.values().any(|v| v == dev_id)
    } else {
        false
    }
}

pub struct ServerBackendRust {
    server_status: QString,
    uptime: QString,
    game_count: i32,
    controller_count: i32,
    lan_ip: QString,
}

impl Default for ServerBackendRust {
    fn default() -> Self {
        let ip = local_ip_address::local_ip()
            .map(|ip| ip.to_string())
            .unwrap_or_default();
        Self {
            server_status: QString::from("Stopped"),
            uptime: QString::from("--:--:--"),
            game_count: 0,
            controller_count: 0,
            lan_ip: QString::from(&ip),
        }
    }
}

#[cxx_qt::bridge]
pub mod qobject {
    unsafe extern "C++" {
        include!("cxx-qt-lib/qstring.h");
        type QString = cxx_qt_lib::QString;
    }

    extern "RustQt" {
        #[qobject]
        #[qml_element]
        #[qproperty(QString, server_status)]
        #[qproperty(QString, uptime)]
        #[qproperty(i32, game_count)]
        #[qproperty(i32, controller_count)]
        #[qproperty(QString, lan_ip)]
        type ServerBackend = super::ServerBackendRust;

        #[qinvokable]
        fn start_server(self: Pin<&mut ServerBackend>);

        #[qinvokable]
        fn stop_server(self: Pin<&mut ServerBackend>);

        #[qinvokable]
        fn refresh(self: Pin<&mut ServerBackend>);

        #[qinvokable]
        fn client_data_json(self: &ServerBackend) -> QString;

        #[qinvokable]
        fn log_entries_json(self: &ServerBackend, level_filter: i32) -> QString;

        #[qinvokable]
        fn clear_log(self: &ServerBackend);
    }
}

impl qobject::ServerBackend {
    fn start_server(self: Pin<&mut Self>) {
        if let Some(init) = BACKEND_INIT.get() {
            let config = init.config.lock().unwrap().clone();
            let data_dir = init.data_dir.clone();
            let _ = init
                .server_tx
                .try_send(ServerCommand::Start { config, data_dir });
        }
    }

    fn stop_server(self: Pin<&mut Self>) {
        if let Some(init) = BACKEND_INIT.get() {
            init.shared
                .request_server_stop
                .store(true, std::sync::atomic::Ordering::Relaxed);
        }
    }

    fn refresh(mut self: Pin<&mut Self>) {
        let Some(init) = BACKEND_INIT.get() else {
            return;
        };
        let shared = &init.shared;

        let status_str = match shared.server_status() {
            ServerStatus::Stopped => "Stopped",
            ServerStatus::Starting => "Starting",
            ServerStatus::Running => "Running",
            ServerStatus::Stopping => "Stopping",
        };
        self.as_mut().set_server_status(QString::from(status_str));

        let uptime_str = if let Some(started) = *shared.server_started_at.lock().unwrap() {
            let secs = started.elapsed().as_secs();
            format!(
                "{:02}:{:02}:{:02}",
                secs / 3600,
                (secs % 3600) / 60,
                secs % 60
            )
        } else {
            String::from("--:--:--")
        };
        self.as_mut().set_uptime(QString::from(&uptime_str));

        let clients = shared.clients();

        let games = clients
            .iter()
            .filter(|c| {
                is_game(c.device_type_code)
                    && c.device_id.is_some()
                    && !matches!(c.device_type_code, Some(7))
            })
            .count();
        let controllers = clients
            .iter()
            .filter(|c| is_controller(c.device_type_code) && c.device_id.is_some())
            .count();
        self.as_mut().set_game_count(games as i32);
        self.as_mut().set_controller_count(controllers as i32);

        let ip = local_ip_address::local_ip()
            .map(|ip| ip.to_string())
            .unwrap_or_default();
        self.set_lan_ip(QString::from(&ip));
    }

    fn client_data_json(&self) -> QString {
        let Some(init) = BACKEND_INIT.get() else {
            return QString::from("{}");
        };
        let clients = init.shared.clients();
        let metrics = init.shared.metrics_connections();

        let registered: Vec<_> = clients
            .iter()
            .filter(|c| !matches!(c.device_type_code, Some(7)) && c.device_id.is_some())
            .collect();

        let icons_dir = crate::app_dirs::icons_cache_dir(init.data_dir.as_deref());
        let games: Vec<_> = registered
            .iter()
            .filter(|c| is_game(c.device_type_code))
            .map(|g| {
                let ctrl_names: Vec<&str> = clients
                    .iter()
                    .filter(|c| is_controller(c.device_type_code))
                    .filter(|c| {
                        c.device_id
                            .as_ref()
                            .and_then(|did| metrics.get(did.as_str()))
                            .map_or(false, |game_did| {
                                g.device_id.as_deref() == Some(game_did.as_str())
                            })
                    })
                    .map(|c| c.device_name.as_deref().unwrap_or("Unknown"))
                    .collect();
                let dur = g.connected_at.elapsed().as_secs();
                let icon_url = g.app_id.as_ref().and_then(|id| {
                    let path = icons_dir.join(format!("{}.png", id));
                    if path.exists() {
                        url::Url::from_file_path(&path).ok().map(|u| u.to_string())
                    } else {
                        None
                    }
                });
                serde_json::json!({
                    "name": g.device_name.as_deref().unwrap_or("Unknown"),
                    "typeName": device_type_name(g.device_type_code),
                    "appLabel": app_label(g.device_type_code, g.domain.as_deref()),
                    "typeColor": device_type_color(g.device_type_code),
                    "controllerCount": ctrl_names.len(),
                    "controllerNames": ctrl_names,
                    "connectionTime": format!("{}:{:02}", dur / 60, dur % 60),
                    "flashing": is_in_metrics(g.device_id.as_ref(), &metrics),
                    "isRetouched": is_retouched_domain(g.domain.as_deref()),
                    "iconUrl": icon_url,
                    "slotId": g.slot_id.unwrap_or(0),
                    "slotColor": slot_color(g.slot_id.unwrap_or(0)),
                    "currentPlayers": g.current_players.unwrap_or(0),
                    "maxPlayers": g.max_players.unwrap_or(0),
                })
            })
            .collect();

        let ctrls: Vec<_> = registered
            .iter()
            .filter(|c| is_controller(c.device_type_code))
            .map(|c| {
                let connected_game = c
                    .device_id
                    .as_ref()
                    .and_then(|did| metrics.get(did.as_str()))
                    .and_then(|game_did| {
                        clients
                            .iter()
                            .find(|g| g.device_id.as_deref() == Some(game_did.as_str()))
                            .and_then(|g| g.device_name.as_deref())
                    });
                let dur = c.connected_at.elapsed().as_secs();
                serde_json::json!({
                    "name": c.device_name.as_deref().unwrap_or("Unknown"),
                    "typeName": device_type_name(c.device_type_code),
                    "appLabel": app_label(c.device_type_code, c.domain.as_deref()),
                    "typeColor": device_type_color(c.device_type_code),
                    "addr": c.addr,
                    "connectedGame": connected_game,
                    "connectionTime": format!("{}:{:02}", dur / 60, dur % 60),
                    "flashing": is_in_metrics(c.device_id.as_ref(), &metrics),
                    "isRetouched": is_retouched_domain(c.domain.as_deref()),
                })
            })
            .collect();

        let result = serde_json::json!({ "games": games, "controllers": ctrls });
        QString::from(&result.to_string())
    }

    fn log_entries_json(&self, level_filter: i32) -> QString {
        let Some(init) = BACKEND_INIT.get() else {
            return QString::from("[]");
        };
        let filter = match level_filter {
            1 => log::LevelFilter::Error,
            2 => log::LevelFilter::Warn,
            3 => log::LevelFilter::Info,
            4 => log::LevelFilter::Debug,
            5 => log::LevelFilter::Trace,
            _ => log::LevelFilter::Info,
        };

        let log_buf = init.shared.log_buffer.lock().unwrap();
        let start_time = log_buf
            .entries()
            .front()
            .map(|e| e.timestamp)
            .unwrap_or_else(Instant::now);

        let entries: Vec<_> = log_buf
            .entries()
            .iter()
            .filter(|e| e.level <= filter)
            .map(|e| {
                let elapsed = e.timestamp.duration_since(start_time).as_secs_f64();
                let color = match e.level {
                    log::Level::Error => "#ff5050",
                    log::Level::Warn => "#ffc800",
                    log::Level::Info => "#b4b4b4",
                    log::Level::Debug => "#787878",
                    log::Level::Trace => "#505050",
                };
                serde_json::json!({
                    "time": format!("{:.3}", elapsed),
                    "level": format!("{}", e.level),
                    "message": &e.message,
                    "color": color,
                })
            })
            .collect();

        QString::from(&serde_json::Value::Array(entries).to_string())
    }

    fn clear_log(&self) {
        if let Some(init) = BACKEND_INIT.get() {
            init.shared.log_buffer.lock().unwrap().clear();
        }
    }
}

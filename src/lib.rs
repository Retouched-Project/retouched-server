// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 ddavef/KinteLiX retouched-server

#[cfg(feature = "gui")]
pub enum ServerCommand {
    Start {
        config: config::Config,
        data_dir: Option<std::path::PathBuf>,
    },
}

pub mod app_dirs;
pub mod cert_gen;
pub mod config;
#[cfg(feature = "gui")]
pub mod gui;
#[cfg(feature = "gui")]
pub mod gui_logger;
pub mod http_server;
pub mod icon_cache;
pub mod server;
pub mod setup;
pub mod shared_state;
pub mod touchy_patcher;
pub mod web_app_server;
pub mod web_manager;
pub mod webrtc_bridge;

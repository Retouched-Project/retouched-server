// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 ddavef/KinteLiX retouched-server

pub mod patcher_backend;
pub mod server_backend;
pub mod settings_backend;
pub mod web_app_backend;
pub mod wizard_backend;

use std::sync::Arc;
use std::sync::Mutex;
use std::sync::atomic::Ordering;

use crate::config::Config;
use crate::shared_state::{ServerStatus, SharedState};
use server_backend::{BACKEND_INIT, BackendInit};

unsafe extern "C" {
    fn runQtApp(app_name: *const std::ffi::c_char, app_version: *const std::ffi::c_char) -> i32;
}

#[unsafe(no_mangle)]
extern "C" fn trayServerToggle() {
    if let Some(init) = BACKEND_INIT.get() {
        match init.shared.server_status() {
            ServerStatus::Stopped => {
                let config = init.config.lock().unwrap().clone();
                let data_dir = init.data_dir.clone();
                let _ = init
                    .server_tx
                    .try_send(crate::ServerCommand::Start { config, data_dir });
            }
            ServerStatus::Running => {
                init.shared
                    .request_server_stop
                    .store(true, Ordering::Relaxed);
            }
            _ => {}
        }
    }
}

#[unsafe(no_mangle)]
extern "C" fn trayIsServerRunning() -> bool {
    BACKEND_INIT
        .get()
        .map(|init| init.shared.server_status() == ServerStatus::Running)
        .unwrap_or(false)
}

#[unsafe(no_mangle)]
extern "C" fn trayQuitRequested() {
    if let Some(init) = BACKEND_INIT.get() {
        init.shared.request_quit.store(true, Ordering::Relaxed);
    }
    web_app_backend::kill_all_processes();
}

pub fn run_qt_app(
    config: Config,
    config_path: std::path::PathBuf,
    data_dir: Option<std::path::PathBuf>,
    show_wizard: bool,
    shared: Arc<SharedState>,
    server_tx: std::sync::mpsc::SyncSender<crate::ServerCommand>,
) {
    BACKEND_INIT
        .set(BackendInit {
            shared,
            server_tx,
            config: Mutex::new(config),
            config_path: config_path.clone(),
            data_dir,
            show_wizard,
        })
        .ok()
        .expect("BackendInit already set");

    let app_name = std::ffi::CString::new("Retouched Server").unwrap();
    let app_version = std::ffi::CString::new(env!("CARGO_PKG_VERSION")).unwrap();
    let _exit_code = unsafe { runQtApp(app_name.as_ptr(), app_version.as_ptr()) };

    if let Some(init) = BACKEND_INIT.get() {
        if let Some(parent) = init.config_path.parent() {
            let _ = std::fs::create_dir_all(parent);
        }
        if let Err(e) = init.config.lock().unwrap().save_to_file(&init.config_path) {
            log::error!("Failed to save config: {}", e);
        }
    }
}

pub async fn run_server_task(
    config: Config,
    data_dir: Option<std::path::PathBuf>,
    shared: Arc<SharedState>,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let data_dir_cached = crate::app_dirs::app_data_dir(data_dir.as_deref());
    let icon_cache = crate::icon_cache::IconCache::new(data_dir.as_deref());
    icon_cache.download_icons().await;

    let http_state = Arc::new(crate::http_server::HttpServerState {
        icon_cache,
        shared: Some(shared.clone()),
        data_dir: data_dir_cached,
    });
    let http_router = crate::http_server::build_router(http_state);
    let http_addr = format!("{}:{}", config.server_host, config.http_port);
    let http_listener = tokio::net::TcpListener::bind(&http_addr).await?;
    log::info!("HTTP server listening on {}", http_addr);

    let shared_for_http = shared.clone();
    let http_handle = tokio::spawn(async move {
        let server = axum::serve(
            http_listener,
            http_router.into_make_service_with_connect_info::<std::net::SocketAddr>(),
        )
        .with_graceful_shutdown(async move {
            loop {
                tokio::time::sleep(std::time::Duration::from_millis(200)).await;
                if shared_for_http.request_server_stop.load(Ordering::Relaxed)
                    || shared_for_http.request_quit.load(Ordering::Relaxed)
                {
                    break;
                }
            }
        });
        if let Err(e) = server.await {
            log::error!("HTTP server error: {}", e);
        }
    });

    let server = crate::server::Server::with_shared(config, Some(shared.clone()));
    let shutdown_tx = server.shutdown_handle();

    let shared_for_watch = shared.clone();
    tokio::spawn(async move {
        loop {
            tokio::time::sleep(std::time::Duration::from_millis(200)).await;
            if shared_for_watch.request_server_stop.load(Ordering::Relaxed)
                || shared_for_watch.request_quit.load(Ordering::Relaxed)
            {
                let _ = shutdown_tx.send(());
                break;
            }
        }
    });

    shared.set_server_status(ServerStatus::Running);
    *shared.server_started_at.lock().unwrap() = Some(std::time::Instant::now());

    server.run().await?;

    let _ = http_handle.await;

    shared.request_server_stop.store(false, Ordering::Relaxed);
    Ok(())
}

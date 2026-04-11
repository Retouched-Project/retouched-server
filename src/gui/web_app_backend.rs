// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 ddavef/KinteLiX retouched-server

use core::pin::Pin;
use cxx_qt_lib::QString;
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, LazyLock, Mutex};

use crate::gui::server_backend::BACKEND_INIT;

const WEB_APP_PORT: u16 = 8089;

#[derive(Clone, Copy, PartialEq)]
enum BridgeStatus {
    Stopped,
    Starting,
    Running,
    Error,
}

#[derive(Clone, Copy, PartialEq)]
enum WebAppStatus {
    NotDownloaded,
    Downloading,
    Stopped,
    Running,
    Error,
}

struct WebAppInternalState {
    initialized: bool,

    bridge_status: Arc<Mutex<BridgeStatus>>,
    bridge_error: Arc<Mutex<String>>,
    bridge_stop_flag: Arc<AtomicBool>,

    web_app_status: Arc<Mutex<WebAppStatus>>,
    web_app_error: Arc<Mutex<String>>,
    web_app_handle: Option<axum_server::Handle>,

    current_version: Arc<Mutex<Option<String>>>,
    custom_web_dir: String,
    data_dir: PathBuf,
    http_port: u16,
    webrtc_port: u16,
    server_port: u16,
    lan_ip: String,
}

impl WebAppInternalState {
    fn effective_web_dir(&self) -> PathBuf {
        let base_dir = if !self.custom_web_dir.is_empty() {
            PathBuf::from(&self.custom_web_dir)
        } else {
            crate::web_manager::web_app_dir(&self.data_dir)
        };

        if base_dir.join("dist").exists() {
            base_dir.join("dist")
        } else {
            base_dir
        }
    }
}

static WEB_STATE: LazyLock<Mutex<WebAppInternalState>> = LazyLock::new(|| {
    Mutex::new(WebAppInternalState {
        initialized: false,
        bridge_status: Arc::new(Mutex::new(BridgeStatus::Stopped)),
        bridge_error: Arc::new(Mutex::new(String::new())),
        bridge_stop_flag: Arc::new(AtomicBool::new(false)),
        web_app_status: Arc::new(Mutex::new(WebAppStatus::NotDownloaded)),
        web_app_error: Arc::new(Mutex::new(String::new())),
        web_app_handle: None,
        current_version: Arc::new(Mutex::new(None)),
        custom_web_dir: String::new(),
        data_dir: PathBuf::new(),
        http_port: 8080,
        webrtc_port: 8443,
        server_port: 8088,
        lan_ip: "127.0.0.1".to_string(),
    })
});

fn ensure_initialized(state: &mut WebAppInternalState) {
    if state.initialized {
        return;
    }
    if let Some(init) = BACKEND_INIT.get() {
        let config = init.config.lock().unwrap();
        state.data_dir = crate::app_dirs::app_data_dir(init.data_dir.as_deref());
        state.http_port = config.http_port;
        state.webrtc_port = config.webrtc_port;
        state.server_port = config.server_port;
        state.custom_web_dir = config.custom_web_dir.clone().unwrap_or_default();
        drop(config);

        state.lan_ip = local_ip_address::local_ip()
            .map(|ip| ip.to_string())
            .unwrap_or_else(|_| "127.0.0.1".to_string());

        let effective_dir = state.effective_web_dir();
        if effective_dir.join("index.html").exists() {
            *state.web_app_status.lock().unwrap() = WebAppStatus::Stopped;
        }
        state.initialized = true;
    }
}

fn path_to_file_url(path: &std::path::Path) -> String {
    #[cfg(windows)]
    {
        format!("file:///{}", path.display().to_string().replace('\\', "/"))
    }
    #[cfg(not(windows))]
    {
        format!("file://{}", path.display())
    }
}

fn generate_qr_file(data: &str, filename: &str, data_dir: &std::path::Path) -> Option<String> {
    use qrcode::QrCode;
    let code = QrCode::new(data.as_bytes()).ok()?;
    let img = code.render::<image::Luma<u8>>().build();
    let qr_dir = data_dir.join("qr");
    std::fs::create_dir_all(&qr_dir).ok()?;
    let path = qr_dir.join(filename);
    img.save(&path).ok()?;
    Some(path_to_file_url(&path))
}

pub struct WebAppBackendRust {
    bridge_status: QString,
    bridge_port: QString,
    lan_ip: QString,
    bridge_error: QString,
    web_app_status: QString,
    web_app_version: QString,
    web_url: QString,
    onboard_url: QString,
    custom_web_dir: QString,
    default_web_dir: QString,
    has_package_json: bool,
    web_app_error: QString,
    qr_web_url: QString,
    qr_onboard_url: QString,
}

impl Default for WebAppBackendRust {
    fn default() -> Self {
        Self {
            bridge_status: QString::from("Stopped"),
            bridge_port: QString::from("8443"),
            lan_ip: QString::from(""),
            bridge_error: QString::from(""),
            web_app_status: QString::from("Not found"),
            web_app_version: QString::from(""),
            web_url: QString::from(""),
            onboard_url: QString::from(""),
            custom_web_dir: QString::from(""),
            default_web_dir: QString::from(""),
            has_package_json: false,
            web_app_error: QString::from(""),
            qr_web_url: QString::from(""),
            qr_onboard_url: QString::from(""),
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
        #[qproperty(QString, bridge_status)]
        #[qproperty(QString, bridge_port)]
        #[qproperty(QString, lan_ip)]
        #[qproperty(QString, bridge_error)]
        #[qproperty(QString, web_app_status)]
        #[qproperty(QString, web_app_version)]
        #[qproperty(QString, web_url)]
        #[qproperty(QString, onboard_url)]
        #[qproperty(QString, custom_web_dir)]
        #[qproperty(QString, default_web_dir)]
        #[qproperty(bool, has_package_json)]
        #[qproperty(QString, web_app_error)]
        #[qproperty(QString, qr_web_url)]
        #[qproperty(QString, qr_onboard_url)]
        type WebAppBackend = super::WebAppBackendRust;

        #[qinvokable]
        fn refresh(self: Pin<&mut WebAppBackend>);

        #[qinvokable]
        fn start_bridge(self: Pin<&mut WebAppBackend>);

        #[qinvokable]
        fn stop_bridge(self: Pin<&mut WebAppBackend>);

        #[qinvokable]
        fn start_web_app(self: Pin<&mut WebAppBackend>);

        #[qinvokable]
        fn stop_web_app(self: Pin<&mut WebAppBackend>);

        #[qinvokable]
        fn download_release(self: Pin<&mut WebAppBackend>);

        #[qinvokable]
        fn set_custom_dir(self: Pin<&mut WebAppBackend>, dir: QString);

        #[qinvokable]
        fn clear_custom_dir(self: Pin<&mut WebAppBackend>);

        #[qinvokable]
        fn set_bridge_port_value(self: Pin<&mut WebAppBackend>, port: QString);

        #[qinvokable]
        fn set_lan_ip_value(self: Pin<&mut WebAppBackend>, ip: QString);

        #[qinvokable]
        fn kill_all(self: Pin<&mut WebAppBackend>);
    }
}

impl qobject::WebAppBackend {
    fn refresh(mut self: Pin<&mut Self>) {
        let mut state = WEB_STATE.lock().unwrap();
        ensure_initialized(&mut state);

        let bridge_st = *state.bridge_status.lock().unwrap();
        let bridge_str = match bridge_st {
            BridgeStatus::Stopped => "Stopped",
            BridgeStatus::Starting => "Starting",
            BridgeStatus::Running => "Running",
            BridgeStatus::Error => "Error",
        };
        self.as_mut().set_bridge_status(QString::from(bridge_str));

        let bridge_err = state.bridge_error.lock().unwrap().clone();
        self.as_mut().set_bridge_error(QString::from(&bridge_err));

        let has_pkg = state.effective_web_dir().join("index.html").exists();
        {
            let mut status = state.web_app_status.lock().unwrap();
            if has_pkg && *status == WebAppStatus::NotDownloaded {
                *status = WebAppStatus::Stopped;
            } else if !has_pkg && *status == WebAppStatus::Stopped {
                *status = WebAppStatus::NotDownloaded;
            }
        }

        let web_st = *state.web_app_status.lock().unwrap();
        let web_str = match web_st {
            WebAppStatus::NotDownloaded => "Not found",
            WebAppStatus::Downloading => "Downloading...",
            WebAppStatus::Stopped => "Ready",
            WebAppStatus::Running => "Running",
            WebAppStatus::Error => "Error",
        };
        self.as_mut().set_web_app_status(QString::from(web_str));

        let web_err = state.web_app_error.lock().unwrap().clone();
        self.as_mut().set_web_app_error(QString::from(&web_err));

        let ver = state
            .current_version
            .lock()
            .unwrap()
            .clone()
            .unwrap_or_default();
        self.as_mut().set_web_app_version(QString::from(&ver));

        let has_pkg = state.effective_web_dir().join("index.html").exists();
        self.as_mut().set_has_package_json(has_pkg);

        let default_dir = crate::web_manager::web_app_dir(&state.data_dir);
        self.as_mut().set_default_web_dir(QString::from(
            &default_dir.display().to_string().replace('\\', "/"),
        ));

        let lan_ip = if let Some(init) = BACKEND_INIT.get() {
            init.shared.detected_lan_ip()
        } else {
            "127.0.0.1".to_string()
        };

        if web_st == WebAppStatus::Running {
            let web_url = format!("https://{}:{}", lan_ip, WEB_APP_PORT);
            let onboard_url = format!("http://{}:{}/onboard", lan_ip, state.http_port);
            self.as_mut().set_web_url(QString::from(&web_url));
            self.as_mut().set_onboard_url(QString::from(&onboard_url));

            if let Some(qr_path) = generate_qr_file(&web_url, "web.png", &state.data_dir) {
                self.as_mut().set_qr_web_url(QString::from(&qr_path));
            }
            if let Some(qr_path) = generate_qr_file(&onboard_url, "onboard.png", &state.data_dir) {
                self.as_mut().set_qr_onboard_url(QString::from(&qr_path));
            }
        } else {
            self.as_mut().set_web_url(QString::from(""));
            self.as_mut().set_onboard_url(QString::from(""));
        }

        self.set_custom_web_dir(QString::from(&state.custom_web_dir));
    }

    fn start_bridge(self: Pin<&mut Self>) {
        let bridge_port_str = self.bridge_port().to_string();
        let bridge_port: u16 = bridge_port_str.parse().unwrap_or(8443);

        let mut state = WEB_STATE.lock().unwrap();
        ensure_initialized(&mut state);

        let lan_ip = if let Some(init) = BACKEND_INIT.get() {
            init.shared.detected_lan_ip()
        } else {
            return;
        };

        let cert_dir = state.data_dir.join("certs");
        if let Err(e) = crate::cert_gen::ensure_cert(&cert_dir, &lan_ip) {
            *state.bridge_error.lock().unwrap() = e.to_string();
            *state.bridge_status.lock().unwrap() = BridgeStatus::Error;
            return;
        }

        *state.bridge_status.lock().unwrap() = BridgeStatus::Starting;
        state.bridge_stop_flag.store(false, Ordering::Relaxed);

        let status = state.bridge_status.clone();
        let error = state.bridge_error.clone();
        let stop_flag = state.bridge_stop_flag.clone();
        let server_port = state.server_port;
        let http_port = state.http_port;

        drop(state);

        std::thread::spawn(move || {
            let rt = tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()
                .unwrap();

            rt.block_on(async {
                match crate::webrtc_bridge::WebRTCBridge::start(
                    bridge_port,
                    server_port,
                    http_port,
                    lan_ip,
                    &cert_dir,
                )
                .await
                {
                    Ok(bridge) => {
                        *status.lock().unwrap() = BridgeStatus::Running;
                        loop {
                            tokio::time::sleep(std::time::Duration::from_millis(500)).await;
                            if stop_flag.load(Ordering::Relaxed) {
                                bridge.shutdown();
                                break;
                            }
                        }
                    }
                    Err(e) => {
                        *error.lock().unwrap() = e.to_string();
                        *status.lock().unwrap() = BridgeStatus::Error;
                    }
                }
            });
        });
    }

    fn stop_bridge(self: Pin<&mut Self>) {
        let state = WEB_STATE.lock().unwrap();
        state.bridge_stop_flag.store(true, Ordering::Relaxed);
        *state.bridge_status.lock().unwrap() = BridgeStatus::Stopped;
    }

    fn start_web_app(self: Pin<&mut Self>) {
        let mut state = WEB_STATE.lock().unwrap();
        ensure_initialized(&mut state);

        let web_dir = state.effective_web_dir();
        if !web_dir.join("index.html").exists() {
            *state.web_app_error.lock().unwrap() =
                "No index.html found. Please download the release.".to_string();
            *state.web_app_status.lock().unwrap() = WebAppStatus::Error;
            return;
        }

        *state.web_app_error.lock().unwrap() = String::new();

        let lan_ip = if let Some(init) = BACKEND_INIT.get() {
            init.shared.detected_lan_ip()
        } else {
            "127.0.0.1".to_string()
        };

        let cert_dir = state.data_dir.join("certs");
        if let Err(e) = crate::cert_gen::ensure_cert(&cert_dir, &lan_ip) {
            *state.web_app_error.lock().unwrap() = format!("Cert generation failed: {}", e);
            *state.web_app_status.lock().unwrap() = WebAppStatus::Error;
            return;
        }

        let handle = axum_server::Handle::new();
        state.web_app_handle = Some(handle.clone());
        *state.web_app_status.lock().unwrap() = WebAppStatus::Running;

        let status = state.web_app_status.clone();
        let error = state.web_app_error.clone();
        let data_dir = state.data_dir.clone();
        let webrtc_port = state.webrtc_port;

        log::info!(
            "Starting Axum static server on port {} serving {}",
            WEB_APP_PORT,
            web_dir.display().to_string().replace('\\', "/")
        );

        drop(state);

        std::thread::spawn(move || {
            let rt = tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()
                .unwrap();

            rt.block_on(async {
                let cert_dir = data_dir.join("certs");
                if let Err(e) = crate::web_app_server::run_web_app_server(
                    web_dir.clone(),
                    webrtc_port,
                    cert_dir,
                    handle,
                )
                .await
                {
                    *error.lock().unwrap() = e;
                    *status.lock().unwrap() = WebAppStatus::Error;
                } else {
                    let mut lock = status.lock().unwrap();
                    if *lock == WebAppStatus::Running {
                        *lock = WebAppStatus::Stopped;
                    }
                }
            });
        });
    }

    fn stop_web_app(self: Pin<&mut Self>) {
        let mut state = WEB_STATE.lock().unwrap();
        stop_web_app_internal(&mut state);
    }

    fn download_release(self: Pin<&mut Self>) {
        let mut state = WEB_STATE.lock().unwrap();
        ensure_initialized(&mut state);

        *state.web_app_status.lock().unwrap() = WebAppStatus::Downloading;
        *state.web_app_error.lock().unwrap() = String::new();

        let data_dir = state.data_dir.clone();
        let status = state.web_app_status.clone();
        let error = state.web_app_error.clone();
        let version = state.current_version.clone();

        std::thread::spawn(move || {
            let target = crate::web_manager::web_app_dir(&data_dir);
            match crate::web_manager::download_web_app(&target) {
                Ok(ver) => {
                    *version.lock().unwrap() = Some(ver);
                    *status.lock().unwrap() = WebAppStatus::Stopped;
                    log::info!("retouched_web downloaded successfully");
                }
                Err(e) => {
                    *error.lock().unwrap() = format!("Download failed: {}", e);
                    *status.lock().unwrap() = WebAppStatus::Error;
                    log::error!("Failed to download retouched_web: {}", e);
                }
            }
        });
    }

    fn set_custom_dir(self: Pin<&mut Self>, dir: QString) {
        let mut state = WEB_STATE.lock().unwrap();
        state.custom_web_dir = dir.to_string();
        if let Some(init) = BACKEND_INIT.get() {
            init.config.lock().unwrap().custom_web_dir = if state.custom_web_dir.is_empty() {
                None
            } else {
                Some(state.custom_web_dir.clone())
            };
        }
    }

    fn clear_custom_dir(self: Pin<&mut Self>) {
        let mut state = WEB_STATE.lock().unwrap();
        state.custom_web_dir.clear();
        if let Some(init) = BACKEND_INIT.get() {
            init.config.lock().unwrap().custom_web_dir = None;
        }
        drop(state);
        self.set_custom_web_dir(QString::from(""));
    }

    fn set_bridge_port_value(self: Pin<&mut Self>, port: QString) {
        self.set_bridge_port(port);
    }

    fn set_lan_ip_value(self: Pin<&mut Self>, ip: QString) {
        self.set_lan_ip(ip);
    }

    fn kill_all(self: Pin<&mut Self>) {
        let mut state = WEB_STATE.lock().unwrap();
        state.bridge_stop_flag.store(true, Ordering::Relaxed);
        stop_web_app_internal(&mut state);
    }
}

pub fn kill_all_processes() {
    let mut state = WEB_STATE.lock().unwrap();
    state.bridge_stop_flag.store(true, Ordering::Relaxed);
    stop_web_app_internal(&mut state);
}

fn stop_web_app_internal(state: &mut WebAppInternalState) {
    if let Some(handle) = state.web_app_handle.take() {
        handle.graceful_shutdown(Some(std::time::Duration::from_secs(2)));
    }
    *state.web_app_status.lock().unwrap() = WebAppStatus::Stopped;
}

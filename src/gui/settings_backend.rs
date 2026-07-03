// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 ddavef/KinteLiX retouched-server

use core::pin::Pin;
use cxx_qt_lib::QString;
use std::sync::atomic::{AtomicBool, AtomicU8, Ordering};

// cached policy redirect status: 0 unknown, 1 active, 2 inactive
static POLICY_STATUS: AtomicU8 = AtomicU8::new(0);
static POLICY_PROBING: AtomicBool = AtomicBool::new(false);

fn server_running() -> bool {
    crate::gui::server_backend::BACKEND_INIT
        .get()
        .map(|init| init.shared.server_status() == crate::shared_state::ServerStatus::Running)
        .unwrap_or(false)
}

// probe off-thread on demand - the probe touches the server so it is not run on a timer
fn spawn_policy_probe() {
    if POLICY_PROBING.swap(true, Ordering::AcqRel) {
        return;
    }
    let target = redirect_target_port();
    std::thread::spawn(move || {
        let code = match crate::setup::port_redirect::probe_status(target) {
            crate::setup::port_redirect::RedirectStatus::Active => 1,
            crate::setup::port_redirect::RedirectStatus::Inactive => 2,
            crate::setup::port_redirect::RedirectStatus::Unknown => 0,
        };
        POLICY_STATUS.store(code, Ordering::Relaxed);
        POLICY_PROBING.store(false, Ordering::Release);
    });
}

pub struct SettingsBackendRust {
    trust_entries_json: QString,
    new_trust_directory: QString,
    hosts_redirect_ip: QString,
    hosts_status_json: QString,
    firewall_backend: QString,
    redirect_backend: QString,
    policy_redirect_status: QString,
}

impl Default for SettingsBackendRust {
    fn default() -> Self {
        Self {
            trust_entries_json: QString::from("[]"),
            new_trust_directory: QString::from(""),
            hosts_redirect_ip: QString::from("127.0.0.1"),
            hosts_status_json: QString::from("[]"),
            firewall_backend: QString::from(""),
            redirect_backend: QString::from(""),
            policy_redirect_status: QString::from("unknown"),
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
        #[qproperty(QString, trust_entries_json)]
        #[qproperty(QString, new_trust_directory)]
        #[qproperty(QString, hosts_redirect_ip)]
        #[qproperty(QString, hosts_status_json)]
        #[qproperty(QString, firewall_backend)]
        #[qproperty(QString, redirect_backend)]
        #[qproperty(QString, policy_redirect_status)]
        type SettingsBackend = super::SettingsBackendRust;

        #[qinvokable]
        fn refresh(self: Pin<&mut SettingsBackend>);

        #[qinvokable]
        fn add_trust_dir(self: Pin<&mut SettingsBackend>);

        #[qinvokable]
        fn remove_trust_dir(self: Pin<&mut SettingsBackend>, index: i32);

        #[qinvokable]
        fn remove_all_trust(self: Pin<&mut SettingsBackend>);

        #[qinvokable]
        fn set_new_trust_dir(self: Pin<&mut SettingsBackend>, dir: QString);

        #[qinvokable]
        fn set_hosts_ip(self: Pin<&mut SettingsBackend>, ip: QString);

        #[qinvokable]
        fn apply_hosts_redirect(self: Pin<&mut SettingsBackend>);

        #[qinvokable]
        fn remove_hosts_redirect(self: Pin<&mut SettingsBackend>);

        #[qinvokable]
        fn open_ports(self: &SettingsBackend);

        #[qinvokable]
        fn close_ports(self: &SettingsBackend);

        #[qinvokable]
        fn apply_policy_redirect(self: &SettingsBackend);

        #[qinvokable]
        fn remove_policy_redirect(self: &SettingsBackend);
    }
}

impl qobject::SettingsBackend {
    fn refresh(mut self: Pin<&mut Self>) {
        let trust_dirs = crate::setup::flash_trust::read_trusted_dirs();
        let trust_json: Vec<_> = trust_dirs.iter().map(|d| serde_json::json!(d)).collect();
        self.as_mut().set_trust_entries_json(QString::from(
            &serde_json::Value::Array(trust_json).to_string(),
        ));

        let entries = crate::setup::hosts::check_hosts_entries();
        let hosts_json: Vec<_> = entries
            .iter()
            .map(|(domain, state)| match state {
                crate::setup::hosts::HostEntryState::Present(ip) => {
                    serde_json::json!({"domain": domain, "status": "ok", "ip": ip})
                }
                crate::setup::hosts::HostEntryState::Missing => {
                    serde_json::json!({"domain": domain, "status": "missing"})
                }
            })
            .collect();
        self.as_mut().set_hosts_status_json(QString::from(
            &serde_json::Value::Array(hosts_json).to_string(),
        ));

        let backend = crate::setup::firewall::detect_backend();
        self.as_mut()
            .set_firewall_backend(QString::from(backend.name()));

        let redirect = crate::setup::port_redirect::detect_backend();
        self.as_mut()
            .set_redirect_backend(QString::from(redirect.name()));

        // an unknown status resolves once the server is running, enabling while
        // the server is stopped leaves it unknown until then
        if server_running() && POLICY_STATUS.load(Ordering::Relaxed) == 0 {
            spawn_policy_probe();
        }

        let status = match POLICY_STATUS.load(Ordering::Relaxed) {
            1 => "active",
            2 => "inactive",
            _ => "unknown",
        };
        self.set_policy_redirect_status(QString::from(status));
    }

    fn add_trust_dir(self: Pin<&mut Self>) {
        let dir = self.as_ref().new_trust_directory().to_string();
        if dir.is_empty() {
            return;
        }
        match crate::setup::flash_trust::add_trusted_dir(&dir) {
            Ok(()) => {
                log::info!("Added trust directory: {}", dir);
                self.set_new_trust_directory(QString::from(""));
            }
            Err(e) => log::error!("Failed to add trust directory: {}", e),
        }
    }

    fn remove_trust_dir(self: Pin<&mut Self>, index: i32) {
        let dirs = crate::setup::flash_trust::read_trusted_dirs();
        if let Some(dir) = dirs.get(index as usize) {
            match crate::setup::flash_trust::remove_trusted_dir(dir) {
                Ok(()) => log::info!("Removed trust directory: {}", dir),
                Err(e) => log::error!("Failed to remove trust directory: {}", e),
            }
        }
    }

    fn remove_all_trust(self: Pin<&mut Self>) {
        match crate::setup::flash_trust::remove_trust_config() {
            Ok(()) => log::info!("Flash trust config removed"),
            Err(e) => log::error!("Failed to remove trust config: {}", e),
        }
    }

    fn set_new_trust_dir(self: Pin<&mut Self>, dir: QString) {
        let native = crate::path_util::to_native_path(&dir.to_string());
        self.set_new_trust_directory(QString::from(&native.to_string_lossy() as &str));
    }

    fn set_hosts_ip(self: Pin<&mut Self>, ip: QString) {
        self.set_hosts_redirect_ip(ip);
    }

    fn apply_hosts_redirect(self: Pin<&mut Self>) {
        let ip = self.hosts_redirect_ip().to_string();
        match crate::setup::hosts::apply_hosts_entries(&ip) {
            Ok(()) => log::info!("Hosts entries applied for {}", ip),
            Err(e) => log::error!("Failed to apply hosts: {}", e),
        }
    }

    fn remove_hosts_redirect(self: Pin<&mut Self>) {
        match crate::setup::hosts::remove_hosts_entries() {
            Ok(()) => log::info!("Hosts entries removed"),
            Err(e) => log::error!("Failed to remove hosts: {}", e),
        }
    }

    fn open_ports(&self) {
        let backend = crate::setup::firewall::detect_backend();
        match crate::setup::firewall::open_ports(&backend) {
            Ok(()) => log::info!("Firewall ports opened"),
            Err(e) => log::error!("Failed to open ports: {}", e),
        }
    }

    fn close_ports(&self) {
        let backend = crate::setup::firewall::detect_backend();
        match crate::setup::firewall::close_ports(&backend) {
            Ok(()) => log::info!("Firewall ports closed"),
            Err(e) => log::error!("Failed to close ports: {}", e),
        }
    }

    fn apply_policy_redirect(&self) {
        let backend = crate::setup::port_redirect::detect_backend();
        let port = redirect_target_port();
        match crate::setup::port_redirect::apply(&backend, port) {
            Ok(()) => log::info!("Policy port redirect applied (843 -> {})", port),
            Err(e) => log::error!("Failed to apply policy redirect: {}", e),
        }
        // probe now if the server is up, otherwise show unknown until it starts
        if server_running() {
            spawn_policy_probe();
        } else {
            POLICY_STATUS.store(0, Ordering::Relaxed);
        }
    }

    fn remove_policy_redirect(&self) {
        let backend = crate::setup::port_redirect::detect_backend();
        let port = redirect_target_port();
        match crate::setup::port_redirect::remove(&backend, port) {
            Ok(()) => log::info!("Policy port redirect removed"),
            Err(e) => log::error!("Failed to remove policy redirect: {}", e),
        }
        // disabling makes it inactive, force the status instead of probing again
        POLICY_STATUS.store(2, Ordering::Relaxed);
    }
}

fn redirect_target_port() -> u16 {
    crate::gui::server_backend::BACKEND_INIT
        .get()
        .map(|init| init.config.lock().unwrap().server_port)
        .unwrap_or(8088)
}

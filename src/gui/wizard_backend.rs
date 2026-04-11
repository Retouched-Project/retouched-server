// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 ddavef/KinteLiX retouched-server

use core::pin::Pin;
use cxx_qt_lib::QString;

use crate::gui::server_backend::BACKEND_INIT;

pub struct WizardBackendRust {
    active: bool,
    current_page: i32,
    games_directory: QString,
    trust_written: bool,
    hosts_redirect_ip: QString,
    hosts_already_configured: bool,
    firewall_opened: bool,
    firewall_backend_name: QString,
    trust_entries_json: QString,
    hosts_status_json: QString,
}

impl Default for WizardBackendRust {
    fn default() -> Self {
        let show = BACKEND_INIT
            .get()
            .map(|init| init.show_wizard)
            .unwrap_or(false);
        Self {
            active: show,
            current_page: 0,
            games_directory: QString::from(""),
            trust_written: false,
            hosts_redirect_ip: QString::from("127.0.0.1"),
            hosts_already_configured: false,
            firewall_opened: false,
            firewall_backend_name: QString::from(""),
            trust_entries_json: QString::from("[]"),
            hosts_status_json: QString::from("[]"),
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
        #[qproperty(bool, active)]
        #[qproperty(i32, current_page)]
        #[qproperty(QString, games_directory)]
        #[qproperty(bool, trust_written)]
        #[qproperty(QString, hosts_redirect_ip)]
        #[qproperty(bool, hosts_already_configured)]
        #[qproperty(bool, firewall_opened)]
        #[qproperty(QString, firewall_backend_name)]
        #[qproperty(QString, trust_entries_json)]
        #[qproperty(QString, hosts_status_json)]
        type WizardBackend = super::WizardBackendRust;

        #[qinvokable]
        fn refresh(self: Pin<&mut WizardBackend>);

        #[qinvokable]
        fn next_page(self: Pin<&mut WizardBackend>);

        #[qinvokable]
        fn prev_page(self: Pin<&mut WizardBackend>);

        #[qinvokable]
        fn finish(self: Pin<&mut WizardBackend>);

        #[qinvokable]
        fn set_games_dir(self: Pin<&mut WizardBackend>, dir: QString);

        #[qinvokable]
        fn write_trust_config(self: Pin<&mut WizardBackend>);

        #[qinvokable]
        fn set_hosts_ip_value(self: Pin<&mut WizardBackend>, ip: QString);

        #[qinvokable]
        fn apply_hosts(self: Pin<&mut WizardBackend>);

        #[qinvokable]
        fn open_firewall_ports(self: Pin<&mut WizardBackend>);
    }
}

impl qobject::WizardBackend {
    fn refresh(mut self: Pin<&mut Self>) {
        let existing = crate::setup::flash_trust::read_trusted_dirs();
        let trust_json: Vec<_> = existing.iter().map(|d| serde_json::json!(d)).collect();
        self.as_mut().set_trust_entries_json(QString::from(
            &serde_json::Value::Array(trust_json).to_string(),
        ));

        let entries = crate::setup::hosts::check_hosts_entries();
        let all_present = entries
            .iter()
            .all(|(_, s)| matches!(s, crate::setup::hosts::HostEntryState::Present(_)));
        self.as_mut().set_hosts_already_configured(all_present);

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
        self.set_firewall_backend_name(QString::from(backend.name()));
    }

    fn next_page(self: Pin<&mut Self>) {
        let page = *self.as_ref().current_page();
        if page < 4 {
            self.set_current_page(page + 1);
        }
    }

    fn prev_page(self: Pin<&mut Self>) {
        let page = *self.as_ref().current_page();
        if page > 0 {
            self.set_current_page(page - 1);
        }
    }

    fn finish(self: Pin<&mut Self>) {
        self.set_active(false);
    }

    fn set_games_dir(self: Pin<&mut Self>, dir: QString) {
        self.set_games_directory(dir);
    }

    fn write_trust_config(self: Pin<&mut Self>) {
        let dir = self.as_ref().games_directory().to_string();
        if dir.is_empty() {
            return;
        }
        match crate::setup::flash_trust::write_trust_config(&dir) {
            Ok(()) => {
                self.set_trust_written(true);
                log::info!("Flash trust config written for {}", dir);
            }
            Err(e) => log::error!("Failed to write trust config: {}", e),
        }
    }

    fn set_hosts_ip_value(self: Pin<&mut Self>, ip: QString) {
        self.set_hosts_redirect_ip(ip);
    }

    fn apply_hosts(self: Pin<&mut Self>) {
        let ip = self.hosts_redirect_ip().to_string();
        match crate::setup::hosts::apply_hosts_entries(&ip) {
            Ok(()) => log::info!("Hosts entries applied for {}", ip),
            Err(e) => log::error!("Failed to apply hosts: {}", e),
        }
    }

    fn open_firewall_ports(self: Pin<&mut Self>) {
        let backend = crate::setup::firewall::detect_backend();
        match crate::setup::firewall::open_ports(&backend) {
            Ok(()) => {
                self.set_firewall_opened(true);
                log::info!("Firewall ports opened");
            }
            Err(e) => log::error!("Failed to open ports: {}", e),
        }
    }
}

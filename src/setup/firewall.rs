// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 ddavef/KinteLiX retouched-server

use std::process::Command;

pub const REQUIRED_PORTS: &[u16] = &[8080, 8088, 8089, 8443, 9081];

#[derive(Clone, Debug)]
pub enum FirewallBackend {
    Ufw,
    Firewalld,
    #[cfg(target_os = "windows")]
    Netsh,
    None,
}

impl FirewallBackend {
    pub fn name(&self) -> &str {
        match self {
            Self::Ufw => "ufw",
            Self::Firewalld => "firewalld",
            #[cfg(target_os = "windows")]
            Self::Netsh => "netsh",
            Self::None => "none",
        }
    }
}

pub fn detect_backend() -> FirewallBackend {
    #[cfg(target_os = "linux")]
    {
        if which_exists("ufw") {
            return FirewallBackend::Ufw;
        }
        if which_exists("firewall-cmd") {
            return FirewallBackend::Firewalld;
        }
        FirewallBackend::None
    }
    #[cfg(target_os = "windows")]
    {
        FirewallBackend::Netsh
    }
    #[cfg(target_os = "macos")]
    {
        FirewallBackend::None
    }
    #[cfg(not(any(target_os = "linux", target_os = "windows", target_os = "macos")))]
    {
        FirewallBackend::None
    }
}

#[cfg(target_os = "linux")]
fn which_exists(cmd: &str) -> bool {
    Command::new("which")
        .arg(cmd)
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
}

fn firewalld_active_zone() -> String {
    let lan_iface = detect_lan_interface();

    if let Some(ref iface) = lan_iface {
        if let Ok(out) = Command::new("firewall-cmd")
            .args(["--get-zone-of-interface", iface])
            .output()
        {
            if out.status.success() {
                let zone = String::from_utf8_lossy(&out.stdout).trim().to_string();
                if !zone.is_empty() {
                    log::info!(
                        "firewalld: interface '{}' belongs to zone '{}'",
                        iface,
                        zone
                    );
                    return zone;
                }
            }
        }
        log::warn!(
            "firewalld: could not determine zone for interface '{}', trying defaults",
            iface
        );
    }

    if let Ok(out) = Command::new("firewall-cmd")
        .arg("--get-default-zone")
        .output()
    {
        let zone = String::from_utf8_lossy(&out.stdout).trim().to_string();
        if !zone.is_empty() {
            log::info!("firewalld: using default zone '{}'", zone);
            return zone;
        }
    }

    log::warn!("firewalld: could not detect zone, falling back to 'public'");
    "public".to_string()
}

fn detect_lan_interface() -> Option<String> {
    let lan_ip = local_ip_address::local_ip().ok()?;

    let ifas = local_ip_address::list_afinet_netifas().ok()?;
    for (name, ip) in &ifas {
        if *ip == lan_ip {
            log::info!("firewall: LAN IP {} is on interface '{}'", lan_ip, name);
            return Some(name.clone());
        }
    }

    log::warn!("firewall: could not find interface for LAN IP {}", lan_ip);
    None
}

pub fn open_ports(backend: &FirewallBackend) -> Result<(), Box<dyn std::error::Error>> {
    match backend {
        FirewallBackend::Ufw => {
            let cmds: Vec<String> = REQUIRED_PORTS
                .iter()
                .map(|p| format!("ufw allow {}/tcp", p))
                .collect();
            let script = cmds.join(" && ");

            let status = Command::new("pkexec")
                .args(["sh", "-c", &script])
                .status()?;
            if !status.success() {
                return Err("pkexec ufw: failed to open ports".into());
            }
            for &port in REQUIRED_PORTS {
                log::info!("ufw: opened port {}/tcp", port);
            }
            Ok(())
        }
        FirewallBackend::Firewalld => {
            let zone = firewalld_active_zone();

            let mut cmds: Vec<String> = REQUIRED_PORTS
                .iter()
                .map(|p| {
                    format!(
                        "firewall-cmd --zone={} --permanent --add-port={}/tcp",
                        zone, p
                    )
                })
                .collect();
            cmds.push("firewall-cmd --reload".to_string());
            let script = cmds.join(" && ");

            let status = Command::new("pkexec")
                .args(["sh", "-c", &script])
                .status()?;
            if !status.success() {
                return Err(format!(
                    "pkexec firewall-cmd: failed to open ports in zone '{}'",
                    zone
                )
                .into());
            }
            for &port in REQUIRED_PORTS {
                log::info!("firewalld: opened port {}/tcp in zone '{}'", port, zone);
            }
            log::info!("firewalld: configuration reloaded");
            Ok(())
        }
        #[cfg(target_os = "windows")]
        FirewallBackend::Netsh => {
            let rules: Vec<String> = REQUIRED_PORTS
                .iter()
                .map(|p| {
                    format!(
                        "netsh advfirewall firewall add rule name='RetouchedServer_TCP_{}' dir=in action=allow protocol=TCP localport={}",
                        p, p
                    )
                })
                .collect();
            let combined = rules.join("; ");

            let status = Command::new("powershell")
                .args([
                    "-Command",
                    &format!(
                        "Start-Process powershell -Verb RunAs -Wait -ArgumentList '-Command','{}'",
                        combined.replace('\'', "''")
                    ),
                ])
                .status()?;
            if !status.success() {
                return Err("netsh: failed to open firewall ports".into());
            }
            for &port in REQUIRED_PORTS {
                log::info!("netsh: opened port {}/tcp", port);
            }
            Ok(())
        }
        FirewallBackend::None => Err("No supported firewall backend detected".into()),
    }
}

pub fn close_ports(backend: &FirewallBackend) -> Result<(), Box<dyn std::error::Error>> {
    match backend {
        FirewallBackend::Ufw => {
            let cmds: Vec<String> = REQUIRED_PORTS
                .iter()
                .map(|p| format!("ufw delete allow {}/tcp", p))
                .collect();
            let script = cmds.join(" ; ");

            let status = Command::new("pkexec")
                .args(["sh", "-c", &script])
                .status()?;
            if !status.success() {
                log::warn!("ufw: some port removals may have failed");
            }
            Ok(())
        }
        FirewallBackend::Firewalld => {
            let zone = firewalld_active_zone();

            let mut cmds: Vec<String> = REQUIRED_PORTS
                .iter()
                .map(|p| {
                    format!(
                        "firewall-cmd --zone={} --permanent --remove-port={}/tcp",
                        zone, p
                    )
                })
                .collect();
            cmds.push("firewall-cmd --reload".to_string());
            let script = cmds.join(" ; ");

            let status = Command::new("pkexec")
                .args(["sh", "-c", &script])
                .status()?;
            if !status.success() {
                log::warn!(
                    "firewalld: some port removals may have failed in zone '{}'",
                    zone
                );
            }
            Ok(())
        }
        #[cfg(target_os = "windows")]
        FirewallBackend::Netsh => {
            let rules: Vec<String> = REQUIRED_PORTS
                .iter()
                .map(|p| {
                    format!(
                        "netsh advfirewall firewall delete rule name='RetouchedServer_TCP_{}'",
                        p
                    )
                })
                .collect();
            let combined = rules.join("; ");

            let _ = Command::new("powershell")
                .args([
                    "-Command",
                    &format!(
                        "Start-Process powershell -Verb RunAs -Wait -ArgumentList '-Command','{}'",
                        combined.replace('\'', "''")
                    ),
                ])
                .status();
            Ok(())
        }
        FirewallBackend::None => Err("No supported firewall backend detected".into()),
    }
}

// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 ddavef/KinteLiX retouched-server

use std::net::IpAddr;
use std::path::PathBuf;
use std::process::Command;

pub const MANAGED_DOMAINS: &[&str] = &["registry.monkeysecurity.com", "playbrassmonkey.com"];

#[derive(Clone, Debug)]
pub enum HostEntryState {
    Present(String),
    Missing,
}

pub fn hosts_file_path() -> PathBuf {
    #[cfg(target_os = "windows")]
    {
        PathBuf::from(r"C:\Windows\System32\drivers\etc\hosts")
    }

    #[cfg(target_os = "macos")]
    {
        PathBuf::from("/private/etc/hosts")
    }

    #[cfg(target_os = "linux")]
    {
        PathBuf::from("/etc/hosts")
    }

    #[cfg(not(any(target_os = "windows", target_os = "macos", target_os = "linux")))]
    {
        PathBuf::from("/etc/hosts")
    }
}

pub fn check_hosts_entries() -> Vec<(String, HostEntryState)> {
    let path = hosts_file_path();
    let content = std::fs::read_to_string(&path).unwrap_or_default();

    MANAGED_DOMAINS
        .iter()
        .map(|&domain| {
            let state = find_entry_ip(&content, domain)
                .map(|ip| HostEntryState::Present(ip))
                .unwrap_or(HostEntryState::Missing);
            (domain.to_string(), state)
        })
        .collect()
}

fn find_entry_ip(content: &str, domain: &str) -> Option<String> {
    for line in content.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        let mut parts = line.split_whitespace();
        if let Some(ip) = parts.next() {
            for hostname in parts {
                if hostname.eq_ignore_ascii_case(domain) {
                    return Some(ip.to_string());
                }
            }
        }
    }
    None
}

fn validate_ip(ip: &str) -> Result<(), Box<dyn std::error::Error>> {
    ip.parse::<IpAddr>()?;
    Ok(())
}

fn build_new_hosts_content(current: &str, ip: &str) -> (String, bool) {
    let mut lines: Vec<String> = current.lines().map(|l| l.to_string()).collect();
    let mut changed = false;

    for &domain in MANAGED_DOMAINS {
        let mut found = false;
        for line in lines.iter_mut() {
            let trimmed = line.trim();
            if trimmed.is_empty() || trimmed.starts_with('#') {
                continue;
            }
            let mut parts = trimmed.split_whitespace();
            if let Some(existing_ip) = parts.next() {
                if parts.any(|h| h.eq_ignore_ascii_case(domain)) {
                    found = true;
                    if existing_ip != ip {
                        *line = format!("{} {}", ip, domain);
                        changed = true;
                    }
                }
            }
        }
        if !found {
            lines.push(format!("{} {}", ip, domain));
            changed = true;
        }
    }

    let mut content = lines.join("\n");
    if !content.ends_with('\n') {
        content.push('\n');
    }
    (content, changed)
}

fn build_hosts_without_managed(current: &str) -> (String, bool) {
    let mut lines: Vec<String> = Vec::new();
    let mut changed = false;

    for line in current.lines() {
        let trimmed = line.trim();
        let is_managed = if !trimmed.is_empty() && !trimmed.starts_with('#') {
            let mut parts = trimmed.split_whitespace();
            if parts.next().is_some() {
                parts.any(|h| MANAGED_DOMAINS.iter().any(|d| h.eq_ignore_ascii_case(d)))
            } else {
                false
            }
        } else {
            false
        };

        if is_managed {
            changed = true;
        } else {
            lines.push(line.to_string());
        }
    }

    let mut content = lines.join("\n");
    if !content.ends_with('\n') {
        content.push('\n');
    }
    (content, changed)
}

pub fn apply_hosts_entries(ip: &str) -> Result<(), Box<dyn std::error::Error>> {
    validate_ip(ip)?;

    let current = std::fs::read_to_string(hosts_file_path()).unwrap_or_default();
    let (new_content, changed) = build_new_hosts_content(&current, ip);

    if !changed {
        log::info!("Hosts file already up to date");
        return Ok(());
    }

    write_hosts_elevated(&new_content)
}

pub fn remove_hosts_entries() -> Result<(), Box<dyn std::error::Error>> {
    let current = std::fs::read_to_string(hosts_file_path()).unwrap_or_default();
    let (new_content, changed) = build_hosts_without_managed(&current);

    if !changed {
        log::info!("No managed entries to remove");
        return Ok(());
    }

    write_hosts_elevated(&new_content)
}

fn write_hosts_elevated(content: &str) -> Result<(), Box<dyn std::error::Error>> {
    let tmp_dir = std::env::temp_dir();
    let tmp_path = tmp_dir.join("retouched_hosts.tmp");
    std::fs::write(&tmp_path, content)?;

    let result = write_hosts_platform(&tmp_path);

    let _ = std::fs::remove_file(&tmp_path);
    result
}

#[cfg(target_os = "linux")]
fn write_hosts_platform(tmp_path: &std::path::Path) -> Result<(), Box<dyn std::error::Error>> {
    let hosts = "/etc/hosts";
    let tmp_str = tmp_path.to_string_lossy();
    let status = Command::new("pkexec")
        .args(["cp", "--", &tmp_str, hosts])
        .status()?;
    if !status.success() {
        return Err(format!("pkexec cp failed (exit {})", status).into());
    }
    log::info!("Hosts file updated successfully");
    Ok(())
}

#[cfg(target_os = "macos")]
fn write_hosts_platform(tmp_path: &std::path::Path) -> Result<(), Box<dyn std::error::Error>> {
    let hosts = "/private/etc/hosts";
    let tmp_str = tmp_path.to_string_lossy();
    let script = format!(
        r#"do shell script "cp -- '{}' '{}'" with administrator privileges"#,
        tmp_str.replace('\'', "'\\''"),
        hosts,
    );
    let status = Command::new("osascript").args(["-e", &script]).status()?;
    if !status.success() {
        return Err(format!("osascript cp failed (exit {})", status).into());
    }
    log::info!("Hosts file updated successfully");
    Ok(())
}

#[cfg(target_os = "windows")]
fn write_hosts_platform(tmp_path: &std::path::Path) -> Result<(), Box<dyn std::error::Error>> {
    let hosts = r"C:\Windows\System32\drivers\etc\hosts";
    let tmp_str = tmp_path.to_string_lossy();
    let status = Command::new("powershell")
        .args([
            "-Command",
            &format!(
                "Start-Process powershell -Verb RunAs -Wait -ArgumentList '-Command','Copy-Item -LiteralPath ''{}'' -Destination ''{}'' -Force'",
                tmp_str, hosts
            ),
        ])
        .status()?;
    if !status.success() {
        return Err("powershell hosts update failed".into());
    }
    log::info!("Hosts file updated successfully");
    Ok(())
}

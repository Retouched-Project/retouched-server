// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 ddavef/KinteLiX retouched-server

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

pub fn apply_hosts_entries(ip: &str) -> Result<(), Box<dyn std::error::Error>> {
    let content = std::fs::read_to_string(hosts_file_path()).unwrap_or_default();

    let mut sed_exprs: Vec<String> = Vec::new();
    let mut append_lines: Vec<String> = Vec::new();

    for &domain in MANAGED_DOMAINS {
        match find_entry_ip(&content, domain) {
            Some(existing_ip) if existing_ip == ip => {
                log::info!("Hosts: {} already points to {}", domain, ip);
            }
            Some(_) => {
                let escaped = domain.replace('.', r"\.");
                sed_exprs.push(format!("s/^[^#]*{}$/{} {}/", escaped, ip, domain));
            }
            None => {
                append_lines.push(format!("{} {}", ip, domain));
            }
        }
    }

    if sed_exprs.is_empty() && append_lines.is_empty() {
        return Ok(());
    }

    run_hosts_batch(&sed_exprs, &append_lines)
}

pub fn remove_hosts_entries() -> Result<(), Box<dyn std::error::Error>> {
    let mut sed_exprs: Vec<String> = Vec::new();
    for &domain in MANAGED_DOMAINS {
        let escaped = domain.replace('.', r"\.");
        sed_exprs.push(format!("/{}$/d", escaped));
    }
    run_hosts_batch(&sed_exprs, &[])
}

#[cfg(target_os = "linux")]
fn run_hosts_batch(
    sed_exprs: &[String],
    append_lines: &[String],
) -> Result<(), Box<dyn std::error::Error>> {
    let hosts = "/etc/hosts";
    let mut cmds: Vec<String> = Vec::new();

    for expr in sed_exprs {
        cmds.push(format!("sed -i '{}' {}", expr, hosts));
    }
    for line in append_lines {
        cmds.push(format!("echo '{}' >> {}", line, hosts));
    }

    if cmds.is_empty() {
        return Ok(());
    }

    let script = cmds.join(" && ");
    let status = Command::new("pkexec")
        .args(["sh", "-c", &script])
        .status()?;
    if !status.success() {
        return Err(format!("pkexec hosts update failed (exit {})", status).into());
    }
    log::info!("Hosts file updated successfully");
    Ok(())
}

#[cfg(target_os = "macos")]
fn run_hosts_batch(
    sed_exprs: &[String],
    append_lines: &[String],
) -> Result<(), Box<dyn std::error::Error>> {
    let hosts = "/private/etc/hosts";
    let mut cmds: Vec<String> = Vec::new();

    for expr in sed_exprs {
        cmds.push(format!("sed -i '' '{}' {}", expr, hosts));
    }
    for line in append_lines {
        cmds.push(format!("echo '{}' >> {}", line, hosts));
    }

    if cmds.is_empty() {
        return Ok(());
    }

    let combined = cmds.join(" && ");
    let script = format!(
        r#"do shell script "{}" with administrator privileges"#,
        combined
    );
    let status = Command::new("osascript").args(["-e", &script]).status()?;
    if !status.success() {
        return Err(format!("osascript hosts update failed (exit {})", status).into());
    }
    log::info!("Hosts file updated successfully");
    Ok(())
}

#[cfg(target_os = "windows")]
fn run_hosts_batch(
    sed_exprs: &[String],
    append_lines: &[String],
) -> Result<(), Box<dyn std::error::Error>> {
    let hosts = r"C:\Windows\System32\drivers\etc\hosts";
    let mut ps_cmds: Vec<String> = Vec::new();

    ps_cmds.push(format!("$c = Get-Content '{}'", hosts));

    for expr in sed_exprs {
        if expr.ends_with("/d") {
            let pattern = expr.trim_start_matches('/').trim_end_matches("/d");
            ps_cmds.push(format!(
                "$c = $c | Where-Object {{ $_ -notmatch '{}' }}",
                pattern
            ));
        } else if expr.starts_with("s/") {
            let inner = expr.trim_start_matches("s/").trim_end_matches('/');
            if let Some((pat, rep)) = inner.split_once('/') {
                ps_cmds.push(format!("$c = $c -replace '{}','{}'", pat, rep));
            }
        }
    }

    for line in append_lines {
        ps_cmds.push(format!("$c += '{}'", line));
    }

    ps_cmds.push(format!("$c | Set-Content '{}'", hosts));

    let combined = ps_cmds.join("; ");
    let status = Command::new("powershell")
        .args([
            "-Command",
            &format!(
                "Start-Process powershell -Verb RunAs -Wait -ArgumentList '-Command','{}' ",
                combined.replace('\'', "''")
            ),
        ])
        .status()?;
    if !status.success() {
        return Err("powershell hosts update failed".into());
    }
    log::info!("Hosts file updated successfully");
    Ok(())
}

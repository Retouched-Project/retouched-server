// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 ddavef/KinteLiX retouched-server

use std::io::{Read, Write};
use std::net::{Ipv4Addr, SocketAddr, TcpStream};
use std::process::Command;
use std::time::Duration;

pub const POLICY_PORT: u16 = 843;

#[derive(Clone, Debug)]
pub enum RedirectBackend {
    Firewalld,
    Iptables,
    Pf,
    None,
}

impl RedirectBackend {
    pub fn name(&self) -> &str {
        match self {
            Self::Firewalld => "firewalld",
            Self::Iptables => "iptables",
            Self::Pf => "pf",
            Self::None => "none",
        }
    }
}

#[cfg(target_os = "linux")]
fn firewalld_available() -> bool {
    crate::setup::firewall::which_exists("firewall-cmd")
}

pub fn detect_backend() -> RedirectBackend {
    #[cfg(target_os = "linux")]
    {
        if firewalld_available() {
            RedirectBackend::Firewalld
        } else {
            RedirectBackend::Iptables
        }
    }
    #[cfg(target_os = "macos")]
    {
        RedirectBackend::Pf
    }
    #[cfg(not(any(target_os = "linux", target_os = "macos")))]
    {
        RedirectBackend::None
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RedirectStatus {
    Active,
    Inactive,
    Unknown,
}

// send a policy request so the server serves the policy and closes cleanly
fn serves_policy(port: u16) -> bool {
    let addr = SocketAddr::from((Ipv4Addr::LOCALHOST, port));
    let Ok(mut stream) = TcpStream::connect_timeout(&addr, Duration::from_millis(300)) else {
        return false;
    };
    let _ = stream.set_write_timeout(Some(Duration::from_millis(300)));
    let _ = stream.set_read_timeout(Some(Duration::from_millis(500)));
    if stream.write_all(b"<policy-file-request/>\0").is_err() {
        return false;
    }
    let mut buf = [0u8; 8];
    matches!(stream.read(&mut buf), Ok(n) if n > 0 && buf[0] == b'<')
}

// 843 serving policy means the redirect is live,
// only the server port serving means it is missing
pub fn probe_status(target_port: u16) -> RedirectStatus {
    crate::server::set_policy_probe_active(true);
    let status = if serves_policy(POLICY_PORT) {
        RedirectStatus::Active
    } else if serves_policy(target_port) {
        RedirectStatus::Inactive
    } else {
        RedirectStatus::Unknown
    };
    crate::server::set_policy_probe_active(false);
    status
}

// no-op where there is no redirect backend, e.g. Windows binds 843 directly
pub fn warn_if_inactive(target_port: u16) {
    if matches!(detect_backend(), RedirectBackend::None) {
        return;
    }
    for _ in 0..10 {
        match probe_status(target_port) {
            RedirectStatus::Active => return,
            RedirectStatus::Inactive => {
                log::warn!(
                    "Port 843 redirect not active: Unity Web Player games will not connect. \
                     Enable the policy port redirect in Settings, or run 'retouched-server redirect apply'."
                );
                return;
            }
            RedirectStatus::Unknown => std::thread::sleep(Duration::from_millis(500)),
        }
    }
}

pub fn apply(
    backend: &RedirectBackend,
    target_port: u16,
) -> Result<(), Box<dyn std::error::Error>> {
    match backend {
        RedirectBackend::Firewalld => {
            let status = Command::new("pkexec")
                .args(["sh", "-c", &firewalld_apply_script(target_port)])
                .status()?;
            if !status.success() {
                return Err("pkexec firewall-cmd: failed to add policy port redirect".into());
            }
            log::info!(
                "firewalld: redirecting tcp/{} to tcp/{} (persistent)",
                POLICY_PORT,
                target_port
            );
            Ok(())
        }
        RedirectBackend::Iptables => {
            let status = Command::new("pkexec")
                .args(["sh", "-c", &iptables_apply_script(target_port)])
                .status()?;
            if !status.success() {
                return Err("pkexec iptables: failed to add policy port redirect".into());
            }
            log::info!(
                "iptables: redirecting tcp/{} to tcp/{}",
                POLICY_PORT,
                target_port
            );
            Ok(())
        }
        RedirectBackend::Pf => Err(pf_unimplemented(target_port)),
        RedirectBackend::None => Err("No supported port-redirect backend detected".into()),
    }
}

pub fn remove(
    backend: &RedirectBackend,
    target_port: u16,
) -> Result<(), Box<dyn std::error::Error>> {
    match backend {
        RedirectBackend::Firewalld => {
            let status = Command::new("pkexec")
                .args(["sh", "-c", &firewalld_remove_script(target_port)])
                .status()?;
            if !status.success() {
                log::warn!("firewalld: policy port redirect removal may have failed");
            }
            Ok(())
        }
        RedirectBackend::Iptables => {
            let status = Command::new("pkexec")
                .args(["sh", "-c", &iptables_remove_script(target_port)])
                .status()?;
            if !status.success() {
                log::warn!("iptables: policy port redirect removal may have failed");
            }
            Ok(())
        }
        RedirectBackend::Pf => Err(pf_unimplemented(target_port)),
        RedirectBackend::None => Err("No supported port-redirect backend detected".into()),
    }
}

fn firewalld_direct_rule(target_port: u16) -> String {
    format!(
        "ipv4 nat OUTPUT 0 -p tcp --dport {pp} -j REDIRECT --to-ports {tp}",
        pp = POLICY_PORT,
        tp = target_port
    )
}

fn firewalld_apply_script(target_port: u16) -> String {
    let rule = firewalld_direct_rule(target_port);
    format!(
        "( firewall-cmd --direct --query-rule {rule} >/dev/null 2>&1 \
          || firewall-cmd --direct --add-rule {rule} ) \
         && ( firewall-cmd --permanent --direct --query-rule {rule} >/dev/null 2>&1 \
          || firewall-cmd --permanent --direct --add-rule {rule} )",
        rule = rule
    )
}

fn firewalld_remove_script(target_port: u16) -> String {
    let rule = firewalld_direct_rule(target_port);
    format!(
        "firewall-cmd --direct --remove-rule {rule} 2>/dev/null ; \
         firewall-cmd --permanent --direct --remove-rule {rule} 2>/dev/null ; true",
        rule = rule
    )
}

fn iptables_apply_script(target_port: u16) -> String {
    let rule = |chain: &str| {
        format!(
            "( iptables -t nat -C {chain} -p tcp --dport {pp} -j REDIRECT --to-ports {tp} 2>/dev/null \
             || iptables -t nat -A {chain} -p tcp --dport {pp} -j REDIRECT --to-ports {tp} )",
            chain = chain,
            pp = POLICY_PORT,
            tp = target_port
        )
    };
    format!("{} && {}", rule("OUTPUT"), rule("PREROUTING"))
}

fn iptables_remove_script(target_port: u16) -> String {
    let rule = |chain: &str| {
        format!(
            "iptables -t nat -D {chain} -p tcp --dport {pp} -j REDIRECT --to-ports {tp} 2>/dev/null || true",
            chain = chain,
            pp = POLICY_PORT,
            tp = target_port
        )
    };
    format!("{} ; {}", rule("OUTPUT"), rule("PREROUTING"))
}

fn pf_unimplemented(target_port: u16) -> Box<dyn std::error::Error> {
    log::warn!(
        "macOS pf redirect is not auto-configured yet; add manually: rdr pass inet proto tcp from any to any port {} -> 127.0.0.1 port {}",
        POLICY_PORT,
        target_port
    );
    "macOS pf redirect not yet implemented".into()
}

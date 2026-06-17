// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 ddavef/KinteLiX retouched-server

use std::process::Command;

pub const POLICY_PORT: u16 = 843;

#[derive(Clone, Debug)]
pub enum RedirectBackend {
    Iptables,
    Pf,
    None,
}

impl RedirectBackend {
    pub fn name(&self) -> &str {
        match self {
            Self::Iptables => "iptables",
            Self::Pf => "pf",
            Self::None => "none",
        }
    }
}

pub fn detect_backend() -> RedirectBackend {
    #[cfg(target_os = "linux")]
    {
        RedirectBackend::Iptables
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

pub fn apply(
    backend: &RedirectBackend,
    target_port: u16,
) -> Result<(), Box<dyn std::error::Error>> {
    match backend {
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

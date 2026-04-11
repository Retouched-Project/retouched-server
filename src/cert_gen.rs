// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 ddavef/KinteLiX retouched-server

use rcgen::{BasicConstraints, CertificateParams, DnType, IsCa, KeyPair, PKCS_ECDSA_P256_SHA256};
use std::path::Path;

pub fn ensure_cert(cert_dir: &Path, lan_ip: &str) -> Result<(), Box<dyn std::error::Error>> {
    std::fs::create_dir_all(cert_dir)?;

    let root_key_path = cert_dir.join("rootCA.key");
    let root_cert_path = cert_dir.join("rootCA.pem");
    let key_path = cert_dir.join("key.pem");
    let cert_path = cert_dir.join("server.pem");
    let last_ips_path = cert_dir.join("last_ips.txt");

    let mut local_ips = vec![lan_ip.to_string()];
    if let Ok(interfaces) = local_ip_address::list_afinet_netifas() {
        for (_name, ip) in interfaces {
            if let std::net::IpAddr::V4(v4) = ip {
                let s = v4.to_string();
                if !local_ips.contains(&s) && !s.starts_with("127.") {
                    local_ips.push(s);
                }
            }
        }
    }
    local_ips.sort();
    let ips_string = local_ips.join(",");

    let certs_exist = key_path.exists()
        && cert_path.exists()
        && root_cert_path.exists()
        && root_key_path.exists();
    let needs_regeneration = if !certs_exist {
        true
    } else if last_ips_path.exists() {
        let saved_ips = std::fs::read_to_string(&last_ips_path).unwrap_or_default();
        !saved_ips.split(',').any(|s| s == lan_ip)
    } else {
        true
    };

    if !needs_regeneration {
        log::info!(
            "TLS certificates already exist and cover current IP ({})",
            lan_ip
        );
        return Ok(());
    }

    log::info!("Updating TLS certificates for IPs: {}", ips_string);

    let root_key_pair = if root_key_path.exists() {
        let pem = std::fs::read_to_string(&root_key_path)?;
        KeyPair::from_pem(&pem)?
    } else {
        let key = KeyPair::generate_for(&PKCS_ECDSA_P256_SHA256)?;
        std::fs::write(&root_key_path, key.serialize_pem())?;
        key
    };

    let mut root_params = CertificateParams::default();
    root_params
        .distinguished_name
        .push(DnType::OrganizationName, "Retouched");
    root_params
        .distinguished_name
        .push(DnType::CommonName, "Retouched Local Root CA");
    root_params.is_ca = IsCa::Ca(BasicConstraints::Unconstrained);
    root_params.not_before = time::OffsetDateTime::from_unix_timestamp(1735689600)
        .unwrap_or(time::OffsetDateTime::now_utc()); // 2025-01-01
    root_params.not_after = root_params.not_before + time::Duration::days(3650);
    root_params.serial_number = Some(rcgen::SerialNumber::from(1));

    let root_cert = root_params.self_signed(&root_key_pair)?;
    if !root_cert_path.exists() {
        std::fs::write(&root_cert_path, root_cert.pem())?;
    }

    let server_key_pair = KeyPair::generate_for(&PKCS_ECDSA_P256_SHA256)?;
    let mut server_params = CertificateParams::default();
    server_params
        .distinguished_name
        .push(DnType::CommonName, "localhost");

    let mut sans = vec![rcgen::SanType::DnsName("localhost".try_into()?)];
    for ip in &local_ips {
        if let Ok(addr) = ip.parse() {
            sans.push(rcgen::SanType::IpAddress(addr));
        }
    }
    server_params.subject_alt_names = sans;

    let now = time::OffsetDateTime::now_utc();
    server_params.not_after = now + time::Duration::days(3650);

    let server_cert = server_params.signed_by(&server_key_pair, &root_cert, &root_key_pair)?;

    std::fs::write(&root_key_path, root_key_pair.serialize_pem())?;
    std::fs::write(&root_cert_path, root_cert.pem())?;
    std::fs::write(&key_path, server_key_pair.serialize_pem())?;
    std::fs::write(&cert_path, server_cert.pem())?;
    std::fs::write(&last_ips_path, ips_string)?;

    log::info!(
        "Root CA written to {} and Server TLS certificate written to {}",
        root_cert_path.display(),
        cert_path.display()
    );

    Ok(())
}

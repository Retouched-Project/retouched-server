// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 ddavef/KinteLiX retouched-server

use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::{Arc, Mutex};

fn safe_extract_zip(
    archive: &mut zip::ZipArchive<std::fs::File>,
    dest: &Path,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let canonical_dest = std::fs::canonicalize(dest)?;
    for i in 0..archive.len() {
        let mut entry = archive.by_index(i)?;
        let name = entry.name().to_string();
        let out_path = dest.join(&name);
        if !canonical_dest.join(&name).starts_with(&canonical_dest) {
            log::warn!("Skipping zip entry with path traversal: {}", name);
            continue;
        }
        if entry.is_dir() {
            std::fs::create_dir_all(&out_path)?;
        } else {
            if let Some(parent) = out_path.parent() {
                std::fs::create_dir_all(parent)?;
            }
            let mut out = std::fs::File::create(&out_path)?;
            std::io::copy(&mut entry, &mut out)?;
        }
    }
    Ok(())
}

fn safe_unpack_tar(
    file: std::fs::File,
    dest: &Path,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let canonical_dest = std::fs::canonicalize(dest)?;
    let tar = flate2::read::GzDecoder::new(file);
    let mut archive = tar::Archive::new(tar);
    for entry in archive.entries()? {
        let mut entry = entry?;
        let path = entry.path()?.to_path_buf();
        let out_path = dest.join(&path);
        if !canonical_dest.join(&path).starts_with(&canonical_dest) {
            log::warn!("Skipping tar entry with path traversal: {}", path.display());
            continue;
        }
        if let Some(parent) = out_path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        entry.unpack(&out_path)?;
    }
    Ok(())
}

const APKTOOL_URL: &str = "https://bitbucket.org/iBotPeaches/apktool/downloads/apktool_2.12.1.jar";
const JADX_RELEASE_API: &str = "https://api.github.com/repos/skylot/jadx/releases/latest";
const UBER_RELEASE_API: &str =
    "https://api.github.com/repos/patrickfav/uber-apk-signer/releases/latest";

fn string_replacements(ip: &str) -> Vec<(&'static str, String)> {
    vec![
        ("registry.monkeysecurity.com", ip.to_string()),
        (
            "http://registry.monkeysecurity.com:8080",
            format!("http://{}:8080", ip),
        ),
        (
            "http://playbrassmonkey.com/alternate-hosts.json",
            format!("http://{}/alternate-hosts.json", ip),
        ),
        (
            "https://registry.monkeysecurity.com",
            format!("https://{}", ip),
        ),
    ]
}

#[derive(Clone, Debug)]
pub enum PatchStep {
    Idle,
    DownloadingTools,
    Decompiling,
    DecompilingSources,
    PatchingStrings,
    Rebuilding,
    Signing,
    Done(PathBuf),
    Error(String),
}

impl std::fmt::Display for PatchStep {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Idle => write!(f, "Idle"),
            Self::DownloadingTools => write!(f, "Downloading tools..."),
            Self::Decompiling => write!(f, "Decompiling APK (apktool)..."),
            Self::DecompilingSources => write!(f, "Decompiling sources (jadx)..."),
            Self::PatchingStrings => write!(f, "Patching strings + smali..."),
            Self::Rebuilding => write!(f, "Rebuilding APK (apktool)..."),
            Self::Signing => write!(f, "Signing APK (uber-apk-signer)..."),
            Self::Done(p) => write!(f, "Done: {}", p.display()),
            Self::Error(e) => write!(f, "Error: {}", e),
        }
    }
}

pub type SharedStep = Arc<Mutex<PatchStep>>;

pub fn new_shared_step() -> SharedStep {
    Arc::new(Mutex::new(PatchStep::Idle))
}

#[derive(Clone, Debug)]
pub struct ToolStatus {
    pub apktool: Option<PathBuf>,
    pub jadx: Option<PathBuf>,
    pub uber: Option<PathBuf>,
    pub jre: Option<PathBuf>,
}

impl ToolStatus {
    pub fn detect(data_dir: &Path) -> Self {
        let apktool = data_dir.join("apktool.jar");
        let uber = data_dir.join("uber-apk-signer.jar");
        let jadx_root = data_dir.join("jadx");
        let jadx_bin = find_jadx_bin(&jadx_root);
        let jre_root = data_dir.join("jre17");
        let jre_bin = find_java_bin(&jre_root);

        Self {
            apktool: apktool.exists().then_some(apktool),
            jadx: jadx_bin,
            uber: uber.exists().then_some(uber),
            jre: jre_bin,
        }
    }
}

fn find_java_bin(jre_root: &Path) -> Option<PathBuf> {
    if !jre_root.exists() {
        return None;
    }
    #[cfg(windows)]
    let bin_name = "java.exe";
    #[cfg(not(windows))]
    let bin_name = "java";

    walkdir(jre_root, bin_name)
}

fn find_jadx_bin(jadx_root: &Path) -> Option<PathBuf> {
    if !jadx_root.exists() {
        return None;
    }
    #[cfg(windows)]
    let bin_name = "jadx.bat";
    #[cfg(not(windows))]
    let bin_name = "jadx";

    walkdir(jadx_root, bin_name)
}

fn walkdir(root: &Path, target: &str) -> Option<PathBuf> {
    for entry in std::fs::read_dir(root).ok()? {
        let entry = entry.ok()?;
        let path = entry.path();
        if path.is_dir() {
            if let Some(found) = walkdir(&path, target) {
                return Some(found);
            }
        } else if path.file_name().is_some_and(|n| n == target) {
            return Some(path);
        }
    }
    None
}

fn download_file(url: &str, dest: &Path) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    if let Some(parent) = dest.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let resp = reqwest::blocking::Client::builder()
        .user_agent("retouched-server")
        .build()?
        .get(url)
        .send()?;
    let bytes = resp.bytes()?;
    let mut file = std::fs::File::create(dest)?;
    file.write_all(&bytes)?;
    Ok(())
}

pub fn download_apktool(dest: &Path) -> Result<PathBuf, Box<dyn std::error::Error + Send + Sync>> {
    download_file(APKTOOL_URL, dest)?;
    log::info!("Downloaded apktool 2.12.1");
    Ok(dest.to_path_buf())
}

pub fn download_jadx(data_dir: &Path) -> Result<PathBuf, Box<dyn std::error::Error + Send + Sync>> {
    let client = reqwest::blocking::Client::builder()
        .user_agent("retouched-server")
        .build()?;
    let resp = client.get(JADX_RELEASE_API).send()?;
    let data: serde_json::Value = resp.json()?;

    let assets = data["assets"]
        .as_array()
        .ok_or("No assets in jadx release")?;
    let asset = assets
        .iter()
        .find(|a| {
            let name = a["name"].as_str().unwrap_or("");
            name.starts_with("jadx-")
                && name.ends_with(".zip")
                && !name.contains("sources")
                && !name.contains("gui")
        })
        .ok_or("No jadx zip asset found")?;

    let url = asset["browser_download_url"]
        .as_str()
        .ok_or("No download URL for jadx")?;

    let zip_path = data_dir.join("jadx.zip");
    download_file(url, &zip_path)?;

    let jadx_dir = data_dir.join("jadx");
    if jadx_dir.exists() {
        std::fs::remove_dir_all(&jadx_dir)?;
    }
    std::fs::create_dir_all(&jadx_dir)?;

    let file = std::fs::File::open(&zip_path)?;
    let mut archive = zip::ZipArchive::new(file)?;
    safe_extract_zip(&mut archive, &jadx_dir)?;

    #[cfg(unix)]
    {
        if let Some(bin) = find_jadx_bin(&jadx_dir) {
            use std::os::unix::fs::PermissionsExt;
            let mut perms = std::fs::metadata(&bin)?.permissions();
            perms.set_mode(perms.mode() | 0o755);
            std::fs::set_permissions(&bin, perms)?;
        }
    }

    log::info!("Downloaded and extracted jadx");
    Ok(jadx_dir)
}

pub fn download_jre(data_dir: &Path) -> Result<PathBuf, Box<dyn std::error::Error + Send + Sync>> {
    let os = match std::env::consts::OS {
        "macos" => "mac",
        "windows" => "windows",
        _ => "linux",
    };
    let arch = match std::env::consts::ARCH {
        "aarch64" => "aarch64",
        _ => "x64",
    };
    let url = format!(
        "https://api.adoptium.net/v3/binary/latest/17/ga/{}/{}/jre/hotspot/normal/eclipse",
        os, arch
    );

    let is_zip = os == "windows";
    let ext = if is_zip { "zip" } else { "tar.gz" };
    let archive_path = data_dir.join(format!("jre17.{}", ext));

    log::info!("Downloading JRE 17 from {}", url);
    download_file(&url, &archive_path)?;

    let jre_dir = data_dir.join("jre17");
    if jre_dir.exists() {
        std::fs::remove_dir_all(&jre_dir)?;
    }
    std::fs::create_dir_all(&jre_dir)?;

    if is_zip {
        let file = std::fs::File::open(&archive_path)?;
        let mut archive = zip::ZipArchive::new(file)?;
        safe_extract_zip(&mut archive, &jre_dir)?;
    } else {
        let file = std::fs::File::open(&archive_path)?;
        safe_unpack_tar(file, &jre_dir)?;
    }
    std::fs::remove_file(&archive_path)?;

    #[cfg(unix)]
    {
        if let Some(bin) = find_java_bin(&jre_dir) {
            use std::os::unix::fs::PermissionsExt;
            let mut perms = std::fs::metadata(&bin)?.permissions();
            perms.set_mode(perms.mode() | 0o755);
            std::fs::set_permissions(&bin, perms)?;
        }
    }

    log::info!("Downloaded and extracted jre17");
    Ok(jre_dir)
}

pub fn download_uber(dest: &Path) -> Result<PathBuf, Box<dyn std::error::Error + Send + Sync>> {
    let client = reqwest::blocking::Client::builder()
        .user_agent("retouched-server")
        .build()?;
    let resp = client.get(UBER_RELEASE_API).send()?;
    let data: serde_json::Value = resp.json()?;

    let assets = data["assets"]
        .as_array()
        .ok_or("No assets in uber-apk-signer release")?;
    let asset = assets
        .iter()
        .find(|a| {
            let name = a["name"].as_str().unwrap_or("");
            name.ends_with(".jar") && name.contains("uber-apk-signer")
        })
        .ok_or("No uber-apk-signer jar found")?;

    let url = asset["browser_download_url"]
        .as_str()
        .ok_or("No download URL for uber-apk-signer")?;

    download_file(url, dest)?;
    log::info!("Downloaded uber-apk-signer");
    Ok(dest.to_path_buf())
}

pub fn ensure_tools(
    data_dir: &Path,
) -> Result<(PathBuf, PathBuf, PathBuf, PathBuf), Box<dyn std::error::Error + Send + Sync>> {
    let apktool_path = data_dir.join("apktool.jar");
    if !apktool_path.exists() {
        download_apktool(&apktool_path)?;
    }

    let uber_path = data_dir.join("uber-apk-signer.jar");
    if !uber_path.exists() {
        download_uber(&uber_path)?;
    }

    let jadx_root = data_dir.join("jadx");
    if !jadx_root.exists() {
        download_jadx(data_dir)?;
    }

    let jadx_bin = find_jadx_bin(&jadx_root).ok_or("jadx executable not found after download")?;

    let jre_root = data_dir.join("jre17");
    if !jre_root.exists() {
        download_jre(data_dir)?;
    }

    let jre_bin = find_java_bin(&jre_root).ok_or("java executable not found after download")?;

    Ok((apktool_path, jadx_bin, uber_path, jre_bin))
}

fn run_cmd(args: &[&str]) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    log::info!("Running: {}", args.join(" "));
    let status = Command::new(args[0]).args(&args[1..]).status()?;
    if !status.success() {
        return Err(format!("Command failed with exit code: {}", status).into());
    }
    Ok(())
}

fn run_cmd_env(
    args: &[&str],
    java_home: &Path,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    log::info!(
        "Running (JAVA_HOME={}): {}",
        java_home.display(),
        args.join(" ")
    );
    let status = Command::new(args[0])
        .args(&args[1..])
        .env("JAVA_HOME", java_home)
        .status()?;
    if !status.success() {
        return Err(format!("Command failed with exit code: {}", status).into());
    }
    Ok(())
}

fn run_cmd_warn(args: &[&str]) -> bool {
    log::info!("Running: {}", args.join(" "));
    match Command::new(args[0]).args(&args[1..]).status() {
        Ok(s) if s.success() => true,
        Ok(s) => {
            log::warn!("Command finished with {}: {}", s, args.join(" "));
            false
        }
        Err(e) => {
            log::warn!("Command failed to run ({}): {}", e, args.join(" "));
            false
        }
    }
}

fn patch_strings(
    decomp_dir: &Path,
    ip: &str,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let strings_xml = decomp_dir.join("res").join("values").join("strings.xml");
    if !strings_xml.exists() {
        return Err(format!("Missing strings.xml at {}", strings_xml.display()).into());
    }
    let mut content = std::fs::read_to_string(&strings_xml)?;
    for (old, new) in string_replacements(ip) {
        content = content.replace(old, &new);
    }
    std::fs::write(&strings_xml, content)?;
    log::info!("Patched strings.xml with IP {}", ip);
    Ok(())
}

fn patch_smali_icon_url(
    decomp_dir: &Path,
    ip: &str,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let smali_path = decomp_dir
        .join("smali")
        .join("com")
        .join("brassmonkeysdk")
        .join("c")
        .join("d.smali");
    if !smali_path.exists() {
        log::warn!(
            "Smali file not found at {}, skipping icon URL patch",
            smali_path.display()
        );
        return Ok(());
    }
    let content = std::fs::read_to_string(&smali_path)?;
    let old = "const-string v3, \"http://prod.playbrassmonkey.com/apps/icons/\"";
    let new = format!("const-string v3, \"http://{}:8080/apps/icons/\"", ip);
    if content.contains(old) {
        let patched = content.replace(old, &new);
        std::fs::write(&smali_path, patched)?;
        log::info!("Patched smali icon URL to point to {}:8080", ip);
    } else {
        log::warn!("Icon URL pattern not found in d.smali, may already be patched"); // Why would you try to patch a patched APK again?
    }
    Ok(())
}

pub fn run_patch_pipeline(
    apk_path: &Path,
    ip: &str,
    data_dir: &Path,
    step: &SharedStep,
) -> Result<PathBuf, Box<dyn std::error::Error + Send + Sync>> {
    let apk_stem = apk_path
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("unknown.apk");
    let decomp_dir = data_dir.join(format!("{}-decompiled", apk_stem));
    let dist_dir = data_dir.join("dist");
    let signed_dir = data_dir.join("signed");

    *step.lock().unwrap() = PatchStep::DownloadingTools;
    let (apktool, jadx, uber, jre) = ensure_tools(data_dir)?;

    if decomp_dir.exists() {
        std::fs::remove_dir_all(&decomp_dir)?;
    }
    std::fs::create_dir_all(&decomp_dir)?;

    *step.lock().unwrap() = PatchStep::Decompiling;
    let apktool_str = apktool.display().to_string();
    let decomp_str = decomp_dir.display().to_string();
    let apk_str = apk_path.display().to_string();
    let java_str = jre.display().to_string();
    run_cmd(&[
        &java_str,
        "-Xmx256m",
        "-jar",
        &apktool_str,
        "d",
        "-f",
        "-o",
        &decomp_str,
        &apk_str,
    ])?;

    *step.lock().unwrap() = PatchStep::DecompilingSources;
    let jadx_str = jadx.display().to_string();
    let java_home = jre
        .parent()
        .and_then(|bin| bin.parent())
        .ok_or("Cannot derive JAVA_HOME from JRE path")?;
    run_cmd_env(&[&jadx_str, "-r", "-d", &decomp_str, &apk_str], java_home)?;

    *step.lock().unwrap() = PatchStep::PatchingStrings;
    patch_strings(&decomp_dir, ip)?;
    patch_smali_icon_url(&decomp_dir, ip)?;

    *step.lock().unwrap() = PatchStep::Rebuilding;
    if dist_dir.exists() {
        std::fs::remove_dir_all(&dist_dir)?;
    }
    let rebuilt_name = apk_stem.strip_suffix(".apk").unwrap_or(apk_stem);
    let rebuilt_apk = dist_dir.join(format!("{}-rebuilt.apk", rebuilt_name));
    let rebuilt_str = rebuilt_apk.display().to_string();
    run_cmd_warn(&[
        &java_str,
        "-Xmx256m",
        "-jar",
        &apktool_str,
        "b",
        &decomp_str,
        "-o",
        &rebuilt_str,
    ]);

    *step.lock().unwrap() = PatchStep::Signing;
    if signed_dir.exists() {
        std::fs::remove_dir_all(&signed_dir)?;
    }
    std::fs::create_dir_all(&signed_dir)?;
    let uber_str = uber.display().to_string();
    let signed_str = signed_dir.display().to_string();
    run_cmd_warn(&[
        &java_str,
        "-Xmx256m",
        "-jar",
        &uber_str,
        "--apks",
        &rebuilt_str,
        "--out",
        &signed_str,
    ]);

    *step.lock().unwrap() = PatchStep::Done(signed_dir.clone());
    log::info!("Signed APK in {}", signed_dir.display());
    Ok(signed_dir)
}

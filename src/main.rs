// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 ddavef/KinteLiX retouched-server

// Windows tomfoolery: avoid scaring the user with a terminal window when starting the GUI unless debugging
#![cfg_attr(
    all(not(debug_assertions), feature = "gui"),
    windows_subsystem = "windows"
)]

use clap::{Parser, Subcommand};
use std::path::PathBuf;
use std::sync::Arc;

use retouched_server::config::Config;
use retouched_server::http_server::HttpServerState;
use retouched_server::icon_cache::IconCache;
use retouched_server::server::Server;
#[cfg(feature = "gui")]
use retouched_server::{ServerCommand, gui, gui_logger};
use retouched_server::{
    app_dirs, cert_gen, http_server, setup, touchy_patcher, web_manager, webrtc_bridge,
};

#[derive(Parser, Debug)]
#[command(name = "retouched-server", version, about = "Retouched Server")]
struct Cli {
    #[arg(short, long)]
    config: Option<PathBuf>,

    #[arg(short, long)]
    debug: bool,

    #[arg(long)]
    log_level: Option<String>,

    #[arg(long)]
    data_dir: Option<PathBuf>,

    #[cfg(feature = "gui")]
    #[arg(long)]
    headless: bool,

    #[command(subcommand)]
    command: Option<Commands>,
}

#[derive(Subcommand, Debug)]
enum Commands {
    Serve {
        #[arg(long)]
        host: Option<String>,
        #[arg(long)]
        port: Option<u16>,
        #[arg(long)]
        http_port: Option<u16>,
        #[arg(long)]
        bridge: bool,
        #[arg(long)]
        bridge_port: Option<u16>,
        #[arg(long)]
        web: bool,
    },
    Web {
        #[command(subcommand)]
        action: WebAction,
    },
    Patch {
        apk: PathBuf,
        #[arg(long, short)]
        ip: String,
    },
    Tools {
        #[command(subcommand)]
        action: ToolsAction,
    },
    Hosts {
        #[command(subcommand)]
        action: HostsAction,
    },
    Trust {
        #[command(subcommand)]
        action: TrustAction,
    },
    Firewall {
        #[command(subcommand)]
        action: FirewallAction,
    },
}

#[derive(Subcommand, Debug)]
enum WebAction {
    Download {
        #[arg(long)]
        dir: Option<PathBuf>,
    },
}

#[derive(Subcommand, Debug)]
enum ToolsAction {
    Download,
}

#[derive(Subcommand, Debug)]
enum HostsAction {
    Status,
    Apply {
        #[arg(long)]
        ip: Option<String>,
    },
    Remove,
}

#[derive(Subcommand, Debug)]
enum TrustAction {
    List,
    Add { dir: PathBuf },
    Remove { dir: PathBuf },
}

#[derive(Subcommand, Debug)]
enum FirewallAction {
    Status,
    Open,
    Close,
}

fn default_config_path() -> PathBuf {
    directories::ProjectDirs::from("com", "retouched", "retouched-server")
        .map(|d| d.config_dir().join("config.json"))
        .unwrap_or_else(|| PathBuf::from("config.json"))
}

fn main() {
    rustls::crypto::ring::default_provider()
        .install_default()
        .expect("Failed to install rustls CryptoProvider");

    let cli = Cli::parse();

    match cli.command {
        #[cfg(feature = "gui")]
        None if !cli.headless => run_gui(cli),
        None | Some(Commands::Serve { .. }) => {
            let rt = tokio::runtime::Builder::new_multi_thread()
                .enable_all()
                .build()
                .expect("Failed to create tokio runtime");
            if let Err(e) = rt.block_on(run_headless(cli)) {
                eprintln!("Server error: {}", e);
                std::process::exit(1);
            }
        }
        Some(Commands::Patch { apk, ip }) => cli_patch(apk, ip, cli.data_dir),
        Some(Commands::Tools { action }) => cli_tools(action, cli.data_dir),
        Some(Commands::Web { action }) => cli_web(action, &cli.config, cli.data_dir),
        Some(Commands::Hosts { action }) => cli_hosts(action),
        Some(Commands::Trust { action }) => cli_trust(action),
        Some(Commands::Firewall { action }) => cli_firewall(action),
    }
}

#[cfg(feature = "gui")]
fn run_gui(cli: Cli) {
    let shared = retouched_server::shared_state::SharedState::new();

    let config_path = cli.config.clone().unwrap_or_else(default_config_path);
    let show_wizard = !config_path.exists();

    let config = if config_path.exists() {
        Config::from_file(&config_path).unwrap_or_else(|e| {
            eprintln!("Failed to load config ({}), using defaults", e);
            Config::default()
        })
    } else {
        Config::default()
    };

    let effective_log_level = cli.log_level.as_deref().unwrap_or(&config.log_level);
    let log_level = parse_log_level(effective_log_level, cli.debug);
    gui_logger::GuiLogger::init(shared.clone(), log_level);

    let data_dir = cli.data_dir.clone();

    let (server_tx, server_rx) = std::sync::mpsc::sync_channel::<ServerCommand>(1);
    {
        let shared_for_server = shared.clone();
        let data_dir_for_thread = data_dir.clone();
        std::thread::Builder::new()
            .name("server".into())
            .spawn(move || {
                let rt = tokio::runtime::Builder::new_multi_thread()
                    .enable_all()
                    .build()
                    .expect("Failed to create tokio runtime");

                let d = data_dir_for_thread.clone();
                rt.spawn(async move {
                    retouched_server::icon_cache::IconCache::new(d.as_deref())
                        .download_icons()
                        .await;
                });

                while let Ok(cmd) = server_rx.recv() {
                    match cmd {
                        ServerCommand::Start { config, data_dir } => {
                            let s = shared_for_server.clone();
                            rt.block_on(async move {
                                s.set_server_status(
                                    retouched_server::shared_state::ServerStatus::Starting,
                                );
                                if let Err(e) =
                                    gui::run_server_task(config, data_dir, s.clone()).await
                                {
                                    log::error!("Server task error: {}", e);
                                }
                                s.set_server_status(
                                    retouched_server::shared_state::ServerStatus::Stopped,
                                );
                                *s.server_started_at.lock().unwrap() = None;
                            });
                        }
                    }
                }
            })
            .expect("Failed to spawn server thread");
    }

    gui::run_qt_app(
        config,
        config_path,
        data_dir,
        show_wizard,
        shared,
        server_tx,
    );
}

async fn run_headless(cli: Cli) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let config_path = cli.config.clone().unwrap_or_else(default_config_path);
    let mut config = match Config::from_file(&config_path) {
        Ok(cfg) => cfg,
        Err(e) => {
            eprintln!("Failed to load config ({}), using defaults", e);
            Config::default()
        }
    };

    let effective_log_level = cli.log_level.as_deref().unwrap_or(&config.log_level);
    let log_level = parse_log_level(effective_log_level, cli.debug);
    env_logger::Builder::new()
        .filter_level(log_level)
        .format_timestamp_millis()
        .init();

    log::info!("Loaded configuration from: {}", config_path.display());

    if cli.debug {
        config.debug = true;
    }
    if let Some(ref level) = cli.log_level {
        config.log_level = level.clone();
    }

    let (run_bridge, run_web, bridge_port_override) = if let Some(Commands::Serve {
        host,
        port,
        http_port,
        bridge,
        bridge_port,
        web,
    }) = cli.command
    {
        if let Some(h) = host {
            config.server_host = h;
        }
        if let Some(p) = port {
            config.server_port = p;
        }
        if let Some(hp) = http_port {
            config.http_port = hp;
        }

        if !bridge && !web {
            eprintln!(
                "Error: You must specify either --bridge or --web (or both) when running 'serve'."
            );
            eprintln!("Try 'retouched-server serve --help' for more information.");
            std::process::exit(1);
        }

        (bridge, web, bridge_port)
    } else {
        (false, false, None)
    };

    log::info!("Retouched Server v{} (headless)", env!("CARGO_PKG_VERSION"));
    log::info!("TCP on {}:{}", config.server_host, config.server_port);
    log::info!("HTTP on {}:{}", config.server_host, config.http_port);

    let data_dir = cli.data_dir.as_deref();
    let data_dir_cached = retouched_server::app_dirs::app_data_dir(data_dir);
    let icon_cache = IconCache::new(data_dir);
    icon_cache.download_icons().await;

    let http_state = Arc::new(HttpServerState {
        icon_cache,
        shared: None,
        data_dir: data_dir_cached.clone(),
    });
    let http_router = http_server::build_router(http_state);
    let http_addr = format!("{}:{}", config.server_host, config.http_port);
    let http_listener = tokio::net::TcpListener::bind(&http_addr).await?;
    log::info!("HTTP server listening on {}", http_addr);
    tokio::spawn(async move {
        if let Err(e) = axum::serve(
            http_listener,
            http_router.into_make_service_with_connect_info::<std::net::SocketAddr>(),
        )
        .await
        {
            log::error!("HTTP server error: {}", e);
        }
    });

    let bridge_opt = if run_bridge {
        let lan_ip = local_ip_address::local_ip()
            .map(|ip| ip.to_string())
            .unwrap_or_else(|_| "127.0.0.1".to_string());

        let cert_dir = data_dir_cached.join("certs");
        if let Err(e) = cert_gen::ensure_cert(&cert_dir, &lan_ip) {
            log::error!("Failed to generate cert: {}", e);
        }
        let bp = bridge_port_override.unwrap_or(config.webrtc_port);

        let bridge = match webrtc_bridge::WebRTCBridge::start(
            bp,
            config.server_port,
            config.http_port,
            lan_ip,
            &cert_dir,
        )
        .await
        {
            Ok(b) => {
                log::info!("WebRTC bridge started on port {}", bp);
                Some(b)
            }
            Err(e) => {
                log::error!("Failed to start WebRTC bridge: {}", e);
                None
            }
        };
        Some(bridge)
    } else {
        None
    };

    let web_app_handle = if run_web {
        let lan_ip = local_ip_address::local_ip()
            .map(|ip| ip.to_string())
            .unwrap_or_else(|_| "127.0.0.1".to_string());
        let cert_dir = data_dir_cached.join("certs");
        if let Err(e) = cert_gen::ensure_cert(&cert_dir, &lan_ip) {
            log::error!("Failed to generate cert for web app: {}", e);
        }

        let bp = bridge_port_override.unwrap_or(config.webrtc_port);
        let effective_web_dir = config
            .custom_web_dir
            .as_ref()
            .map(PathBuf::from)
            .unwrap_or_else(|| retouched_server::web_manager::web_app_dir(&data_dir_cached));
        let effective_web_dir = if effective_web_dir.join("dist").exists() {
            effective_web_dir.join("dist")
        } else {
            effective_web_dir
        };

        if !effective_web_dir.join("index.html").exists() {
            log::warn!(
                "Retouched Web not found at {}. Run 'retouched-server web download' first.",
                effective_web_dir.display()
            );
        }

        Some(retouched_server::web_app_server::spawn_web_app_server(
            effective_web_dir,
            bp,
            cert_dir,
        ))
    } else {
        None
    };

    let server = Server::new(config);
    let shutdown_tx = server.shutdown_handle();

    tokio::spawn(async move {
        tokio::signal::ctrl_c().await.ok();
        log::info!("Ctrl+C received, shutting down...");
        let _ = shutdown_tx.send(());
        if let Some(bridge_wrapped) = bridge_opt {
            if let Some(bridge) = bridge_wrapped {
                bridge.shutdown();
            }
        }
        if let Some(handle) = web_app_handle {
            handle.graceful_shutdown(Some(std::time::Duration::from_secs(2)));
        }
    });

    server.run().await?;

    log::info!("Server stopped");
    Ok(())
}

fn cli_patch(apk: PathBuf, ip: String, data_dir: Option<PathBuf>) {
    let data_dir = app_dirs::app_data_dir(data_dir.as_deref());
    let step = touchy_patcher::new_shared_step();
    match touchy_patcher::run_patch_pipeline(&apk, &ip, &data_dir, &step) {
        Ok(out) => println!("Patched APK: {}", out.display()),
        Err(e) => {
            eprintln!("Patch failed: {}", e);
            std::process::exit(1);
        }
    }
}

fn cli_tools(action: ToolsAction, data_dir: Option<PathBuf>) {
    let data_dir = app_dirs::app_data_dir(data_dir.as_deref());
    match action {
        ToolsAction::Download => match touchy_patcher::ensure_tools(&data_dir) {
            Ok(_) => println!("Tools downloaded successfully."),
            Err(e) => {
                eprintln!("Failed to download tools: {}", e);
                std::process::exit(1);
            }
        },
    }
}

fn cli_web(action: WebAction, config_path: &Option<PathBuf>, data_dir: Option<PathBuf>) {
    let resolved_config_path = config_path.clone().unwrap_or_else(default_config_path);
    let config = Config::from_file(&resolved_config_path).unwrap_or_default();
    let data_dir_resolved = app_dirs::app_data_dir(data_dir.as_deref());

    fn resolve_web_dir(
        cli_dir: Option<PathBuf>,
        config: &Config,
        data_dir: &std::path::Path,
    ) -> PathBuf {
        cli_dir
            .or_else(|| config.custom_web_dir.as_ref().map(PathBuf::from))
            .unwrap_or_else(|| web_manager::web_app_dir(data_dir))
    }

    match action {
        WebAction::Download { dir } => {
            let target = resolve_web_dir(dir, &config, &data_dir_resolved);
            match web_manager::download_web_app(&target) {
                Ok(tag) => println!("Downloaded retouched_web {} to {}", tag, target.display()),
                Err(e) => {
                    eprintln!("Download failed: {}", e);
                    std::process::exit(1);
                }
            }
        }
    }
}

fn cli_hosts(action: HostsAction) {
    use setup::hosts::{
        HostEntryState, apply_hosts_entries, check_hosts_entries, remove_hosts_entries,
    };
    match action {
        HostsAction::Status => {
            for (domain, state) in check_hosts_entries() {
                match state {
                    HostEntryState::Present(ip) => println!("{} -> {}", domain, ip),
                    HostEntryState::Missing => println!("{} (missing)", domain),
                }
            }
        }
        HostsAction::Apply { ip } => {
            let ip = ip.unwrap_or_else(|| "127.0.0.1".to_string());
            match apply_hosts_entries(&ip) {
                Ok(()) => println!("Hosts entries applied for {}", ip),
                Err(e) => {
                    eprintln!("Failed to apply hosts entries: {}", e);
                    std::process::exit(1);
                }
            }
        }
        HostsAction::Remove => match remove_hosts_entries() {
            Ok(()) => println!("Hosts entries removed."),
            Err(e) => {
                eprintln!("Failed to remove hosts entries: {}", e);
                std::process::exit(1);
            }
        },
    }
}

fn cli_trust(action: TrustAction) {
    use setup::flash_trust::{add_trusted_dir, read_trusted_dirs, remove_trusted_dir};
    match action {
        TrustAction::List => {
            let dirs = read_trusted_dirs();
            if dirs.is_empty() {
                println!("No trusted directories.");
            } else {
                for d in &dirs {
                    println!("{}", d);
                }
            }
        }
        TrustAction::Add { dir } => {
            let native = retouched_server::path_util::to_native_path(&dir.to_string_lossy());
            match add_trusted_dir(&native.to_string_lossy()) {
                Ok(()) => println!("Added trusted directory: {}", native.display()),
                Err(e) => {
                    eprintln!("Failed to add trusted directory: {}", e);
                    std::process::exit(1);
                }
            }
        }
        TrustAction::Remove { dir } => {
            let native = retouched_server::path_util::to_native_path(&dir.to_string_lossy());
            match remove_trusted_dir(&native.to_string_lossy()) {
                Ok(()) => println!("Removed trusted directory: {}", native.display()),
                Err(e) => {
                    eprintln!("Failed to remove trusted directory: {}", e);
                    std::process::exit(1);
                }
            }
        }
    }
}

fn cli_firewall(action: FirewallAction) {
    use setup::firewall::{close_ports, detect_backend, open_ports};
    let backend = detect_backend();
    match action {
        FirewallAction::Status => println!("Firewall backend: {}", backend.name()),
        FirewallAction::Open => match open_ports(&backend) {
            Ok(()) => println!("Firewall ports opened."),
            Err(e) => {
                eprintln!("Failed to open ports: {}", e);
                std::process::exit(1);
            }
        },
        FirewallAction::Close => match close_ports(&backend) {
            Ok(()) => println!("Firewall ports closed."),
            Err(e) => {
                eprintln!("Failed to close ports: {}", e);
                std::process::exit(1);
            }
        },
    }
}

fn parse_log_level(level_str: &str, debug: bool) -> log::LevelFilter {
    if debug {
        return log::LevelFilter::Debug;
    }
    match level_str.to_uppercase().as_str() {
        "TRACE" => log::LevelFilter::Trace,
        "DEBUG" => log::LevelFilter::Debug,
        "INFO" => log::LevelFilter::Info,
        "WARN" | "WARNING" => log::LevelFilter::Warn,
        "ERROR" => log::LevelFilter::Error,
        _ => log::LevelFilter::Info,
    }
}

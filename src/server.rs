// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 ddavef/KinteLiX retouched-server

use std::collections::HashMap;
use std::sync::Arc;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpListener, TcpStream};
use tokio::sync::{Mutex, RwLock, broadcast};

use bronze_monkey::devices::bm_address::BMAddress;
use bronze_monkey::devices::device_core::DeviceCore;
use bronze_monkey::engine::actions::{Action, LogLevel, RegistryEventKind};
use bronze_monkey::engine::processing::Engine;
use bronze_monkey::externals::handshake::Handshake;
use bronze_monkey::messages::bm_encoding::Value;
use bronze_monkey::types::device_type::DeviceType;

use crate::config::Config;
use crate::shared_state::{ConnectedClient, SharedState};

struct Client {
    device_id: Option<String>,
    device_name: Option<String>,
    device_type_code: Option<i32>,
    tx: tokio::sync::mpsc::Sender<Vec<u8>>,
    addr: std::net::SocketAddr,
    connected_at: std::time::Instant,
    app_id: Option<String>,
    domain: Option<String>,
    slot_id: Option<i16>,
    current_players: Option<i16>,
    max_players: Option<i16>,
}

struct ServerState {
    engine: Mutex<Engine>,
    clients: RwLock<HashMap<u64, Client>>,
    device_to_client: RwLock<HashMap<String, u64>>,
    next_client_id: std::sync::atomic::AtomicU64,
    gui_shared: Option<Arc<SharedState>>,
}

impl ServerState {
    async fn sync_clients_to_gui(&self) {
        if let Some(ref shared) = self.gui_shared {
            let clients = self.clients.read().await;
            let snapshot: Vec<ConnectedClient> = clients
                .iter()
                .map(|(&_id, c)| ConnectedClient {
                    device_id: c.device_id.clone(),
                    device_name: c.device_name.clone(),
                    device_type_code: c.device_type_code,
                    addr: c.addr.to_string(),
                    connected_at: c.connected_at,
                    domain: c.domain.clone(),
                    app_id: c.app_id.clone(),
                    slot_id: c.slot_id,
                    current_players: c.current_players,
                    max_players: c.max_players,
                })
                .collect();
            shared.set_clients(snapshot);
        }
    }
}

pub struct Server {
    config: Config,
    state: Arc<ServerState>,
    shutdown_tx: broadcast::Sender<()>,
    server_device_id: String,
}

impl Server {
    pub fn new(config: Config) -> Self {
        Self::with_shared(config, None)
    }

    pub fn with_shared(config: Config, gui_shared: Option<Arc<SharedState>>) -> Self {
        let (shutdown_tx, _) = broadcast::channel(1);
        let server_device_id = uuid::Uuid::new_v4().to_string();

        let mut engine = Engine::new();
        let core = DeviceCore {
            device_id: server_device_id.clone(),
            device_name: "RetouchedServer".into(),
            device_type: DeviceType::Server,
            address: Some(BMAddress {
                address: config.server_host.clone(),
                unreliable_port: config.server_port as i32,
                reliable_port: config.server_port as i32,
            }),
        };
        engine.init_local_device(core);

        let state = Arc::new(ServerState {
            engine: Mutex::new(engine),
            clients: RwLock::new(HashMap::new()),
            device_to_client: RwLock::new(HashMap::new()),
            next_client_id: std::sync::atomic::AtomicU64::new(1),
            gui_shared,
        });

        Self {
            config,
            state,
            shutdown_tx,
            server_device_id,
        }
    }

    pub fn shutdown_handle(&self) -> broadcast::Sender<()> {
        self.shutdown_tx.clone()
    }

    pub async fn run(&self) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let addr = format!("{}:{}", self.config.server_host, self.config.server_port);
        let listener = TcpListener::bind(&addr).await?;
        log::info!("Server listening on {}", addr);

        let mut shutdown_rx = self.shutdown_tx.subscribe();

        loop {
            tokio::select! {
                result = listener.accept() => {
                    match result {
                        Ok((stream, addr)) => {
                            log::info!("New connection from {}", addr);
                            let state = self.state.clone();
                            let max_packet = self.config.max_packet_size;
                            let mut shutdown_rx2 = self.shutdown_tx.subscribe();
                            let server_device_id = self.server_device_id.clone();
                            tokio::spawn(async move {
                                if let Err(e) = handle_client(
                                    stream, addr, state, max_packet,
                                    &mut shutdown_rx2, &server_device_id,
                                ).await {
                                    log::error!("Client {} error: {}", addr, e);
                                }
                            });
                        }
                        Err(e) => {
                            log::error!("Accept error: {}", e);
                        }
                    }
                }
                _ = shutdown_rx.recv() => {
                    log::info!("Shutdown signal received");
                    break;
                }
            }
        }

        let clients = self.state.clients.read().await;
        for (_, client) in clients.iter() {
            let _ = client.tx.send(Vec::new()).await;
        }

        Ok(())
    }
}

async fn handle_client(
    mut stream: TcpStream,
    addr: std::net::SocketAddr,
    state: Arc<ServerState>,
    max_packet_size: usize,
    shutdown_rx: &mut broadcast::Receiver<()>,
    server_device_id: &str,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let version_bytes = Handshake::default().to_bytes();
    stream.write_all(&version_bytes).await?;
    log::debug!("Sent handshake to {}", addr);

    let mut handshake_buf = [0u8; 12];
    match tokio::time::timeout(
        std::time::Duration::from_secs(2),
        stream.read_exact(&mut handshake_buf),
    )
    .await
    {
        Ok(Ok(_)) => log::debug!("Handshake from {}: {:02x?}", addr, &handshake_buf),
        Ok(Err(e)) => {
            log::warn!("Handshake read error from {}: {}", addr, e);
            return Ok(());
        }
        Err(_) => log::debug!("No handshake reply from {} (timeout), proceeding", addr),
    }

    let (tx, mut rx) = tokio::sync::mpsc::channel::<Vec<u8>>(256);
    let client_id = state
        .next_client_id
        .fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    {
        let mut clients = state.clients.write().await;
        clients.insert(
            client_id,
            Client {
                device_id: None,
                device_name: None,
                device_type_code: None,
                tx: tx.clone(),
                addr,
                connected_at: std::time::Instant::now(),
                app_id: None,
                domain: None,
                slot_id: None,
                current_players: None,
                max_players: None,
            },
        );
    }
    state.sync_clients_to_gui().await;

    let (mut reader, mut writer) = stream.into_split();
    let writer_handle = tokio::spawn(async move {
        while let Some(data) = rx.recv().await {
            if data.is_empty() {
                break;
            }
            if writer.write_all(&data).await.is_err() {
                break;
            }
        }
    });

    let mut buffer = Vec::with_capacity(4096);
    let mut read_buf = [0u8; 4096];
    let server_device_id = server_device_id.to_string();

    loop {
        tokio::select! {
            n = reader.read(&mut read_buf) => {
                match n {
                    Ok(0) => { log::info!("Client {} disconnected", addr); break; }
                    Ok(n) => {
                        buffer.extend_from_slice(&read_buf[..n]);
                        loop {
                            if buffer.len() < 4 { break; }
                            let pkt_size = u32::from_le_bytes([buffer[0], buffer[1], buffer[2], buffer[3]]) as usize;
                            if pkt_size > max_packet_size { log::error!("Packet too large from {}", addr); break; }
                            if buffer.len() < 4 + pkt_size { break; }

                            let full_packet: Vec<u8> = buffer[..4 + pkt_size].to_vec();
                            buffer.drain(..4 + pkt_size);

                            let actions = {
                                let mut engine = state.engine.lock().await;
                                engine.process_incoming(&full_packet)
                            };
                            route_actions(&state, &actions, client_id, &server_device_id).await;
                        }
                    }
                    Err(e) => { log::error!("Read error from {}: {}", addr, e); break; }
                }
            }
            _ = shutdown_rx.recv() => { log::info!("Client {} shutting down", addr); break; }
        }
    }

    {
        let mut clients = state.clients.write().await;
        if let Some(client) = clients.remove(&client_id) {
            if let Some(dev_id) = &client.device_id {
                state.device_to_client.write().await.remove(dev_id);
                let disconnect_actions = state.engine.lock().await.drop_device(dev_id);
                route_send_actions(&state, &disconnect_actions, &clients).await;
                if let Some(ref shared) = state.gui_shared {
                    shared.metrics_connections.lock().unwrap().remove(dev_id);
                    shared.pending_connections.lock().unwrap().remove(dev_id);
                    shared
                        .metrics_connections
                        .lock()
                        .unwrap()
                        .retain(|_, game_did| game_did != dev_id);
                }
                log::info!("Client {} (device={}) cleaned up", addr, dev_id);
            }
        }
    }
    state.sync_clients_to_gui().await;

    let _ = tx.send(Vec::new()).await;
    let _ = writer_handle.await;
    Ok(())
}

async fn route_send_actions(
    _state: &Arc<ServerState>,
    actions: &[Action],
    clients: &HashMap<u64, Client>,
) {
    for action in actions {
        if let Action::Send {
            target_device_id,
            payload,
            ..
        } = action
        {
            for (_, c) in clients.iter() {
                if c.device_id.as_deref() == Some(target_device_id.as_str()) {
                    let _ = c.tx.send(payload.clone()).await;
                    break;
                }
            }
        }
    }
}

async fn send_actions_to_client(client: &Client, actions: &[Action]) {
    for a in actions {
        if let Action::Send { payload, .. } = a {
            let _ = client.tx.send(payload.clone()).await;
        }
    }
}

async fn route_actions(
    state: &Arc<ServerState>,
    actions: &[Action],
    source_client_id: u64,
    _server_device_id: &str,
) {
    for action in actions {
        match action {
            Action::Send {
                target_device_id,
                payload,
                ..
            } => {
                let clients = state.clients.read().await;
                let d2c = state.device_to_client.read().await;

                if let Some(source) = clients.get(&source_client_id) {
                    if source.device_id.as_deref() == Some(target_device_id.as_str()) {
                        let _ = source.tx.send(payload.clone()).await;
                        continue;
                    }
                }
                if let Some(&target_cid) = d2c.get(target_device_id) {
                    if let Some(target) = clients.get(&target_cid) {
                        let _ = target.tx.send(payload.clone()).await;
                    }
                } else {
                    for (cid, client) in clients.iter() {
                        if *cid != source_client_id {
                            let _ = client.tx.send(payload.clone()).await;
                        }
                    }
                }
            }

            Action::UpdateRegistry { record } => {
                let dev_id = record.core.device_id.clone();
                let dev_name = record.core.device_name.clone();
                let dev_type = record.core.device_type.code();

                {
                    let mut d2c = state.device_to_client.write().await;
                    if let Some(&old_cid) = d2c.get(&dev_id) {
                        if old_cid != source_client_id {
                            log::info!(
                                "Device {} re-registered: evicting stale client {} in favour of {}",
                                dev_id,
                                old_cid,
                                source_client_id
                            );
                            let mut clients = state.clients.write().await;
                            if let Some(old_client) = clients.remove(&old_cid) {
                                let _ = old_client.tx.send(Vec::new()).await;
                            }
                        }
                    }
                    d2c.insert(dev_id.clone(), source_client_id);
                }

                let mut clients = state.clients.write().await;
                if let Some(c) = clients.get_mut(&source_client_id) {
                    c.device_id = Some(dev_id.clone());
                    c.device_name = Some(dev_name.clone());
                    c.device_type_code = Some(dev_type);
                    if let Some(ref info) = record.info {
                        c.app_id = Some(info.app_id.clone());
                        c.slot_id = Some(info.slot_id);
                        c.current_players = info.current_players;
                        c.max_players = info.max_players;
                    }
                }
                drop(clients);
                log::info!(
                    "Registry updated: {} ({}) type={}",
                    dev_name,
                    dev_id,
                    dev_type
                );
                state.sync_clients_to_gui().await;
            }

            Action::RegistryEvent { kind, infos, .. } => match kind {
                RegistryEventKind::OnRegister => {
                    log::info!("Registry register: {} infos", infos.len());
                    let engine = state.engine.lock().await;
                    let d2c = state.device_to_client.read().await;
                    let mut clients = state.clients.write().await;
                    for info in infos {
                        let did = &info.device.device_id;
                        let latest = engine.registry().get(did).and_then(|r| r.info.as_ref());
                        let src = latest.unwrap_or(info);
                        if let Some(&cid) = d2c.get(did.as_str()) {
                            if let Some(c) = clients.get_mut(&cid) {
                                c.app_id = Some(src.app_id.clone());
                                c.slot_id = Some(src.slot_id);
                                c.current_players = src.current_players;
                                c.max_players = src.max_players;
                            }
                        }
                    }
                    drop(clients);
                    drop(d2c);
                    drop(engine);
                    state.sync_clients_to_gui().await;
                }
                RegistryEventKind::OnList => log::debug!("Registry list: {} infos", infos.len()),
                RegistryEventKind::OnHostConnected | RegistryEventKind::OnHostUpdate => {
                    log::info!("Host connected/update: {} infos", infos.len());
                    let engine = state.engine.lock().await;
                    let d2c = state.device_to_client.read().await;
                    let mut clients = state.clients.write().await;
                    for info in infos {
                        let did = &info.device.device_id;
                        let latest = engine.registry().get(did).and_then(|r| r.info.as_ref());
                        let src = latest.unwrap_or(info);
                        if let Some(&cid) = d2c.get(did.as_str()) {
                            if let Some(c) = clients.get_mut(&cid) {
                                c.app_id = Some(src.app_id.clone());
                                c.slot_id = Some(src.slot_id);
                                c.current_players = src.current_players;
                                c.max_players = src.max_players;
                            }
                        }
                    }
                    drop(clients);
                    drop(d2c);
                    drop(engine);
                    state.sync_clients_to_gui().await;
                }
                RegistryEventKind::OnHostDisconnected => {
                    log::info!("Host disconnected");
                    let disconnected_ids: Vec<String> =
                        infos.iter().map(|i| i.device.device_id.clone()).collect();
                    if !disconnected_ids.is_empty() {
                        if let Some(ref shared) = state.gui_shared {
                            let mut mc = shared.metrics_connections.lock().unwrap();
                            for did in &disconnected_ids {
                                mc.retain(|_, game_did| game_did != did);
                            }
                        }
                        state.sync_clients_to_gui().await;
                    }
                }
                RegistryEventKind::DeviceConnectRequested => {
                    if let Some(host_info) = infos.first() {
                        let clients = state.clients.read().await;
                        let source_dev_id = clients
                            .get(&source_client_id)
                            .and_then(|c| c.device_id.clone());
                        let source_name = clients
                            .get(&source_client_id)
                            .and_then(|c| c.device_name.as_deref())
                            .unwrap_or("?");
                        let game_device_id = &host_info.device.device_id;
                        log::info!(
                            "Device connect requested: {} -> {}",
                            source_name,
                            game_device_id
                        );
                        if let Some(ctrl_did) = source_dev_id {
                            if let Some(shared) = &state.gui_shared {
                                shared
                                    .pending_connections
                                    .lock()
                                    .unwrap()
                                    .insert(ctrl_did, game_device_id.clone());
                            }
                        }
                    }
                }
            },

            Action::Invoke { method, params, .. } => {
                if method == "registry.register" {
                    let domain = params.iter().find_map(|p| {
                        if let Value::String(s) = p {
                            Some(s.clone())
                        } else {
                            None
                        }
                    });
                    if domain.is_some() {
                        let mut clients = state.clients.write().await;
                        if let Some(c) = clients.get_mut(&source_client_id) {
                            c.domain = domain;
                        }
                        drop(clients);
                        state.sync_clients_to_gui().await;
                    }
                }
                handle_invoke(state, source_client_id, method, params).await;
            }

            Action::Log { level, message } => match level {
                LogLevel::Trace => log::trace!("[BMEngine] {}", message),
                LogLevel::Debug => log::debug!("[BMEngine] {}", message),
                LogLevel::Info => log::info!("[BMEngine] {}", message),
                LogLevel::Warn => log::warn!("[BMEngine] {}", message),
                LogLevel::Error => log::error!("[BMEngine] {}", message),
            },

            _ => {}
        }
    }
}

async fn handle_invoke(
    state: &Arc<ServerState>,
    source_client_id: u64,
    method: &str,
    _params: &[Value],
) {
    match method {
        "registry.register" => {
            let clients = state.clients.read().await;
            let info = clients
                .get(&source_client_id)
                .map(|c| {
                    format!(
                        "{} ({}) type={}",
                        c.device_name.as_deref().unwrap_or("?"),
                        c.device_id.as_deref().unwrap_or("?"),
                        c.device_type_code.unwrap_or(0)
                    )
                })
                .unwrap_or_default();
            log::info!("registry.register: {}", info);
        }

        "registry.list" => {
            log::debug!("registry.list from client {}", source_client_id);
        }

        "ping" => {
            let dev_id = {
                let clients = state.clients.read().await;
                clients
                    .get(&source_client_id)
                    .and_then(|c| c.device_id.clone())
            };
            if let Some(did) = dev_id {
                let mut engine = state.engine.lock().await;
                use bronze_monkey::types::packet_type::PacketType;
                let acts = engine.make_packet(&did, 1, Some(0), PacketType::Echo, None);
                let clients = state.clients.read().await;
                if let Some(c) = clients.get(&source_client_id) {
                    send_actions_to_client(c, &acts).await;
                }
            }
        }

        "registry.relay" | "registry.update" => {
            log::debug!("{} from client {}", method, source_client_id);
        }

        _ => log::debug!("Unhandled invoke: {}", method),
    }
}

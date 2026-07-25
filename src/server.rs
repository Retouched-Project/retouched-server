// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 ddavef/KinteLiX retouched-server

use std::collections::HashMap;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpListener, TcpStream};
use tokio::sync::{Mutex, RwLock, broadcast};

use bronze_monkey::codec::externals::bm_registry_info::BMRegistryInfo;
use bronze_monkey::codec::externals::handshake::Handshake;
use bronze_monkey::devices::bm_address::BMAddress;
use bronze_monkey::devices::device_core::DeviceCore;
use bronze_monkey::engine::methods::DEVICE_CONNECT_REQUESTED;
use bronze_monkey::engine::{DeviceRecord, Engine, Event, Outgoing, ProcessOutput};
use bronze_monkey::types::device_type::DeviceType;

use crate::config::Config;
use crate::shared_state::{ConnectedClient, SharedState};

const CROSS_DOMAIN_POLICY: &str = r#"<?xml version="1.0"?><cross-domain-policy><allow-access-from domain="*" to-ports="1008-49151" /></cross-domain-policy>"#;

const CONTROLLER_POLICY_PORT: u16 = 9010;

// set by the redirect probe so its own loopback policy fetch is not logged
static POLICY_PROBE_ACTIVE: AtomicBool = AtomicBool::new(false);

pub fn set_policy_probe_active(active: bool) {
    POLICY_PROBE_ACTIVE.store(active, Ordering::Relaxed);
}

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
    announced_address: Option<String>,
    reliable_port: Option<i32>,
    unreliable_port: Option<i32>,
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
                    announced_address: c.announced_address.clone(),
                    reliable_port: c.reliable_port,
                    unreliable_port: c.unreliable_port,
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
}

impl Server {
    pub fn new(config: Config) -> Self {
        Self::with_shared(config, None)
    }

    pub fn with_shared(config: Config, gui_shared: Option<Arc<SharedState>>) -> Self {
        let (shutdown_tx, _) = broadcast::channel(1);
        let server_device_id = bronze_monkey::identity::generate_device_id();

        let mut engine = Engine::new();
        let core = DeviceCore {
            device_id: server_device_id,
            device_name: "RetouchedServer".into(),
            device_type: DeviceType::Server,
            address: Some(BMAddress {
                address: config.server_host.clone(),
                unreliable_port: config.server_port as i32,
                reliable_port: config.server_port as i32,
            }),
        };
        engine.init_local_device(core);
        engine.configure_roles(true, None);

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
        }
    }

    pub fn shutdown_handle(&self) -> broadcast::Sender<()> {
        self.shutdown_tx.clone()
    }

    fn spawn_controller_policy_listener(&self) {
        let host = self.config.server_host.clone();
        let mut shutdown_rx = self.shutdown_tx.subscribe();
        tokio::spawn(async move {
            let addr = format!("{}:{}", host, CONTROLLER_POLICY_PORT);
            let listener = match TcpListener::bind(&addr).await {
                Ok(l) => {
                    log::info!("Controller policy server listening on {}", addr);
                    l
                }
                Err(e) => {
                    log::warn!("Could not bind controller policy port {}: {}", addr, e);
                    return;
                }
            };
            loop {
                tokio::select! {
                    accepted = listener.accept() => {
                        if let Ok((stream, peer)) = accepted {
                            tokio::spawn(serve_policy(stream, peer));
                        }
                    }
                    _ = shutdown_rx.recv() => break,
                }
            }
        });
    }

    pub async fn run(&self) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let addr = format!("{}:{}", self.config.server_host, self.config.server_port);
        let listener = TcpListener::bind(&addr).await?;
        log::info!("Server listening on {}", addr);

        let mut shutdown_rx = self.shutdown_tx.subscribe();

        self.spawn_controller_policy_listener();

        loop {
            tokio::select! {
                result = listener.accept() => {
                    match result {
                        Ok((stream, addr)) => {
                            let _ = stream.set_nodelay(true);
                            if !(POLICY_PROBE_ACTIVE.load(Ordering::Relaxed) && addr.ip().is_loopback()) {
                                log::info!("New connection from {}", addr);
                            }
                            let state = self.state.clone();
                            let max_packet = self.config.max_packet_size;
                            let mut shutdown_rx2 = self.shutdown_tx.subscribe();
                            tokio::spawn(async move {
                                if let Err(e) = handle_client(
                                    stream, addr, state, max_packet, &mut shutdown_rx2,
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

async fn serve_policy(mut stream: TcpStream, peer: std::net::SocketAddr) {
    let mut buf = [0u8; 64];
    let _ =
        tokio::time::timeout(std::time::Duration::from_millis(500), stream.read(&mut buf)).await;
    log::info!("Serving socket policy file to {}", peer);
    let _ = stream.write_all(CROSS_DOMAIN_POLICY.as_bytes()).await;
    let _ = stream.write_all(&[0]).await;
    let _ = stream.flush().await;
}

async fn handle_client(
    mut stream: TcpStream,
    addr: std::net::SocketAddr,
    state: Arc<ServerState>,
    max_packet_size: usize,
    shutdown_rx: &mut broadcast::Receiver<()>,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let mut peek_buf = [0u8; 1];
    if let Ok(Ok(n)) = tokio::time::timeout(
        std::time::Duration::from_millis(200),
        stream.peek(&mut peek_buf),
    )
    .await
    {
        if n > 0 && peek_buf[0] == b'<' {
            let mut req = [0u8; 23];
            if let Ok(Ok(_)) = tokio::time::timeout(
                std::time::Duration::from_millis(200),
                stream.read_exact(&mut req),
            )
            .await
            {
                if req.starts_with(b"<policy-file-request") {
                    if !(POLICY_PROBE_ACTIVE.load(Ordering::Relaxed) && addr.ip().is_loopback()) {
                        log::info!("Serving socket policy file to {}", addr);
                    }
                    let _ = stream.write_all(CROSS_DOMAIN_POLICY.as_bytes()).await;
                    let _ = stream.write_all(&[0]).await;
                    return Ok(());
                }
            }
        }
    }

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
                announced_address: None,
                reliable_port: None,
                unreliable_port: None,
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

                            let output = {
                                let mut engine = state.engine.lock().await;
                                engine.process_incoming(&full_packet)
                            };
                            route_output(&state, &output, client_id).await;
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
                let disconnect_outgoings = state.engine.lock().await.drop_device(dev_id);
                route_send_outgoings(&disconnect_outgoings, &clients).await;
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

async fn route_output(state: &Arc<ServerState>, output: &ProcessOutput, source_client_id: u64) {
    for ev in &output.events {
        match ev {
            Event::PeerSeen { record } | Event::PeerConnected { record } => {
                bind_device(state, source_client_id, record).await;
            }
            Event::PeerRegistered { info, domain, .. } => {
                log::info!(
                    "Registered: {} appId={} slot={}",
                    info.device.device_name,
                    info.app_id,
                    info.slot_id
                );
                update_client_registry(state, info, domain.as_deref()).await;
            }
            Event::HostUpdated { info } => {
                update_client_registry(state, info, None).await;
            }
            Event::Relayed {
                destination,
                method,
                ..
            } if method.as_str() == DEVICE_CONNECT_REQUESTED => {
                record_pending_connection(state, source_client_id, destination).await;
            }
            Event::Invoke { method, .. } => {
                log::debug!(
                    "Unhandled invoke from client {}: {}",
                    source_client_id,
                    method
                );
            }
            _ => {}
        }
    }

    let routes: Vec<(u64, Vec<u8>)> = {
        let d2c = state.device_to_client.read().await;
        output
            .outgoings
            .iter()
            .filter_map(|o| {
                d2c.get(&o.target_device_id)
                    .map(|&cid| (cid, o.payload.clone()))
            })
            .collect()
    };
    if routes.is_empty() {
        return;
    }
    let clients = state.clients.read().await;
    for (cid, payload) in routes {
        if let Some(c) = clients.get(&cid) {
            let _ = c.tx.send(payload).await;
        }
    }
}

async fn route_send_outgoings(outgoings: &[Outgoing], clients: &HashMap<u64, Client>) {
    for o in outgoings {
        for (_, c) in clients.iter() {
            if c.device_id.as_deref() == Some(o.target_device_id.as_str()) {
                let _ = c.tx.send(o.payload.clone()).await;
                break;
            }
        }
    }
}

async fn bind_device(state: &Arc<ServerState>, source_client_id: u64, record: &DeviceRecord) {
    let dev_id = record.core.device_id.clone();
    if dev_id.is_empty() {
        return;
    }

    {
        let clients = state.clients.read().await;
        if clients
            .get(&source_client_id)
            .and_then(|c| c.device_id.as_deref())
            == Some(dev_id.as_str())
        {
            return;
        }
    }

    let stale = {
        let mut d2c = state.device_to_client.write().await;
        d2c.insert(dev_id.clone(), source_client_id)
            .filter(|&old| old != source_client_id)
    };
    if let Some(old_cid) = stale {
        let mut clients = state.clients.write().await;
        if let Some(old) = clients.remove(&old_cid) {
            let _ = old.tx.send(Vec::new()).await;
        }
        log::info!(
            "Device {} re-registered: evicting stale client {} in favour of {}",
            dev_id,
            old_cid,
            source_client_id
        );
    }

    {
        let mut clients = state.clients.write().await;
        if let Some(c) = clients.get_mut(&source_client_id) {
            c.device_id = Some(dev_id.clone());
            c.device_name = Some(record.core.device_name.clone());
            c.device_type_code = Some(record.core.device_type.code());
        }
    }
    log::info!(
        "Device connected: {} ({}) type={}",
        record.core.device_name,
        dev_id,
        record.core.device_type.code()
    );
    state.sync_clients_to_gui().await;
}

async fn update_client_registry(
    state: &Arc<ServerState>,
    info: &BMRegistryInfo,
    domain: Option<&str>,
) {
    let cid = state
        .device_to_client
        .read()
        .await
        .get(&info.device.device_id)
        .copied();
    let Some(cid) = cid else {
        return;
    };
    {
        let mut clients = state.clients.write().await;
        if let Some(c) = clients.get_mut(&cid) {
            c.app_id = Some(info.app_id.clone());
            c.slot_id = Some(info.slot_id);
            c.current_players = info.current_players;
            c.max_players = info.max_players;
            c.announced_address = Some(info.device_address.address.clone());
            c.reliable_port = Some(info.device_address.reliable_port);
            c.unreliable_port = Some(info.device_address.unreliable_port);
            if let Some(d) = domain {
                c.domain = Some(d.to_string());
            }
        }
    }
    state.sync_clients_to_gui().await;
}

async fn record_pending_connection(
    state: &Arc<ServerState>,
    source_client_id: u64,
    game_device_id: &str,
) {
    let game_device_id = game_device_id.to_string();
    let (source_dev, source_name) = {
        let clients = state.clients.read().await;
        let c = clients.get(&source_client_id);
        (
            c.and_then(|c| c.device_id.clone()),
            c.and_then(|c| c.device_name.clone()).unwrap_or_default(),
        )
    };
    log::info!(
        "Device connect requested: {} -> {}",
        source_name,
        game_device_id
    );
    if let Some(ctrl_did) = source_dev {
        if let Some(shared) = &state.gui_shared {
            shared
                .pending_connections
                .lock()
                .unwrap()
                .insert(ctrl_did, game_device_id);
        }
    }
}

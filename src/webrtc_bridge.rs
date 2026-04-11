// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 ddavef/KinteLiX retouched-server

use std::net::SocketAddr;
use std::path::Path;
use std::sync::Arc;

use axum::Router;
use axum::extract::State;
use axum::http::StatusCode;
use axum::response::IntoResponse;
use axum::routing::{any, get, post};
use serde::{Deserialize, Serialize};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpListener, TcpStream, UdpSocket};
use tokio::sync::{Mutex, Notify, broadcast};
use tower_http::cors::{AllowOrigin, CorsLayer};

use webrtc::api::APIBuilder;
use webrtc::api::media_engine::MediaEngine;
use webrtc::api::setting_engine::SettingEngine;
use webrtc::data_channel::RTCDataChannel;
use webrtc::data_channel::data_channel_message::DataChannelMessage;
use webrtc::ice_transport::ice_server::RTCIceServer;
use webrtc::peer_connection::configuration::RTCConfiguration;
use webrtc::peer_connection::sdp::session_description::RTCSessionDescription;

#[derive(Clone)]
struct BridgeState {
    registry_addr: SocketAddr,
    http_addr: SocketAddr,
    announce_host: String,
    webrtc_api: Arc<webrtc::api::API>,
    _shutdown: broadcast::Sender<()>,
}

#[derive(Deserialize)]
struct OfferRequest {
    sdp: String,
    #[serde(rename = "type")]
    _sdp_type: String,
}

#[derive(Serialize)]
struct AnswerResponse {
    sdp: String,
    #[serde(rename = "type")]
    sdp_type: String,
}

#[derive(Deserialize)]
struct GameControlMessage {
    #[serde(rename = "type")]
    msg_type: String,
    port: Option<u16>,
}

#[derive(Serialize)]
struct PortAssignment {
    #[serde(rename = "type")]
    msg_type: String,
    port: u16,
    host: String,
}

struct ClientSession {
    game_listener: Mutex<Option<TcpListener>>,
    game_ready: Notify,
    registry_tcp: Mutex<Option<tokio::net::tcp::OwnedWriteHalf>>,
    game_tcp_write: Arc<Mutex<Option<tokio::net::tcp::OwnedWriteHalf>>>,
    game_remote_addr: Mutex<Option<std::net::IpAddr>>,
    game_udp_port: Mutex<Option<u16>>,
    game_udp_socket: UdpSocket,
}

impl ClientSession {
    async fn new() -> Result<Arc<Self>, Box<dyn std::error::Error + Send + Sync>> {
        let game_udp_socket = UdpSocket::bind("0.0.0.0:0").await?;
        Ok(Arc::new(Self {
            game_listener: Mutex::new(None),
            game_ready: Notify::new(),
            registry_tcp: Mutex::new(None),
            game_tcp_write: Arc::new(Mutex::new(None)),
            game_remote_addr: Mutex::new(None),
            game_udp_port: Mutex::new(None),
            game_udp_socket,
        }))
    }
}

pub struct WebRTCBridge {
    shutdown_tx: broadcast::Sender<()>,
}

impl WebRTCBridge {
    pub async fn start(
        bridge_port: u16,
        registry_port: u16,
        http_port: u16,
        announce_host: String,
        cert_dir: &Path,
    ) -> Result<Self, Box<dyn std::error::Error + Send + Sync>> {
        let (shutdown_tx, _) = broadcast::channel::<()>(1);

        let cert_pem = std::fs::read(cert_dir.join("server.pem"))?;
        let key_pem = std::fs::read(cert_dir.join("key.pem"))?;

        let certs = rustls_pemfile::certs(&mut &cert_pem[..]).collect::<Result<Vec<_>, _>>()?;
        let key = rustls_pemfile::private_key(&mut &key_pem[..])?
            .ok_or("no private key found in key.pem")?;

        let tls_config = rustls::ServerConfig::builder()
            .with_no_client_auth()
            .with_single_cert(certs, key)?;

        let rustls_config =
            axum_server::tls_rustls::RustlsConfig::from_config(Arc::new(tls_config));

        let mut media_engine = MediaEngine::default();
        media_engine.register_default_codecs()?;
        let setting_engine = SettingEngine::default();
        let webrtc_api = Arc::new(
            APIBuilder::new()
                .with_media_engine(media_engine)
                .with_setting_engine(setting_engine)
                .build(),
        );

        let bridge_state = BridgeState {
            registry_addr: SocketAddr::from(([127, 0, 0, 1], registry_port)),
            http_addr: SocketAddr::from(([127, 0, 0, 1], http_port)),
            announce_host,
            webrtc_api,
            _shutdown: shutdown_tx.clone(),
        };

        let cors = CorsLayer::new()
            .allow_origin(AllowOrigin::any())
            .allow_methods(tower_http::cors::Any)
            .allow_headers(tower_http::cors::Any);

        let app = Router::new()
            .route("/offer", post(handle_offer))
            .route("/bmregistry/{*tail}", any(proxy_bmregistry))
            .route("/apps/icons/{tail}", get(proxy_icons))
            .layer(cors)
            .with_state(bridge_state);

        let addr = SocketAddr::from(([0, 0, 0, 0], bridge_port));
        log::info!("WebRTC bridge listening on https://0.0.0.0:{}", bridge_port);

        let mut shutdown_rx = shutdown_tx.subscribe();
        let server_handle = axum_server::Handle::new();
        let handle_for_shutdown = server_handle.clone();

        tokio::spawn(async move {
            let _ = shutdown_rx.recv().await;
            log::info!("WebRTC bridge shutting down");
            handle_for_shutdown.shutdown();
        });

        tokio::spawn(async move {
            if let Err(e) = axum_server::bind_rustls(addr, rustls_config)
                .handle(server_handle)
                .serve(app.into_make_service())
                .await
            {
                log::error!("WebRTC bridge server error: {}", e);
            }
        });

        Ok(Self { shutdown_tx })
    }

    pub fn shutdown(&self) {
        let _ = self.shutdown_tx.send(());
    }
}

async fn handle_offer(
    State(state): State<BridgeState>,
    axum::Json(offer_req): axum::Json<OfferRequest>,
) -> impl IntoResponse {
    match create_peer_connection(state, offer_req).await {
        Ok(answer) => (StatusCode::OK, axum::Json(answer)).into_response(),
        Err(e) => {
            log::error!("WebRTC offer error: {}", e);
            (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response()
        }
    }
}

async fn create_peer_connection(
    state: BridgeState,
    offer_req: OfferRequest,
) -> Result<AnswerResponse, Box<dyn std::error::Error + Send + Sync>> {
    let config = RTCConfiguration {
        ice_servers: vec![RTCIceServer {
            urls: vec!["stun:stun.l.google.com:19302".to_string()],
            ..Default::default()
        }],
        ..Default::default()
    };

    let pc = state.webrtc_api.new_peer_connection(config).await?;
    let pc = Arc::new(pc);

    let session = ClientSession::new().await?;

    let state_for_dc = state.clone();
    let session_for_dc = session.clone();
    pc.on_data_channel(Box::new(move |dc: Arc<RTCDataChannel>| {
        let label = dc.label().to_string();
        let st = state_for_dc.clone();
        let sess = session_for_dc.clone();
        Box::pin(async move {
            match label.as_str() {
                "registry" => setup_registry_channel(dc, st, sess).await,
                "game" => setup_game_channel(dc, sess).await,
                "game-unreliable" => setup_game_unreliable_channel(dc, sess).await,
                other => log::warn!("Unknown data channel: {}", other),
            }
        })
    }));

    let session_for_state = session.clone();
    let pc_for_state = Arc::downgrade(&pc);
    pc.on_peer_connection_state_change(Box::new(move |state| {
        let sess = session_for_state.clone();
        let pc_weak = pc_for_state.clone();
        Box::pin(async move {
            use webrtc::peer_connection::peer_connection_state::RTCPeerConnectionState;
            log::info!("Peer connection state: {:?}", state);
            if state == RTCPeerConnectionState::Failed
                || state == RTCPeerConnectionState::Closed
                || state == RTCPeerConnectionState::Disconnected
            {
                log::info!("Peer connection {:?}: cleaning up client session", state);
                let _ = sess.game_listener.lock().await.take();
                if let Some(mut tcp) = sess.registry_tcp.lock().await.take() {
                    log::info!("Shutting down registry TCP for disconnected WebRTC client");
                    let _ = tcp.shutdown().await;
                }
                if let Some(mut tcp) = sess.game_tcp_write.lock().await.take() {
                    let _ = tcp.shutdown().await;
                }
                if let Some(pc) = pc_weak.upgrade() {
                    let _ = pc.close().await;
                }
            }
        })
    }));

    let offer = RTCSessionDescription::offer(offer_req.sdp)?;
    pc.set_remote_description(offer).await?;

    let answer = pc.create_answer(None).await?;
    pc.set_local_description(answer.clone()).await?;

    let (ice_done_tx, ice_done_rx) = tokio::sync::oneshot::channel::<()>();
    let ice_done_tx = Arc::new(Mutex::new(Some(ice_done_tx)));

    pc.on_ice_gathering_state_change(Box::new(move |gs| {
        let tx = ice_done_tx.clone();
        Box::pin(async move {
            if gs == webrtc::ice_transport::ice_gatherer_state::RTCIceGathererState::Complete {
                if let Some(tx) = tx.lock().await.take() {
                    let _ = tx.send(());
                }
            }
        })
    }));

    let _ = tokio::time::timeout(std::time::Duration::from_secs(5), ice_done_rx).await;

    let local_desc = pc
        .local_description()
        .await
        .ok_or("no local description after ICE gathering")?;

    Ok(AnswerResponse {
        sdp: local_desc.sdp,
        sdp_type: "answer".into(),
    })
}

async fn setup_registry_channel(
    dc: Arc<RTCDataChannel>,
    state: BridgeState,
    session: Arc<ClientSession>,
) {
    let dc_for_open = dc.clone();
    let state_for_open = state.clone();

    dc.on_open(Box::new(move || {
        let dc = dc_for_open.clone();
        let state = state_for_open.clone();
        let session = session.clone();

        Box::pin(async move {
            log::info!("Registry data channel opened");

            let tcp: TcpStream = {
                let mut last_err = None;
                let mut stream = None;
                for attempt in 1..=10 {
                    match TcpStream::connect(state.registry_addr).await {
                        Ok(s) => {
                            if attempt > 1 {
                                log::info!("Connected to registry TCP on attempt {}", attempt);
                            }
                            stream = Some(s);
                            break;
                        }
                        Err(e) => {
                            log::warn!("Registry TCP connect attempt {}/10 failed: {}", attempt, e);
                            last_err = Some(e);
                            tokio::time::sleep(std::time::Duration::from_millis(500)).await;
                        }
                    }
                }
                match stream {
                    Some(s) => s,
                    None => {
                        log::error!(
                            "Failed to connect to registry TCP after 10 attempts: {}",
                            last_err.unwrap()
                        );
                        return;
                    }
                }
            };
            let (mut tcp_read, tcp_write) = tcp.into_split();

            let game_listener: TcpListener = match TcpListener::bind("0.0.0.0:0").await {
                Ok(l) => l,
                Err(e) => {
                    log::error!("Failed to bind game listener: {}", e);
                    return;
                }
            };
            let game_port = game_listener.local_addr().unwrap().port();
            log::info!("Allocated game proxy port {} for this client", game_port);

            *session.game_listener.lock().await = Some(game_listener);
            session.game_ready.notify_waiters();

            *session.registry_tcp.lock().await = Some(tcp_write);

            let sess_for_msg = session.clone();
            dc.on_message(Box::new(move |msg: DataChannelMessage| {
                let sess = sess_for_msg.clone();
                Box::pin(async move {
                    let mut slot = sess.registry_tcp.lock().await;
                    if let Some(ref mut w) = *slot {
                        if let Err(e) = w.write_all(&msg.data).await {
                            log::error!("Registry TCP write error: {}", e);
                        }
                    }
                })
            }));

            let pa = PortAssignment {
                msg_type: "port_assignment".into(),
                port: game_port,
                host: state.announce_host.clone(),
            };
            if let Ok(json) = serde_json::to_string(&pa) {
                log::info!("Sending port_assignment to client: {}", json);
                let json_bytes = bytes::Bytes::from(json.into_bytes());
                if let Err(e) = dc.send(&json_bytes).await {
                    log::error!("Failed to send port_assignment: {}", e);
                }
            }

            let dc_for_tcp = dc.clone();
            tokio::spawn(async move {
                let mut buf = vec![0u8; 65536];
                loop {
                    match tcp_read.read(&mut buf).await {
                        Ok(0) => {
                            log::info!("Registry TCP closed");
                            break;
                        }
                        Ok(n) => {
                            if dc_for_tcp
                                .send(&bytes::Bytes::copy_from_slice(&buf[..n]))
                                .await
                                .is_err()
                            {
                                break;
                            }
                        }
                        Err(e) => {
                            log::error!("Registry TCP read error: {}", e);
                            break;
                        }
                    }
                }
            });
        })
    }));
}

async fn setup_game_channel(dc: Arc<RTCDataChannel>, session: Arc<ClientSession>) {
    let dc_for_open = dc.clone();

    let tcp_write = session.game_tcp_write.clone();

    let tw_for_msg = tcp_write.clone();
    let session_for_msg = session.clone();
    dc.on_message(Box::new(move |msg: DataChannelMessage| {
        let tw = tw_for_msg.clone();
        let sess = session_for_msg.clone();
        Box::pin(async move {
            if !msg.data.is_empty() && msg.data[0] == b'{' {
                if let Ok(text) = String::from_utf8(msg.data.to_vec()) {
                    if let Ok(ctrl) = serde_json::from_str::<GameControlMessage>(&text) {
                        match ctrl.msg_type.as_str() {
                            "disconnect_game" => {
                                log::info!("Game disconnect requested");
                                let mut slot = tw.lock().await;
                                if let Some(ref mut w) = *slot {
                                    let _ = w.shutdown().await;
                                }
                                *slot = None;
                                return;
                            }
                            "set_game_udp_port" => {
                                if let Some(port) = ctrl.port {
                                    *sess.game_udp_port.lock().await = Some(port);
                                    log::info!("Game UDP port set to {}", port);
                                }
                                return;
                            }
                            _ => {}
                        }
                    }
                }
            }
            let mut slot = tw.lock().await;
            if let Some(ref mut w) = *slot {
                if let Err(e) = w.write_all(&msg.data).await {
                    log::error!("Game TCP write error: {}", e);
                }
            }
        })
    }));

    dc.on_open(Box::new(move || {
        let dc = dc_for_open.clone();
        let session = session.clone();
        let tcp_write = tcp_write.clone();
        Box::pin(async move {
            log::info!("Game data channel opened");

            let listener: TcpListener = loop {
                {
                    let mut slot = session.game_listener.lock().await;
                    if let Some(l) = slot.take() {
                        break l;
                    }
                }
                log::info!("Game DC: waiting for registry to create game listener...");
                session.game_ready.notified().await;
            };

            let port = listener.local_addr().unwrap().port();

            loop {
                log::info!("Game DC: waiting for TCP connection on port {}...", port);

                let (game_tcp, game_addr) = match listener.accept().await {
                    Ok((s, addr)) => {
                        log::info!("Game TCP connected from {}", addr);
                        (s, addr)
                    }
                    Err(e) => {
                        log::error!("Game listener accept error: {}", e);
                        return;
                    }
                };

                *session.game_remote_addr.lock().await = Some(game_addr.ip());

                let (mut tcp_read, tcp_write_half) = game_tcp.into_split();

                *tcp_write.lock().await = Some(tcp_write_half);

                let dc2 = dc.clone();
                let tw_cleanup = tcp_write.clone();
                let relay = tokio::spawn(async move {
                    let mut buf = vec![0u8; 65536];
                    loop {
                        match tcp_read.read(&mut buf).await {
                            Ok(0) => {
                                log::info!("Game TCP closed");
                                let closed_msg = bytes::Bytes::from(
                                    r#"{"type":"game_closed"}"#.as_bytes().to_vec(),
                                );
                                let _ = dc2.send(&closed_msg).await;
                                break;
                            }
                            Ok(n) => {
                                if dc2
                                    .send(&bytes::Bytes::copy_from_slice(&buf[..n]))
                                    .await
                                    .is_err()
                                {
                                    break;
                                }
                            }
                            Err(e) => {
                                log::error!("Game TCP read error: {}", e);
                                break;
                            }
                        }
                    }
                    *tw_cleanup.lock().await = None;
                });

                let _ = relay.await;
                *session.game_udp_port.lock().await = None;
                *session.game_remote_addr.lock().await = None;
                log::info!("Game relay ended: ready for next game connection");
            }
        })
    }));
}

async fn setup_game_unreliable_channel(dc: Arc<RTCDataChannel>, session: Arc<ClientSession>) {
    dc.on_message(Box::new(move |msg: DataChannelMessage| {
        let session = session.clone();
        Box::pin(async move {
            if msg.data.len() <= 4 {
                log::warn!(
                    "UDP: received short datagram ({} bytes), dropping",
                    msg.data.len()
                );
                return;
            }
            let addr = *session.game_remote_addr.lock().await;
            let port = *session.game_udp_port.lock().await;
            let (addr, port) = match (addr, port) {
                (Some(a), Some(p)) => (a, p),
                _ => {
                    log::warn!("UDP: game address/port not yet set, dropping packet");
                    return;
                }
            };
            let payload = &msg.data[4..];
            let target = std::net::SocketAddr::new(addr, port);
            if let Err(e) = session.game_udp_socket.send_to(payload, target).await {
                log::error!("Game UDP send error: {}", e);
            }
        })
    }));
}

async fn proxy_bmregistry(
    State(state): State<BridgeState>,
    req: axum::extract::Request,
) -> axum::response::Response {
    let path = req.uri().path().to_string();
    let query = req
        .uri()
        .query()
        .map(|q| format!("?{}", q))
        .unwrap_or_default();
    let target = format!("http://{}{}{}", state.http_addr, path, query);
    let method = req.method().clone();
    let content_type = req.headers().get("content-type").cloned();
    let body = match axum::body::to_bytes(req.into_body(), 1024 * 64).await {
        Ok(b) => b,
        Err(e) => {
            log::error!("Failed to read proxy request body: {}", e);
            return (StatusCode::BAD_REQUEST, e.to_string()).into_response();
        }
    };
    proxy_request(&target, method, content_type, body).await
}

async fn proxy_icons(
    State(state): State<BridgeState>,
    axum::extract::Path(tail): axum::extract::Path<String>,
) -> axum::response::Response {
    let target = format!("http://{}/apps/icons/{}", state.http_addr, tail);
    proxy_request(&target, axum::http::Method::GET, None, bytes::Bytes::new()).await
}

async fn proxy_request(
    target: &str,
    method: axum::http::Method,
    content_type: Option<axum::http::HeaderValue>,
    body: bytes::Bytes,
) -> axum::response::Response {
    let client = reqwest::Client::builder()
        .danger_accept_invalid_certs(true)
        .build()
        .unwrap();

    let mut req = client.request(method, target);
    if let Some(ct) = content_type {
        if let Ok(v) = ct.to_str() {
            req = req.header("content-type", v);
        }
    }
    if !body.is_empty() {
        req = req.body(body);
    }
    match req.send().await {
        Ok(resp) => {
            let status =
                StatusCode::from_u16(resp.status().as_u16()).unwrap_or(StatusCode::BAD_GATEWAY);
            let ct = resp.headers().get("content-type").cloned();
            let body = resp.bytes().await.unwrap_or_default();
            let mut r = (status, body).into_response();
            if let Some(ct) = ct {
                r.headers_mut().insert("content-type", ct);
            }
            r
        }
        Err(e) => {
            log::error!("Proxy error to {}: {}", target, e);
            (StatusCode::BAD_GATEWAY, e.to_string()).into_response()
        }
    }
}

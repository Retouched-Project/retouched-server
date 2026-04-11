// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 ddavef/KinteLiX retouched-server

use axum::{
    Router,
    extract::{ConnectInfo, Path, Query, State},
    http::StatusCode,
    response::{IntoResponse, Json, Response},
    routing::{get, post},
};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::sync::Arc;

use crate::icon_cache::IconCache;
use crate::shared_state::SharedState;

pub struct HttpServerState {
    pub icon_cache: IconCache,
    pub shared: Option<Arc<SharedState>>,
    pub data_dir: PathBuf,
}

pub fn build_router(state: Arc<HttpServerState>) -> Router {
    Router::new()
        .route("/bmregistry/getInfo.jsp", get(handle_get_info))
        .route("/apps/icons/{app_id}", get(handle_icon))
        .route("/bmregistry/metrics", post(handle_metrics))
        .route("/onboard", get(handle_onboard))
        .route("/ca.crt", get(handle_ca_cert))
        .with_state(state)
}

#[derive(Deserialize)]
struct GetInfoQuery {
    #[serde(rename = "appId")]
    app_id: Option<String>,
    #[serde(rename = "deviceId")]
    device_id: Option<String>,
}

#[derive(Serialize)]
struct GetInfoResponse {
    #[serde(rename = "appId")]
    app_id: String,
    #[serde(rename = "deviceId")]
    device_id: String,
    play: u32,
    purchase: u32,
    premium: bool,
    trial: bool,
    #[serde(rename = "canPlay")]
    can_play: bool,
}

async fn handle_get_info(
    Query(params): Query<GetInfoQuery>,
) -> Result<Json<GetInfoResponse>, (StatusCode, String)> {
    let app_id = params
        .app_id
        .ok_or((StatusCode::BAD_REQUEST, "Missing appId".into()))?;
    let device_id = params
        .device_id
        .ok_or((StatusCode::BAD_REQUEST, "Missing deviceId".into()))?;

    log::info!("[HTTP] getInfo: appId={}, deviceId={}", app_id, device_id);

    Ok(Json(GetInfoResponse {
        app_id,
        device_id,
        play: 0,
        purchase: 0,
        premium: false,
        trial: false,
        can_play: true,
    }))
}

async fn handle_icon(
    State(state): State<Arc<HttpServerState>>,
    Path(app_id): Path<String>,
) -> Response {
    let app_id = app_id.trim_end_matches(".png");
    match state.icon_cache.get_icon(app_id) {
        Some(data) => (
            StatusCode::OK,
            [
                ("content-type", "image/png"),
                ("cache-control", "public, max-age=86400"),
            ],
            data,
        )
            .into_response(),
        None => {
            log::info!("[HTTP] Icon not found: {}", app_id);
            (StatusCode::NOT_FOUND, format!("Icon not found: {}", app_id)).into_response()
        }
    }
}

fn metric_type_name(code: i64) -> &'static str {
    match code {
        1685287796 => "device_session_start", // 0x64737374 "dsst"
        1685284196 => "device_session_end",   // 0x64736564 "dsed"
        _ => "unknown",
    }
}

#[derive(Deserialize, Debug)]
#[allow(dead_code)]
struct MetricEvent {
    #[serde(rename = "type", default)]
    event_type: i64,
    #[serde(default)]
    time: i64,
    #[serde(rename = "appId", default)]
    app_id: String,
    #[serde(rename = "deviceId", default)]
    device_id: String,
    #[serde(default)]
    data: String,
}

#[derive(Deserialize)]
struct MetricsForm {
    action: Option<String>,
    events: Option<String>,
    token: Option<String>,
}

/*
Brass Monkey/Touchy doesn't have a clean way to signal the server what the controller has connected to, so we have to do this terribleness!

This exploits the metrics service inside the app to accurately track when a controller connects to a game while also handling the edge case where multiple game instances are possible.
It does this by first checking for the registry.relay deviceConnectRequested BMInvoke, then confirms it when the app sends dsst (device session start) to the HTTP server via metrics.

We have to check the BMInvoke (which also tells us the unique-per-instance deviceID in the BMPacket) because the metric only tells us the game's appID,
which would be the same for all duplicate game instances as it is hardcoded in every BM game.

The app only sends the metric if the connection was successful, thus we can be sure that it's accurate.
To be consistent, the Retouched controller apps use the same logic.
*/
async fn handle_metrics(
    State(state): State<Arc<HttpServerState>>,
    body: String,
) -> Json<serde_json::Value> {
    if let Ok(form) = serde_urlencoded::from_str::<MetricsForm>(&body) {
        let action = form.action.as_deref().unwrap_or("?");
        let token = form.token.as_deref().unwrap_or("?");

        if let Some(events_json) = &form.events {
            match serde_json::from_str::<Vec<MetricEvent>>(events_json) {
                Ok(events) => {
                    for evt in &events {
                        let name = metric_type_name(evt.event_type);
                        log::info!(
                            "[Metrics] {} | device={} app={} token={} time={}",
                            name,
                            evt.device_id,
                            evt.app_id,
                            token,
                            evt.time
                        );
                        if let Some(ref shared) = state.shared {
                            match evt.event_type {
                                1685287796 => {
                                    // dsst
                                    if !evt.device_id.is_empty() {
                                        let mut pending =
                                            shared.pending_connections.lock().unwrap();
                                        if let Some(game_device_id) = pending.remove(&evt.device_id)
                                        {
                                            log::info!(
                                                "[Metrics] Confirmed connection: controller {} -> game device {}",
                                                evt.device_id,
                                                game_device_id
                                            );
                                            shared
                                                .metrics_connections
                                                .lock()
                                                .unwrap()
                                                .insert(evt.device_id.clone(), game_device_id);
                                        } else {
                                            log::info!(
                                                "[Metrics] dsst for {} with no pending DeviceConnectRequested",
                                                evt.device_id
                                            );
                                        }
                                    }
                                }
                                1685284196 => {
                                    // dsed
                                    if !evt.device_id.is_empty() {
                                        shared
                                            .metrics_connections
                                            .lock()
                                            .unwrap()
                                            .remove(&evt.device_id);
                                        shared
                                            .pending_connections
                                            .lock()
                                            .unwrap()
                                            .remove(&evt.device_id);
                                    }
                                }
                                _ => {}
                            }
                        }
                    }
                }
                Err(e) => {
                    log::warn!(
                        "[Metrics] Failed to parse events JSON: {} (action={})",
                        e,
                        action
                    );
                }
            }
        } else {
            log::info!("[Metrics] action={} token={} (no events)", action, token);
        }
    } else {
        log::warn!("[Metrics] Unparseable body: {}", body);
    }

    Json(serde_json::json!({"status": "success"}))
}

async fn handle_ca_cert(
    State(state): State<Arc<HttpServerState>>,
    ConnectInfo(addr): ConnectInfo<std::net::SocketAddr>,
) -> Response {
    if !is_lan_ip(addr.ip()) {
        log::warn!("Rejected WAN request to /ca.crt from {}", addr.ip());
        return (
            StatusCode::FORBIDDEN,
            "Access denied. Must be on the same Wi-Fi network.",
        )
            .into_response();
    }

    let cert_path = state.data_dir.join("certs").join("rootCA.pem");
    match std::fs::read(&cert_path) {
        Ok(data) => {
            log::info!("[HTTP] Serving rootCA.pem to {}", addr.ip());
            (
                StatusCode::OK,
                [
                    ("content-type", "application/x-x509-ca-cert"),
                    (
                        "content-disposition",
                        "attachment; filename=\"RetouchedRootCA.pem\"",
                    ),
                    ("cache-control", "no-store"),
                ],
                data,
            )
                .into_response()
        }
        Err(e) => {
            log::error!("[HTTP] Failed to read rootCA.pem: {}", e);
            (
                StatusCode::NOT_FOUND,
                "CA Certificate not found. Ensure certificates are generated.",
            )
                .into_response()
        }
    }
}

async fn handle_onboard(ConnectInfo(addr): ConnectInfo<std::net::SocketAddr>) -> Response {
    if !is_lan_ip(addr.ip()) {
        log::warn!("Rejected WAN request to /onboard from {}", addr.ip());
        return (
            StatusCode::FORBIDDEN,
            "Access denied. Must be on the same Wi-Fi network.", // You shall not pass!
        )
            .into_response();
    }

    log::info!("[HTTP] Serving onboarding page to {}", addr.ip());
    let html = include_str!("../assets/onboard.html");
    axum::response::Html(html).into_response()
}

fn is_lan_ip(ip: std::net::IpAddr) -> bool {
    match ip {
        std::net::IpAddr::V4(v4) => v4.is_private() || v4.is_loopback() || v4.is_link_local(),
        std::net::IpAddr::V6(v6) => v6.is_loopback() || (v6.segments()[0] & 0xffc0) == 0xfe80,
    }
}

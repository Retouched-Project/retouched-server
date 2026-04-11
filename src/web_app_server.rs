// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 ddavef/KinteLiX retouched-server

use axum::{
    Router,
    extract::{Request, State},
    http::StatusCode,
    response::{IntoResponse, Response},
    routing::{any, post},
};
use tower_http::services::{ServeDir, ServeFile};

#[derive(Clone)]
struct ProxyState {
    webrtc_port: u16,
}

async fn handle_proxy_webrtc(State(state): State<ProxyState>, req: Request) -> Response {
    let path = req.uri().path().to_string();
    let query = req
        .uri()
        .query()
        .map(|q| format!("?{}", q))
        .unwrap_or_default();
    let target = format!("https://localhost:{}{}{}", state.webrtc_port, path, query);

    let method = req.method().clone();
    let content_type = req.headers().get("content-type").cloned();

    let body_bytes = match axum::body::to_bytes(req.into_body(), 10 * 1024 * 1024).await {
        Ok(b) => b,
        Err(e) => return (StatusCode::BAD_REQUEST, e.to_string()).into_response(),
    };

    let client = reqwest::Client::builder()
        .danger_accept_invalid_certs(true)
        .build()
        .unwrap();

    let mut proxy_req = client.request(method, &target);
    if let Some(ct) = content_type {
        proxy_req = proxy_req.header("content-type", ct);
    }
    proxy_req = proxy_req.body(body_bytes);

    match proxy_req.send().await {
        Ok(resp) => {
            let status =
                StatusCode::from_u16(resp.status().as_u16()).unwrap_or(StatusCode::BAD_GATEWAY);
            let mut builder = Response::builder().status(status);

            for (name, value) in resp.headers().iter() {
                builder = builder.header(name, value);
            }

            let body = match resp.bytes().await {
                Ok(b) => axum::body::Body::from(b),
                Err(_) => axum::body::Body::empty(),
            };

            builder.body(body).unwrap()
        }
        Err(e) => {
            log::error!("Proxy error to {}: {}", target, e);
            (StatusCode::BAD_GATEWAY, e.to_string()).into_response()
        }
    }
}

pub async fn run_web_app_server(
    web_dir: std::path::PathBuf,
    webrtc_port: u16,
    cert_dir: std::path::PathBuf,
    handle: axum_server::Handle,
) -> Result<(), String> {
    if !web_dir.join("index.html").exists() {
        let msg = format!(
            "Cannot serve web app: index.html not found in {}",
            web_dir.display()
        );
        log::error!("{}", msg);
        return Err(msg);
    }

    log::info!(
        "Starting Web App static server on https://0.0.0.0:8089 serving {}",
        web_dir.display()
    );

    let cert_key = cert_dir.join("key.pem");
    let cert_pem = cert_dir.join("server.pem");

    let certs =
        rustls_pemfile::certs(&mut &std::fs::read(&cert_pem).map_err(|e| e.to_string())?[..])
            .collect::<Result<Vec<_>, _>>()
            .map_err(|e| e.to_string())?;

    let key =
        rustls_pemfile::private_key(&mut &std::fs::read(&cert_key).map_err(|e| e.to_string())?[..])
            .map_err(|e| e.to_string())?
            .ok_or_else(|| "No private key found".to_string())?;

    let tls_config = rustls::ServerConfig::builder()
        .with_no_client_auth()
        .with_single_cert(certs, key)
        .unwrap();

    let rustls_config =
        axum_server::tls_rustls::RustlsConfig::from_config(std::sync::Arc::new(tls_config));

    let index_path = web_dir.join("index.html");

    let app = Router::new()
        .route("/offer", post(handle_proxy_webrtc))
        .route("/bmregistry/{*tail}", any(handle_proxy_webrtc))
        .route("/apps/icons/{*tail}", any(handle_proxy_webrtc))
        .fallback_service(ServeDir::new(&web_dir).fallback(ServeFile::new(index_path)))
        .with_state(ProxyState { webrtc_port });

    let addr = std::net::SocketAddr::from(([0, 0, 0, 0], 8089));

    if let Err(e) = axum_server::bind_rustls(addr, rustls_config)
        .handle(handle)
        .serve(app.into_make_service())
        .await
    {
        let msg = format!("Web app static serving error: {}", e);
        log::error!("{}", msg);
        return Err(msg);
    }

    Ok(())
}

pub fn spawn_web_app_server(
    web_dir: std::path::PathBuf,
    webrtc_port: u16,
    cert_dir: std::path::PathBuf,
) -> axum_server::Handle {
    let handle = axum_server::Handle::new();
    let h_clone = handle.clone();
    tokio::spawn(async move {
        let _ = run_web_app_server(web_dir, webrtc_port, cert_dir, h_clone).await;
    });
    handle
}

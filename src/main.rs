//! # Usage examples
//!
//! File in json_routes
//! ```
//! {"function_type": "normal_page",
//! "route": "/help",
//! "title": "Help Page",
//! "body": "help.html"
//! }
//! # assert_eq!(4, sum2(2, 2));
//! ```

#![allow(unused_imports)]

mod certs;
mod config;
mod htmlv;
mod my_api_config;
mod myapi;
mod procmon;

use axum::{
    BoxError, Router,
    body::Body,
    extract::ConnectInfo,
    handler::HandlerWithoutStateExt,
    http::{Request, StatusCode, Uri, uri::Authority},
    middleware::Next,
    response::{Html, IntoResponse, Redirect},
    routing::get,
};
use axum_extra::extract::Host;
use certs::load_tls_config;
use config::CertMode;
use config::{SystemConfig, load_or_create_config};
use htmlv::load_template_config;
use myapi::routes;
use std::net::{IpAddr, SocketAddr};
use tower_http::trace::TraceLayer;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

#[tokio::main]
async fn main() {
    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| format!("{}=debug", env!("CARGO_CRATE_NAME")).into()),
        )
        .with(tracing_subscriber::fmt::layer())
        .init();

    let config = load_or_create_config("config.yaml");

    load_template_config();
    match config.cert_mode {
        CertMode::SelfSigned | CertMode::Manual => {
            // Load TLS config for these modes
            let tls_config = load_tls_config(&config.cert_mode).await;

            // Spawn HTTP->HTTPS redirect server on HTTP port
            tokio::spawn(redirect_http_to_https(config.clone()));

            // Build router with middleware layers
            let app = routes()
                .layer(axum::middleware::from_fn(restrict_to_local_clients))
                .layer(TraceLayer::new_for_http());
            let addr = SocketAddr::from(([0, 0, 0, 0], config.https));
            tracing::info!("HTTPS server listening on {}", addr);

            axum_server::bind_rustls(addr, tls_config)
                .serve(app.into_make_service_with_connect_info::<SocketAddr>())
                .await
                .unwrap();
        }

        CertMode::None => {
            // No TLS: serve plain HTTP only, no redirect
            let addr = SocketAddr::from(([0, 0, 0, 0], config.http));
            tracing::info!("HTTP server listening on {}", addr);

            // Build router with middleware layers
            let app = routes()
                .layer(axum::middleware::from_fn(restrict_to_local_clients))
                .layer(TraceLayer::new_for_http());
            axum_server::bind(addr)
                .serve(app.into_make_service_with_connect_info::<SocketAddr>())
                .await
                .unwrap();
        }
    }
}

pub async fn restrict_to_local_clients(
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
    req: Request<Body>,
    next: Next,
) -> impl IntoResponse {
    if is_local_ip(addr.ip()) {
        next.run(req).await
    } else {
        (StatusCode::FORBIDDEN, "Access restricted to local network").into_response()
    }
}

fn is_local_ip(ip: IpAddr) -> bool {
    match ip {
        IpAddr::V4(ipv4) => {
            ipv4.octets()[0] == 10
                || (ipv4.octets()[0] == 192 && ipv4.octets()[1] == 168)
                || (ipv4.octets()[0] == 172 && (16..=31).contains(&ipv4.octets()[1]))
                || ipv4.is_loopback()
        }
        IpAddr::V6(ipv6) => ipv6.is_loopback(),
    }
}

// Redirect HTTP to HTTPS
async fn redirect_http_to_https(config: SystemConfig) {
    fn make_https(host: &str, uri: Uri, https_port: u16) -> Result<Uri, BoxError> {
        let mut parts = uri.into_parts();
        parts.scheme = Some(axum::http::uri::Scheme::HTTPS);

        if parts.path_and_query.is_none() {
            parts.path_and_query = Some("/".parse().unwrap());
        }

        let authority: Authority = host.parse()?;
        let bare_host = match authority.port() {
            Some(p) => authority
                .as_str()
                .strip_suffix(p.as_str())
                .unwrap()
                .strip_suffix(':')
                .unwrap(),
            None => authority.as_str(),
        };

        parts.authority = Some(format!("{bare_host}:{https_port}").parse()?);
        Ok(Uri::from_parts(parts)?)
    }

    let redirect = move |Host(host): Host, uri: Uri| async move {
        match make_https(&host, uri, config.https) {
            Ok(uri) => Ok(Redirect::permanent(&uri.to_string())),
            Err(error) => {
                tracing::warn!(%error, "failed to convert URI to HTTPS");
                Err(StatusCode::BAD_REQUEST)
            }
        }
    };

    let addr = SocketAddr::from(([0, 0, 0, 0], config.http));
    let listener = tokio::net::TcpListener::bind(addr).await.unwrap();
    tracing::debug!(
        "HTTP redirect listening on {}",
        listener.local_addr().unwrap()
    );
    axum::serve(listener, redirect.into_make_service())
        .await
        .unwrap();
}

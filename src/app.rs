use axum_login::{
    AuthManagerLayerBuilder, login_required,
    tower_sessions::{ExpiredDeletion, Expiry, SessionManagerLayer},
};
use axum_messages::MessagesManagerLayer;
use sqlx::SqlitePool;
use time::Duration;
use tokio::signal;
use tower_sessions::cookie::Key;
use tower_sessions_sqlx_store::SqliteStore;

use crate::{add_user::adduser_from_prompt, certs::load_tls_config};
use crate::config::CertMode;
use crate::config::{SystemConfig, load_or_create_config};
use crate::htmlv::load_template_config;
use crate::myapi::routes;
use axum::{
    BoxError, Router,
    body::Body,
    extract::ConnectInfo,
    handler::HandlerWithoutStateExt,
    http::{Request, StatusCode, Uri, uri::Authority},
    middleware::Next,
    response::{IntoResponse, Redirect},
};
use axum_extra::extract::Host;
use std::net::{IpAddr, SocketAddr};
use tower_http::trace::TraceLayer;

use crate::auth::{login, private, users::Backend};
use tracing;
pub struct RustyWebApp {
    db: SqlitePool,
    config: SystemConfig,
}
///Ensure there is a user already.
async fn first_time_setup(backend: &Backend)-> Result<(), Box<dyn std::error::Error>> {

    if backend.check_for_users().await? {
        println!("Users already exist.");
    } else {
        println!("No users found. You should create a default user!");
        adduser_from_prompt().await?;
    }

    Ok(())
}

impl RustyWebApp {
    pub async fn new() -> Result<Self, Box<dyn std::error::Error>> {
        let config = load_or_create_config("config.yaml");

        // Ensure the database file exists before proceeding.
        if !std::path::Path::new("thisbackend.db").exists() {
            std::fs::File::create("thisbackend.db")?;
        }

        let db = SqlitePool::connect("thisbackend.db").await?;
        sqlx::migrate!().run(&db).await?;

        Ok(Self { db, config })
    }
    pub async fn run(self) -> Result<(), Box<dyn std::error::Error>> {
        // Session layer.
        //
        // This uses `tower-sessions` to establish a layer that will provide the session
        // as a request extension.
        load_template_config();
        let session_store = SqliteStore::new(self.db.clone());
        session_store.migrate().await?;

        let shutdown_handle = axum_server::Handle::new();
        let shutdown_handle_clone = shutdown_handle.clone();

        let deletion_task = tokio::task::spawn(
            session_store
                .clone()
                .continuously_delete_expired(tokio::time::Duration::from_secs(60)),
        );

        tokio::spawn(async move {
            shutdown_signal().await;
            tracing::warn!("Shutting down.");
            shutdown_handle_clone.shutdown();
        });
        let key = Key::generate();

        let session_layer = SessionManagerLayer::new(session_store)
            .with_secure(false)
            .with_expiry(Expiry::OnInactivity(Duration::days(1)))
            .with_signed(key);

        // Auth service.
        //
        // This combines the session layer with our backend to establish the auth
        // service which will provide the auth session as a request extension.
        let backend = Backend::new(self.db);
        
        first_time_setup(&backend).await?;
        let auth_layer = AuthManagerLayerBuilder::new(backend, session_layer).build();

        match self.config.cert_mode {
            CertMode::SelfSigned | CertMode::Manual => {
                
                let tls_config = load_tls_config(&self.config.cert_mode).await;
                tokio::spawn(redirect_http_to_https(self.config.clone()));

                let app = Router::new()
                    // Public (login-free) routes
                    .merge(routes())
                    // Protected (login-required) routes
                    .nest(
                        "/protected",
                        private::router()
                            .route_layer(login_required!(Backend, login_url = "/login")),
                    )
                    // Auth routes (e.g., login, logout)
                    .merge(login::router())
                    // Global middleware (auth manager, session layer, logging)
                    .layer(MessagesManagerLayer)
                    .layer(auth_layer)
                    .layer(axum::middleware::from_fn(restrict_to_local_clients))
                    .layer(TraceLayer::new_for_http());

                let addr = SocketAddr::from(([0, 0, 0, 0], self.config.https));
                tracing::info!("HTTPS server listening on {}", addr);

                axum_server::bind_rustls(addr, tls_config)
                    .handle(shutdown_handle)
                    .serve(app.into_make_service_with_connect_info::<SocketAddr>())
                    .await?;
                deletion_task.await??;

                Ok(())
            }

            CertMode::None => {
                // No TLS: serve plain HTTP only, no redirect
                let addr = SocketAddr::from(([0, 0, 0, 0], self.config.http));
                tracing::info!("HTTP server listening on {}", addr);

                // Build router with middleware layers
                let app = routes()
                    .layer(axum::middleware::from_fn(restrict_to_local_clients))
                    .layer(TraceLayer::new_for_http());
                axum_server::bind(addr)
                    .handle(shutdown_handle)
                    .serve(app.into_make_service_with_connect_info::<SocketAddr>())
                    .await?;
                deletion_task.await??;

                Ok(())
            }
        }
    }
}

/// Shutdown handler.
async fn shutdown_signal() {
    tracing::warn!("Checking for shutdown signal...");
    let ctrl_c = async {
        signal::ctrl_c().await.expect("failed to listen for ctrl_c");
    };

    tracing::warn!("Shutting down!");
    #[cfg(unix)]
    let terminate = async {
        use tokio::signal::unix::{SignalKind, signal};
        let mut term_signal =
            signal(SignalKind::terminate()).expect("failed to listen for SIGTERM");
        term_signal.recv().await;
    };

    #[cfg(not(unix))]
    let terminate = std::future::pending::<()>(); // no-op

    tokio::select! {
        _ = ctrl_c => {
            tracing::warn!("Received Ctrl+C");
        }
        _ = terminate => {
            tracing::warn!("Received SIGTERM");
        }
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

///Restrict to clients on the local network.
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

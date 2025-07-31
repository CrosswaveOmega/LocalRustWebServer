/*
self signed certifications here, incase https on a local network is 
wanted.
*/
use axum_server::tls_rustls::RustlsConfig;
use std::path::PathBuf;
use crate::config::CertMode;

// Load TLS config based on certificate mode
// This is a work in progress.
pub async fn load_tls_config(mode: &CertMode) -> RustlsConfig {
    match mode {
        CertMode::SelfSigned => RustlsConfig::from_pem_file(
            PathBuf::from("self_signed_certs/cert.pem"),
            PathBuf::from("self_signed_certs/key.pem"),
        )
        .await
        .expect("Failed to load self-signed certificate"),

        CertMode::Manual => RustlsConfig::from_pem_file(
            PathBuf::from("manual_certs/cert.pem"),
            PathBuf::from("manual_certs/key.pem"),
        )
        .await
        .expect("Failed to load manual certificate"),

        CertMode::None => panic!("No TLS config, fallback to HTTP"), // No TLS config, fallback to HTTP

    }
}

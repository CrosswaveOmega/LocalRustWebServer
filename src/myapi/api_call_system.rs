/// A generic external API handler.
/// Currently only usable for GET requests.
use axum::{
    Router, extract::Query, response::Html, response::IntoResponse, response::Json, routing::get,
};
use reqwest::{Client, header};

use axum::http::StatusCode;
use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use serde_urlencoded;
use std::collections::HashMap;
use std::env;
use std::fmt::Debug;
use std::path::Path;
use std::time::{SystemTime, UNIX_EPOCH};
use tracing;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

use crate::my_api_config::ApiEndpointConfig;

//
// --- Constant Base URL ---

/// Fetches and returns JSON as a raw `serde_json::Value`.
async fn fetch_raw_json(client: &Client, url: &str) -> Option<Value> {
    match client
        .get(url)
        .header(header::CONTENT_TYPE, "application/json")
        .header(header::ACCEPT, "application/json")
        .header("Accept-Language", "en-US")
        .send()
        .await
    {
        Ok(resp) => {
            let body = resp.text().await.ok()?;
            tracing::debug!("Body content is {}", body);
            match serde_json::from_str::<Value>(&body) {
                Ok(data) => Some(data),
                Err(e) => {
                    tracing::error!("Error parsing raw JSON from {}: {}", url, e);
                    None
                }
            }
        }
        Err(e) => {
            tracing::error!("Error fetching {}: {}", url, e);
            None
        }
    }
}

/// Handler for calling an external API with dynamic endpoint and base URL.
/// Base_URL and endpoints are loaded within the relevant json_routes
pub async fn api_caller(
    Query(params): Query<HashMap<String, String>>,
    base_url: String,
    endpoints: HashMap<String, ApiEndpointConfig>,
) -> impl IntoResponse {
    let client = Client::new();

    // match the endpoint to the path parameter
    let endpoint_key = match params.get("endpoint") {
        Some(e) => e.to_lowercase(),
        None => return (StatusCode::BAD_REQUEST, "Missing 'endpoint' parameter").into_response(),
    };

    //get configuration for this endpoint
    let endpoint_cfg = match endpoints.get(&endpoint_key) {
        Some(cfg) => cfg,
        None => return (StatusCode::BAD_REQUEST, "Invalid endpoint").into_response(),
    };

    let mut url = format!("{}{}", base_url, endpoint_cfg.path);

    // merge default params and user-supplied ones
    let mut merged_params = endpoint_cfg.default_params.clone();

    for field in &["path", "default_params"] {
        tracing::debug!("Calling path {}: {:?}", field, endpoint_cfg.path);
    }

    for (k, v) in &params {
        if k != "endpoint" {
            merged_params.insert(k.clone(), v.clone());
            tracing::debug!("added key: {} with value: {}", k, v);
        }
    }
    tracing::info!("Calling url {}", url);
    if !merged_params.is_empty() {
        let query_str =
            serde_urlencoded::to_string(&merged_params).unwrap_or_else(|_| "".to_string());
        tracing::info!("Encoded query string: {}", query_str);
        url.push('?');
        url.push_str(&query_str);
    }

    match fetch_raw_json(&client, &url).await {
        Some(data) => Json(data).into_response(),
        None => (
            StatusCode::INTERNAL_SERVER_ERROR,
            "Failed to fetch data from remote API",
        )
            .into_response(),
    }
}

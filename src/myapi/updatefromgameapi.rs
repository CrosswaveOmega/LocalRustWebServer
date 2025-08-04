// an extension for Helldiver 2, a game I play.
use axum::{
    Router, extract::Query, response::Html, response::IntoResponse, response::Json, routing::get,
};
use reqwest::{Client, header};

use axum::http::StatusCode;
use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;
use std::env;
use std::fmt::Debug;
use std::path::Path;
use std::time::{SystemTime, UNIX_EPOCH};
use tracing;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

//
// --- Query Parameters ---
//
#[derive(Deserialize)]
pub struct ApiProxyParams {
    /// The endpoint to call
    pub endpoint: String,
    /// Optional parameter for NewsFeed: maximum number of entries
    pub maxEntries: Option<i32>,
    /// Optional parameter for NewsFeed: only entries after this timestamp
    pub fromTimestamp: Option<i64>,
}

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
            tracing::info!("{}", body);
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

/// Call the API with dynamic endpoint and base URL.
pub async fn api_proxy(
    Query(params): Query<HashMap<String, String>>,
    base_url: String,
) -> impl IntoResponse {
    let client = Client::new();
    let endpoint = match params.get("endpoint") {
        Some(e) => e.to_lowercase(),
        None => return (StatusCode::BAD_REQUEST, "Missing 'endpoint' parameter").into_response(),
    };

    let result = match endpoint.as_str() {
        "warstatus" => {
            let url = format!("{}/WarSeason/801/Status", base_url);
            fetch_raw_json(&client, &url).await
        }
        "warinfo" => {
            let url = format!("{}/WarSeason/801/WarInfo", base_url);
            fetch_raw_json(&client, &url).await
        }
        "newsfeed" => {
            let mut url = format!("{}/NewsFeed/801", base_url);
            let mut query_parts = Vec::new();

            if let Some(max_entries) = params.get("maxEntries") {
                query_parts.push(format!("maxEntries={}", max_entries));
            } else {
                query_parts.push("maxEntries=1024".to_string());
            }

            if let Some(from_ts) = params.get("fromTimestamp") {
                query_parts.push(format!("fromTimestamp={}", from_ts));
            }

            if !query_parts.is_empty() {
                url.push('?');
                url.push_str(&query_parts.join("&"));
            }

            fetch_raw_json(&client, &url).await
        }
        "majororders" => {
            let url = format!("{}/v2/Assignment/War/801", base_url);
            fetch_raw_json(&client, &url).await
        }
        "globalstats" => {
            let url = format!("{}/Stats/War/801/Summary", base_url);
            fetch_raw_json(&client, &url).await
        }
        "missionrewards" => {
            let url = format!("{}/Mission/RewardEntries", base_url);
            fetch_raw_json(&client, &url).await
        }
        "newsticker" => {
            let url = format!("{}/WarSeason/NewsTicker", base_url);
            fetch_raw_json(&client, &url).await
        }
        "gweffects" => {
            let url = format!("{}/WarSeason/GalacticWarEffects", base_url);
            fetch_raw_json(&client, &url).await
        }
        _ => return (StatusCode::BAD_REQUEST, "Invalid endpoint").into_response(),
    };

    match result {
        Some(data) => Json(data).into_response(),
        None => (
            StatusCode::INTERNAL_SERVER_ERROR,
            "Failed to fetch data from remote API",
        )
            .into_response(),
    }
}

// an extension for Helldiver 2, a game I play.
use crate::htmlv::{RenderHtml, HtmlV};
use axum::{routing::get, Router, response::Html,response::Json,
    extract::Query, response::IntoResponse};
use reqwest::{Client,header};


use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};
use serde::{Serialize,Deserialize};
use serde::de::DeserializeOwned;
use serde_json::Value;
use std::fmt::Debug;  
use axum::http::StatusCode;
use std::time::{SystemTime, UNIX_EPOCH};
use std::env;
use std::path::Path;

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
//
const API_BASE_URL: &str = "https://api.live.prod.thehelldiversgame.com/api";

/// Fetches and returns JSON as raw `serde_json::Value`.
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
            println!("{}",body);
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
pub async fn api_proxy(Query(params): Query<ApiProxyParams>) -> impl IntoResponse {
    let client = Client::new();
    let endpoint = params.endpoint.to_lowercase();

    match endpoint.as_str() {
        "warstatus" => {
            let url = format!("{}/WarSeason/801/Status", API_BASE_URL);
            match fetch_raw_json(&client, &url).await {
                Some(data) => Json(data).into_response(),
                None => (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    "Failed to fetch RewardEntries",
                )
                    .into_response(),
            }
        }
        "warinfo" => {
            let url = format!("{}/WarSeason/801/WarInfo", API_BASE_URL);
            match fetch_raw_json(&client, &url).await {
                Some(data) => Json(data).into_response(),
                None => (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    "Failed to fetch RewardEntries",
                )
                    .into_response(),
            }
        }
        "newsfeed" => {
            let mut url = format!("{}/NewsFeed/801", API_BASE_URL);
            // Append optional query parameters if provided
            let mut query_params = Vec::new();
            if let Some(max) = params.maxEntries {
                query_params.push(format!("maxEntries={}", max));
            }else{
                query_params.push(format!("maxEntries={}", 1024));
            }
            if let Some(ts) = params.fromTimestamp {
                query_params.push(format!("fromTimestamp={}", ts));
            }
            if !query_params.is_empty() {
                url.push('?');
                url.push_str(&query_params.join("&"));
            }
            match fetch_raw_json(&client, &url).await {
                Some(data) => Json(data).into_response(),
                None => (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    "Failed to fetch RewardEntries",
                )
                    .into_response(),
            }
        }
        "majororders" => {
            let url = format!("{}/v2/Assignment/War/801", API_BASE_URL);
            match fetch_raw_json(&client, &url).await {
                Some(data) => Json(data).into_response(),
                None => (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    "Failed to fetch RewardEntries",
                )
                    .into_response(),
            }
        }
        "globalstats" => {
            let url = format!("{}/Stats/War/801/Summary", API_BASE_URL);
            match fetch_raw_json(&client, &url).await {
                Some(data) => Json(data).into_response(),
                None => (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    "Failed to fetch RewardEntries",
                )
                    .into_response(),
            }
        }
        "missionrewards" => {
            let url = format!("{}/Mission/RewardEntries", API_BASE_URL);
            match fetch_raw_json(&client, &url).await {
                Some(data) => Json(data).into_response(),
                None => (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    "Failed to fetch RewardEntries",
                )
                    .into_response(),
            }
        }
        "newsticker" => {
            let url = format!("{}/WarSeason/NewsTicker", API_BASE_URL);
            match fetch_raw_json(&client, &url).await {
                Some(data) => Json(data).into_response(),
                None => (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    "Failed to fetch RewardEntries",
                )
                    .into_response(),
            }
        }
        "gweffects" => {
            let url = format!("{}/WarSeason/GalacticWarEffects", API_BASE_URL);
            match fetch_raw_json(&client, &url).await {
                Some(data) => Json(data).into_response(),
                None => (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    "Failed to fetch RewardEntries",
                )
                    .into_response(),
            }
        }
        

        _ => (StatusCode::BAD_REQUEST, "Invalid endpoint").into_response(),
    }
}


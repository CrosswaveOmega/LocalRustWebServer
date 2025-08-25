use crate::auth::users::AuthSession;
use crate::htmlv::{HtmlV, RenderHtml};
use crate::my_api_config::ApiEndpointConfig;
use axum::{extract::Query, response::IntoResponse};
use std::collections::HashMap;
use std::process::Command;

// for NormalPageTemplate:
pub async fn normal_page_template_handler(
    title: String,
    body: String,
    template: i32,
) -> HtmlV<String> {
    HtmlV((title, body).render_html_from_int(template))
}

/// for NormalPageTemplate,  secure variant.
pub async fn normal_page_template_handler_secure(
    auth_session: AuthSession,
    title: String,
    body: String,
    template: i32,
) -> HtmlV<String> {
    match auth_session.user {
        Some(user) => HtmlV((title, body, user.username).render_html_from_int(template)),
        None => {
            let error_message = "Internal Server Error";
            HtmlV((title, format!("{} - {}", error_message, body)).render_html_from_int(-1))
        }
    }
}
/// for the Log Get Handler:
pub async fn get_logs_handler(
    Query(params): Query<HashMap<String, String>>,
    log_file_types: Option<Vec<String>>,
    title: String,
) -> HtmlV<String> {
    // ensure there are provided log files.  if not, raise an exception
    let log_paths = log_file_types.unwrap_or_else(|| panic!("Log not found"));

    // parse selected log index from query
    let selected_index = params.get("log").and_then(|v| v.parse::<usize>().ok());

    // determine selected log path (if valid index), or default to first
    let log_file_path = selected_index
        .and_then(|i| log_paths.get(i))
        .unwrap_or(&log_paths[0]);

    // run tail command
    let output = Command::new("tail")
        .arg("-n")
        .arg("50")
        .arg(log_file_path)
        .output();

    match output {
        Ok(output) if output.status.success() => {
            let logs = String::from_utf8_lossy(&output.stdout).to_string();
            let log_lines = logs
                .lines()
                .map(|line| format!("<li>{}</li>", line))
                .collect::<Vec<_>>()
                .join("\n");

            HtmlV((title, format!("<ul>{}</ul>", log_lines)).render_html_from_int(-1))
        }
        Ok(output) => {
            let error_message = String::from_utf8_lossy(&output.stderr).to_string();
            HtmlV((title, format!("Error: {}", error_message)).render_html_from_int(-1))
        }
        Err(e) => {
            HtmlV((title, format!("<pre>Failed to run tail: {}</pre>", e)).render_html_from_int(-1))
        }
    }
}

pub async fn get_logs_handler_wrapped(
    query: Query<HashMap<String, String>>,
    log_file_types: Option<Vec<String>>,
    title: String,
) -> impl IntoResponse {
    get_logs_handler(query, log_file_types, title).await
}

pub async fn api_caller_wrapped(
    query: Query<HashMap<String, String>>,
    base_url: String,
    endpoints: HashMap<String, ApiEndpointConfig>,
) -> impl IntoResponse {
    crate::myapi::api_call_system::api_caller(query, base_url, endpoints).await
}

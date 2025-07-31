use crate::htmlv::{RenderHtml, HtmlV};
use crate::my_api_config::{RouteFunction};
use axum::{extract::Query, routing::get, 
    routing::{post},Router,response::{IntoResponse}};
use serde::Deserialize;
use std::process::{Stdio,Command};
use std::time::{SystemTime, UNIX_EPOCH};
use std::collections::HashMap;
use tokio::process::Command as TokioCommand;
use tower_http::services::ServeDir;

use shellexpand;
use std::thread;
use std::fs::OpenOptions;
use std::io::Write;
use std::fs;
use std::path::{Path, PathBuf};
use serde_json::Value;


/// Returns Ok(true) if lock acquired, Ok(false) if lock held by another process.
pub async fn try_acquire_lock(lock_path: &str) -> Result<bool, std::io::Error> {
    let lock_status = Command::new("flock")
        .arg("-n")
        .arg(lock_path)
        .arg("echo")
        .arg("Locked")
        .output();

    match lock_status {
        Ok(output) => Ok(output.status.success()),
        Err(e) => Err(e),
    }
}


/// Spawn a shell script in the background and log its output.
/// Returns a message about whether the script started successfully.
pub async fn spawn_script_in_background(script_path: &str, log_path: &str) -> Result<String, std::io::Error> {
    tracing::debug!("Running {} in the background", script_path);

    let mut command = TokioCommand::new("sh");
    command
        .arg("-c")
        .arg(format!(
            "{} > {} 2>&1",
            script_path,
            log_path
        ))
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());

    match command.spawn() {
        Ok(mut child) => {
            if let Ok(Some(_)) = child.try_wait() {
                tracing::debug!("Script {} has finished running.", script_path);
                Ok(format!("Script {} has finished running.", script_path))
            } else {
                tracing::debug!("Script {} is running in the background.", script_path);
                Ok(format!("Script {} is running.", script_path))
            }
        }
        Err(e) => {
            tracing::debug!("Failed to start script {}: {}", script_path, e);
            Err(e)
        }
    }
}



/// For the RunCommand RouteFunction
pub async fn run_command_handler(lock:String,log:String,script:String,title:String) -> HtmlV<String> {
    let lock_file_path = shellexpand::tilde(&lock).to_string();
    let log_file_path = shellexpand::tilde(&log).to_string();
    let script_file_path = shellexpand::tilde(&script).to_string();

    match try_acquire_lock(&lock_file_path).await {
        Ok(true) => {
            let handle = tokio::spawn(async move {
                match spawn_script_in_background(&script_file_path, &log_file_path).await {
                    Ok(msg) => msg,
                    Err(e) => format!("Failed to run script: {}", e),
                }
            });

            let mywork = match handle.await {
                Ok(result) => result,
                Err(e) => format!("Failed to execute async task: {}", e),
            };

            HtmlV((title, format!("<p>{}</p>", mywork)).render_html_from_int(1))
        }
        Ok(false) => {
            HtmlV((title, "<p>This script is already running...</p>".to_string()).render_html_from_int(1))
        }
        Err(e) => {
            HtmlV((title, format!("<p>Error acquiring lock: {}</p>", e)).render_html_from_int(1))
        }
    }
}
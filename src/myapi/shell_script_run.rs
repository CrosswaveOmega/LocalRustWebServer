use crate::auth::users::{AuthSession, User};
use crate::htmlv::{HtmlV, RenderHtml};
use crate::state;
use axum::Json;
use axum::response::IntoResponse;
use serde_json::Value as JsonValue;

use shellexpand;
use state::AppSingleton;
use std::process::{Command, Stdio};
use std::{collections::HashMap, sync::Arc};
use tokio::process::Command as TokioCommand;
use tokio::sync::oneshot;

use tokio::sync::Mutex;

use once_cell::sync::Lazy;

use process_wrap::tokio::CommandWrap;

#[cfg(unix)]
use process_wrap::tokio::ProcessGroup;

#[cfg(windows)]
use process_wrap::tokio::JobObject;

pub static SCRIPT_KILL_SENDERS: Lazy<Mutex<HashMap<String, oneshot::Sender<()>>>> =
    Lazy::new(|| Mutex::new(HashMap::new()));

/// Returns Ok(true) if lock acquired, Ok(false) if lock held by another process.
pub async fn try_acquire_lock(lock_path: &str) -> Result<bool, std::io::Error> {
    if lock_path.is_empty() {
        return Ok(true);
    }

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

/// Call this to stop the script at SCRIPTPATH
pub async fn stop_script(script_path: &str) -> Result<(), std::io::Error> {
    let sender_opt = SCRIPT_KILL_SENDERS.lock().await.remove(script_path);
    if let Some(kill_tx) = sender_opt {
        match kill_tx.send(()) {
            Ok(_) => {
                println!("Kill signal sent for {}", script_path);
                Ok(())
            }
            Err(_) => Err(std::io::Error::new(
                std::io::ErrorKind::Other,
                "Failed to send kill signal",
            )),
        }
    } else {
        Err(std::io::Error::new(
            std::io::ErrorKind::NotFound,
            "No running script found for this path",
        ))
    }
}

/// Static handler that is called when a script finishes
fn script_finished_handler(script_path: &str, status: std::process::ExitStatus) {
    let status_str = if let Some(code) = status.code() {
        format!("exit code {}", code)
    } else {
        "terminated by signal".to_string()
    };
    tracing::info!("Script {} finished with status: {}", script_path, status);

    let app = AppSingleton::instance();

    app.insert_status(script_path, &status_str);
}

/// Spawn a shell script in the background and log its output.
/// Returns a message about whether the script started successfully.
pub async fn spawn_script_in_background(
    script_path: &str,
    log_path: &str,
) -> Result<String, std::io::Error> {
    let app = AppSingleton::instance();
    tracing::debug!("Running {} in the background", script_path);

    // CommandWrap has better process grouping.
    let mut cmdwrap = CommandWrap::with_new("sh", |cmd: &mut TokioCommand| {
        cmd.arg("-c")
            .arg(format!("{} > {} 2>&1", script_path, log_path))
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());
    });

    // Configuration for proper cross-platform cleanup
    #[cfg(unix)]
    cmdwrap.wrap(ProcessGroup::leader()); // Unix: new process group
    #[cfg(windows)]
    cmdwrap.wrap(JobObject); // Windows: Job object kill-on-drop

    match cmdwrap.spawn() {
        Ok(mut child) => {
            println!("Child ID: {:?}", child.id());
            app.insert_status(script_path, "running");
            let (kill_tx, mut kill_rx) = oneshot::channel::<()>();
            SCRIPT_KILL_SENDERS
                .lock()
                .await
                .insert(script_path.to_string(), kill_tx);

            let script_path_clone = script_path.to_string();

            tokio::spawn(async move {
                tokio::select! {
                    status = child.wait() => {
                        match status {
                            Ok(status) => {
                                // Call a static handler function here
                                script_finished_handler(&script_path_clone, status);
                                SCRIPT_KILL_SENDERS.lock().await.remove(&script_path_clone);
                            }
                            Err(e) => {
                                tracing::error!("Failed to wait for script {}: {}", script_path_clone, e);
                                SCRIPT_KILL_SENDERS.lock().await.remove(&script_path_clone);
                            }
                        }
                    }
                    _ = &mut kill_rx => {


                        println!("GOTTEN KILL REQUEST.");
                        if let Err(e) = Pin::from(child.kill()).await {
                            println!("Failed to kill script {}: {}", script_path_clone, e);

                            tracing::error!("Failed to kill script {}: {}", script_path_clone, e);
                        }

                        println!("Kill sent...");
                        let _ = child.wait().await;

                        println!("Script {} was killed.", script_path_clone);
                        tracing::debug!("Script {} was killed.", script_path_clone);

                        app.insert_status(&script_path_clone, "Killed...");
                        SCRIPT_KILL_SENDERS.lock().await.remove(&script_path_clone);
                    }
                }
            });

            tracing::debug!("Script {} is running in the background.", script_path);
            Ok(format!("Script {} is running.", script_path))
        }

        Err(e) => {
            tracing::debug!("Failed to start script {}: {}", script_path, e);
            Err(e)
        }
    }
}

/// For the RunCommand RouteFunction
pub async fn run_command_handler(
    lock: String,
    log: String,
    script: String,
    title: String,
    template: i32,
) -> HtmlV<String> {
    let lock_file_path = shellexpand::tilde(&lock).to_string();
    let log_file_path = shellexpand::tilde(&log).to_string();
    let script_file_path = shellexpand::tilde(&script).to_string();
    tracing::info!(
        "calling {},{},{}",
        lock_file_path,
        log_file_path,
        script_file_path
    );
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

            HtmlV((title, format!("<p>{}</p>", mywork)).render_html_from_int(template))
        }
        Ok(false) => HtmlV(
            (
                title,
                "<p>This script is already running...</p>".to_string(),
            )
                .render_html_from_int(template),
        ),
        Err(e) => HtmlV(
            (title, format!("<p>Error acquiring lock: {}</p>", e)).render_html_from_int(template),
        ),
    }
}

/// Secure variant of run command.
pub async fn run_command_handler_secure_wrap(
    user: User,
    lock: String,
    log: String,
    script: String,
    title: String,
    template: i32,
) -> HtmlV<String> {
    let lock_file_path = shellexpand::tilde(&lock).to_string();
    let log_file_path = shellexpand::tilde(&log).to_string();
    let script_file_path = shellexpand::tilde(&script).to_string();
    tracing::info!(
        "calling {},{},{}",
        lock_file_path,
        log_file_path,
        script_file_path
    );
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

            HtmlV(
                (title, format!("<p>{}</p>", mywork), user.username).render_html_from_int(template),
            )
        }
        Ok(false) => HtmlV(
            (
                title,
                "<p>This script is already running...</p>".to_string(),
                user.username,
            )
                .render_html_from_int(template),
        ),
        Err(e) => HtmlV(
            (
                title,
                format!("<p>Error acquiring lock: {}</p>", e),
                user.username,
            )
                .render_html_from_int(template),
        ),
    }
}

/// Secure variant of run command.
pub async fn run_command_handler_secure(
    auth_session: AuthSession,
    lock: String,
    log: String,
    script: String,
    title: String,
    template: i32,
) -> HtmlV<String> {
    match auth_session.user {
        Some(user) => {
            run_command_handler_secure_wrap(
                user,
                lock.clone(),
                log.clone(),
                script.clone(),
                title.clone(),
                template,
            )
            .await
        }
        None => {
            let error_message = "Internal Server Error-insufficient perms";
            HtmlV((title, format!("{}", error_message)).render_html_from_int(-1))
        }
    }
}

/// Secure variant of run command.
pub async fn get_command_statuses(title: String, template: i32) -> HtmlV<String> {
    let app = AppSingleton::instance();
    let hashjson = app.hashstatus_to_json();
    HtmlV((title, format!("<p>{}</p>", hashjson)).render_html_from_int(template))
}
/// Secure variant of run command.
pub async fn get_command_statuses_secure(auth_session: AuthSession) -> impl IntoResponse {
    match auth_session.user {
        Some(_user) => {
            let app = AppSingleton::instance();
            let hashjson_str = app.hashstatus_to_json(); // returns String
            let hashjson: JsonValue =
                serde_json::from_str(&hashjson_str).unwrap_or_else(|_| serde_json::json!({}));
            Json(hashjson)
        }
        None => {
            let error_json = serde_json::json!({
                "status": "error",
                "message": "Insufficient permission"
            });
            Json(error_json)
        }
    }
}

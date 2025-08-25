use crate::auth::users::{AuthSession, User};
use crate::htmlv::{HtmlV, RenderHtml};
use crate::state;
use state::AppSingleton;
use std::process::{Command, Stdio};
use tokio::process::Command as TokioCommand;

use shellexpand;

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

    // Additional actions can go here
}
/// Spawn a shell script in the background and log its output.
/// Returns a message about whether the script started successfully.
pub async fn spawn_script_in_background(
    script_path: &str,
    log_path: &str,
) -> Result<String, std::io::Error> {
    let app = AppSingleton::instance();
    tracing::debug!("Running {} in the background", script_path);

    let mut command = TokioCommand::new("sh");
    command
        .arg("-c")
        .arg(format!("{} > {} 2>&1", script_path, log_path))
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());

    match command.spawn() {
        Ok(mut child) => {
            app.insert_status(script_path, "running");
            let script_path_clone = script_path.to_string();
            tokio::spawn(async move {
                match child.wait().await {
                    Ok(status) => {
                        // Call a static handler function here
                        script_finished_handler(&script_path_clone, status);
                    }
                    Err(e) => {
                        tracing::error!("Failed to wait for script {}: {}", script_path_clone, e);
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
pub async fn get_command_statuses_secure(
    auth_session: AuthSession,
    title: String,
    template: i32,
) -> HtmlV<String> {
    let app = AppSingleton::instance();
    let hashjson = app.hashstatus_to_json();
    HtmlV((title, format!("<p>{}</p>", hashjson)).render_html_from_int(template))
}

use std::sync::Arc;

use crate::{
    htmlv::get_tera,
    my_api_config::RouteFunction,
    myapi::{
        add_route_to_router, build_help_page_html, load_routes_from_dir,
        shell_script_run::{get_command_statuses_secure, stop_script},
    },
};

use crate::auth::users::AuthSession;
use axum::{
    Json, Router,
    http::StatusCode,
    response::{Html, IntoResponse},
    routing::{get, post},
};
use axum_messages::{Message, Messages};
use serde::Deserialize;
use serde_json::json;
use tera::Context;

#[derive(Deserialize)]
pub struct StopRequest {
    script: String,
}

fn render_protected_template(
    messages: Vec<Message>,
    username: &str,
) -> Result<String, tera::Error> {
    let tera = get_tera();
    let mut context = Context::new();

    let messages_as_strs: Vec<String> = messages.into_iter().map(|m| m.message).collect();
    context.insert("messages", &messages_as_strs);
    context.insert("username", &username);
    context.insert("title", "Protected");

    context.insert("body", "Something...");
    tera.render("private.html", &context)
}

#[derive(Clone)]
pub struct AppState {
    pub shutdown: Arc<axum_server::Handle>,
}

/// Build the SECURE routes.
pub fn router(handle: Arc<axum_server::Handle>) -> Router<()> {
    let route_functions = load_routes_from_dir("./json_routes");
    build_secure_router_from_route_functions(route_functions, handle)
}

pub async fn stop_command_handler(
    auth_session: AuthSession,
    Json(payload): Json<StopRequest>,
) -> impl IntoResponse {
    let script = payload.script;
    println!("!Checking for kill {}", script);

    match auth_session.user {
        Some(user) => match stop_script(&script).await {
            Ok(msg) => {
                tracing::info!("User {} stopped script {}", user.username, script);
                (StatusCode::OK, msg).into_response()
            }
            Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response(),
        },
        None => StatusCode::UNAUTHORIZED.into_response(),
    }
}

pub fn make_reload_route(handle: Arc<axum_server::Handle>) -> axum::routing::MethodRouter {
    let reload_route = {
        let shutdown_handle = handle.clone();
        post(move |auth_session: AuthSession| {
            let shutdown_handle = shutdown_handle.clone();

            async move {
                match auth_session.user {
                    Some(user) => {
                        // Spawn shutdown in the background
                        tokio::spawn(async move {
                            tokio::time::sleep(std::time::Duration::from_millis(10)).await;
                            println!("Shutting down server (requested by {})...", user.username);
                            shutdown_handle.shutdown();
                        });

                        // Respond immediately
                        Json(json!({
                            "status": "ok",
                            "message": "Server is reloading..."
                        }))
                        .into_response()
                    }
                    None => StatusCode::UNAUTHORIZED.into_response(),
                }
            }
        })
    };
    reload_route
}

pub fn build_secure_router_from_route_functions(
    route_functions: Vec<RouteFunction>,
    handle: Arc<axum_server::Handle>,
) -> Router<()> {
    let mut router = Router::new()
        .route("/", get(self::get::protected))
        .route("/reload", make_reload_route(handle))
        .route("/command_status", get(get_command_statuses_secure))
        .route("/kill_script", post(stop_command_handler));
    let help_text = build_help_page_html(route_functions.clone());

    for route_func in route_functions {
        let meta = match &route_func {
            RouteFunction::NormalPage { meta, .. }
            | RouteFunction::HelpPage { meta, .. }
            | RouteFunction::CommandStatus { meta, .. }
            | RouteFunction::RunCommand { meta, .. }
            | RouteFunction::GetLogs { meta, .. }
            | RouteFunction::ApiCaller { meta, .. } => meta,
        };

        // only add auth required endpoints to this router.
        if meta.auth_level >= 1 {
            router = add_route_to_router(router, route_func, &help_text);
        }
    }
    router
}

mod get {
    use super::*;

    pub async fn protected(auth_session: AuthSession, messages: Messages) -> impl IntoResponse {
        match auth_session.user {
            Some(user) => {
                let rendered =
                    render_protected_template(messages.into_iter().collect(), &user.username)
                        .expect("failed to render template");

                Html(rendered).into_response()
            }
            None => StatusCode::INTERNAL_SERVER_ERROR.into_response(),
        }
    }
}

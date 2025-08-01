mod shell_script_run;
mod updatefromgameapi;

use shell_script_run::run_command_handler;
use updatefromgameapi::api_proxy;

use crate::htmlv::{HtmlV, RenderHtml};
use crate::my_api_config::RouteFunction;
use crate::procmon::system_usage_handler;

use axum::{Router, extract::Query, response::IntoResponse, routing::get, routing::post};
use serde::Deserialize;
use std::process::{Command, Stdio};
use std::time::{SystemTime, UNIX_EPOCH};

use std::collections::HashMap;
use tower_http::services::ServeDir;

use serde_json::Value;
use std::fs;
use std::fs::OpenOptions;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::thread;

fn load_routes_from_json(path: &str) -> Vec<RouteFunction> {
    let contents = fs::read_to_string(path).expect("Failed to read routes JSON file");
    serde_json::from_str(&contents).expect("Failed to parse routes JSON")
}

pub fn load_routes_from_dir(dir_path: &str) -> Vec<RouteFunction> {
    let mut all_routes = Vec::new();

    println!("Scanning directory: {dir_path}");

    // ===Load all .html files into a HashMap, and keep full file name as key ===
    let mut html_map = HashMap::new();
    if let Ok(entries) = fs::read_dir(dir_path) {
        for entry in entries.flatten() {
            let path = entry.path();

            if path.extension().map_or(false, |ext| ext == "html") {
                if let Some(filename) = path.file_name().and_then(|s| s.to_str()) {
                    let content = fs::read_to_string(&path)
                        .unwrap_or_else(|_| panic!("Failed to read HTML file: {:?}", path));
                    println!("Loaded HTML file: {}", filename);
                    html_map.insert(filename.to_string(), content);
                }
            }
        }
    } else {
        panic!("Failed to open directory: {dir_path}");
    }

    // ===find and parse all .json files===
    let json_paths: Vec<PathBuf> = fs::read_dir(dir_path)
        .expect("Failed to read directory")
        .filter_map(|entry| {
            let path = entry.ok()?.path();
            if path.extension()? == "json" {
                Some(path)
            } else {
                None
            }
        })
        .collect();

    println!("Found {} JSON file(s).", json_paths.len());

    // ===process each JSON file===
    for json_path in json_paths {
        println!("Processing JSON file: {:?}", json_path);

        let json_content = fs::read_to_string(&json_path)
            .unwrap_or_else(|_| panic!("Failed to read JSON file: {:?}", json_path));

        let parsed: Value = serde_json::from_str(&json_content)
            .unwrap_or_else(|_| panic!("Invalid JSON format in: {:?}", json_path));

        let route_entries = match parsed {
            Value::Array(arr) => arr,
            obj @ Value::Object(_) => vec![obj],
            _ => panic!("Unsupported JSON structure in {:?}", json_path),
        };

        //=== substitute body field with html file contents, if the value in "body" matches===
        for mut entry in route_entries {
            if let Some(body_key) = entry.get("body").and_then(|b| b.as_str()) {
                if let Some(body_content) = html_map.get(body_key) {
                    println!("Replacing 'body' with content from file: {}", body_key);
                    entry["body"] = Value::String(body_content.clone());
                }
            }

            // === deserialize into a RouteFunction ===
            let route: RouteFunction = serde_json::from_value(entry)
                .unwrap_or_else(|e| panic!("Failed to parse route from {:?}: {e}", json_path));

            all_routes.push(route);
        }
    }

    println!("Loaded {} route(s).", all_routes.len());
    all_routes
}

// For NormalPage:
async fn normal_page_handler(title: String, body: String) -> HtmlV<String> {
    HtmlV((title, body).render_html_from_int(0))
}

// For NormalPageTemplate:
async fn normal_page_template_handler(title: String, body: String, template: i32) -> HtmlV<String> {
    HtmlV((title, body).render_html_from_int(template))
}

pub async fn get_logs_handler(
    Query(params): Query<HashMap<String, String>>,
    log_file_types: Option<Vec<String>>,
    title: String,
) -> HtmlV<String> {
    // Use provided log files or raise exception.
    let log_paths = log_file_types.unwrap_or_else(|| panic!("Log not found"));

    // Parse selected log index from query
    let selected_index = params.get("log").and_then(|v| v.parse::<usize>().ok());

    // Determine selected log path (if valid index), or default to first
    let log_file_path = selected_index
        .and_then(|i| log_paths.get(i))
        .unwrap_or(&log_paths[0]);

    // Run tail command
    let output = Command::new("tail")
        .arg("-n")
        .arg("100")
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

            HtmlV((title, format!("<ul>{}</ul>", log_lines)).render_html_from_int(1))
        }
        Ok(output) => {
            let error_message = String::from_utf8_lossy(&output.stderr).to_string();
            HtmlV((title, format!("Error: {}", error_message)).render_html_from_int(1))
        }
        Err(e) => {
            HtmlV((title, format!("<pre>Failed to run tail: {}</pre>", e)).render_html_from_int(1))
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
/*
pub fn build_router_from_route_functions(route_functions: Vec<RouteFunction>) -> Router {
    let mut router = Router::new();

    for route_func in route_functions.into_iter() {
        match route_func {
            RouteFunction::NormalPage { route, title, body } => {
                // Clone for move into async closure
                let title_clone = title.clone();
                let body_clone = body.clone();

                router = router.route(
                    &route,
                    get(move || normal_page_handler(title_clone.clone(), body_clone.clone())),
                );
            }
            RouteFunction::NormalPageTemplate {
                route,
                title,
                body,
                template_num,
            } => {
                // Clone for move into async closure
                let title_clone = title.clone();
                let body_clone = body.clone();

                router = router.route(
                    &route,
                    get(move || {
                        normal_page_template_handler(
                            title_clone.clone(),
                            body_clone.clone(),
                            template_num.clone(),
                        )
                    }),
                );
            }
            RouteFunction::RunCommand {
                route,
                lock_file_path,
                log_file_path,
                script_file_path,
                title,
                template_num,
            } => {
                let lock_clone = lock_file_path.clone();
                let log_clone = log_file_path.clone();
                let script_clone = script_file_path.clone();
                let title_clone = title.clone();

                router = router.route(
                    &route,
                    get(move || {
                        run_command_handler(
                            lock_clone.clone(),
                            log_clone.clone(),
                            script_clone.clone(),
                            title_clone.clone(),
                            template_num.clone(),
                        )
                    }),
                );
            }
            RouteFunction::GetLogs {
                route,
                log_file_types,
                title,
            } => {
                let log_types = log_file_types.clone();
                let title = title.clone();
                router = router.route(
                    &route,
                    get(move |query| {
                        get_logs_handler_wrapped(query, log_types.clone(), title.clone())
                    }),
                );
            } // Add more cases here for additional variants...
        }
    }

    router
}
*/
pub fn build_router_from_route_functions(route_functions: Vec<RouteFunction>) -> Router {
    let mut router = Router::new();

    for route_func in route_functions {
        let meta = match &route_func {
            RouteFunction::NormalPage { meta, .. }
            | RouteFunction::RunCommand { meta, .. }
            | RouteFunction::GetLogs { meta, .. } => meta,
        };

        println!("{},{},{}", meta.route, meta.title, meta.description);
        match route_func {
            RouteFunction::NormalPage { meta, body } => {
                let title_clone = meta.title.clone();
                let body_clone = body.clone();
                let template_num = meta.template_num;
                router = router.route(
                    &meta.route,
                    get(move || {
                        normal_page_template_handler(
                            title_clone.clone(),
                            body_clone.clone(),
                            template_num.clone(),
                        )
                    }),
                );
            }

            RouteFunction::RunCommand {
                meta,
                lock_file_path,
                log_file_path,
                script_file_path,
            } => {
                let lock_clone = lock_file_path.clone();
                let log_clone = log_file_path.clone();
                let script_clone = script_file_path.clone();
                let title_clone = meta.title.clone();
                router = router.route(
                    &meta.route,
                    get(move || {
                        run_command_handler(
                            lock_clone.clone(),
                            log_clone.clone(),
                            script_clone.clone(),
                            title_clone.clone(),
                            meta.template_num,
                        )
                    }),
                );
            }

            RouteFunction::GetLogs {
                meta,
                log_file_types,
            } => {
                let title = meta.title.clone();
                let log_types = log_file_types.clone();
                router = router.route(
                    &meta.route,
                    get(move |query| {
                        get_logs_handler_wrapped(query, log_types.clone(), title.clone())
                    }),
                );
            }
        }
    }

    router
}

pub fn routes() -> Router {
    let route_functions = load_routes_from_dir("./json_routes");
    build_router_from_route_functions(route_functions)
        // MANUAL ROUTES.
        // Extension 1, System Resource Monitor.
        .route("/procmon", get(system_usage_handler))
        // Extension 2, Game Api caller.
        .route("/api-proxy", get(api_proxy))
        .nest_service("/static", ServeDir::new("statics"))
}

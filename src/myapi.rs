pub(crate) mod api_call_system;
pub(crate) mod handlers;
pub(crate) mod shell_script_run;

use crate::my_api_config::RouteFunction;
use crate::procmon::system_usage_handler;

use axum::{Router, routing::get};
use html_escape;
use std::collections::HashMap;
use tower_http::services::ServeDir;

use serde_json::Value;
use std::fs;

use std::path::PathBuf;

use tracing;

/// Get all JSON files from dir_path and load them
/// valid RouteFunction structs
///
pub fn load_routes_from_dir(dir_path: &str) -> Vec<RouteFunction> {
    let mut all_routes = Vec::new();

    tracing::info!("Scanning directory: {dir_path}");

    // load all .html files into a HashMap, and keep full file name as key
    let mut html_map = HashMap::new();
    if let Ok(entries) = fs::read_dir(dir_path) {
        for entry in entries.flatten() {
            let path = entry.path();

            if path.extension().map_or(false, |ext| ext == "html") {
                if let Some(filename) = path.file_name().and_then(|s| s.to_str()) {
                    let content = fs::read_to_string(&path)
                        .unwrap_or_else(|_| panic!("Failed to read HTML file: {:?}", path));
                    tracing::info!("Loaded HTML file: {}", filename);
                    html_map.insert(filename.to_string(), content);
                }
            }
        }
    } else {
        panic!("Failed to open directory: {dir_path}");
    }

    // find and parse all .json files
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

    tracing::info!("Found {} JSON file(s).", json_paths.len());

    // process each JSON file
    for json_path in json_paths {
        tracing::info!("Processing JSON file: {:?}", json_path);

        let json_content = fs::read_to_string(&json_path)
            .unwrap_or_else(|_| panic!("Failed to read JSON file: {:?}", json_path));

        let parsed: Value = serde_json::from_str(&json_content)
            .unwrap_or_else(|_| panic!("Invalid JSON format in: {:?}", json_path));

        let route_entries = match parsed {
            Value::Array(arr) => arr,
            obj @ Value::Object(_) => vec![obj],
            _ => panic!("Unsupported JSON structure in {:?}", json_path),
        };

        // substitute body field with html file contents, if the value in "body" matches
        for mut entry in route_entries {
            if let Some(body_key) = entry.get("body").and_then(|b| b.as_str()) {
                if let Some(body_content) = html_map.get(body_key) {
                    tracing::info!("Replacing 'body' with content from file: {}", body_key);
                    entry["body"] = Value::String(body_content.clone());
                }
            }

            //  deserialize into a RouteFunction
            let route: RouteFunction = serde_json::from_value(entry)
                .unwrap_or_else(|e| panic!("Failed to parse route from {:?}: {e}", json_path));

            all_routes.push(route);
        }
    }
    all_routes.sort_by_key(|route_func| {
        let meta = match route_func {
            RouteFunction::NormalPage { meta, .. }
            | RouteFunction::HelpPage { meta, .. }
            | RouteFunction::CommandStatus { meta, .. }
            | RouteFunction::RunCommand { meta, .. }
            | RouteFunction::GetLogs { meta, .. }
            | RouteFunction::ApiCaller { meta, .. } => meta,
        };
        meta.help_order
    });

    tracing::info!("Loaded {} route(s).", all_routes.len());
    all_routes
}

/// Build up the body for a help page
/// while taking in an array of RotueFunctions
pub fn build_help_page_html(route_functions: Vec<RouteFunction>) -> String {
    let mut html = String::from("<h3>Help Page</h3>\n<ul>\n");

    for route_func in route_functions {
        let meta = match route_func {
            RouteFunction::NormalPage { meta, .. }
            | RouteFunction::HelpPage { meta, .. }
            | RouteFunction::CommandStatus { meta, .. }
            | RouteFunction::RunCommand { meta, .. }
            | RouteFunction::GetLogs { meta, .. }
            | RouteFunction::ApiCaller { meta, .. } => meta,
        };

        let route_prefix = if meta.auth_level >= 1 {
            "/protected"
        } else {
            ""
        };

        html.push_str(&format!(
            "    <li><a href=\"{route_prefix}{route}\">{title}</a>: {desc}</li>\n",
            route_prefix = route_prefix,
            route = html_escape::encode_safe(&meta.route),
            title = html_escape::encode_safe(&meta.title),
            desc = html_escape::encode_safe(&meta.description),
        ));
    }

    html.push_str("</ul>\n");
    html
}

/// Given a Router and a RouteFunction, add it to the router.
/// Must pass in the help_text for the /help page.
pub fn add_route_to_router(router: Router, route_func: RouteFunction, help_text: &str) -> Router {
    let meta = match &route_func {
        RouteFunction::NormalPage { meta, .. }
        | RouteFunction::HelpPage { meta, .. }
        | RouteFunction::CommandStatus { meta, .. }
        | RouteFunction::RunCommand { meta, .. }
        | RouteFunction::GetLogs { meta, .. }
        | RouteFunction::ApiCaller { meta, .. } => meta,
    };

    tracing::info!(
        "{},{},{},{}",
        meta.route,
        meta.title,
        meta.description,
        meta.template_num
    );
    let (path, route) = route_func.into_route(help_text);
    router.route(&path, route)
}

pub fn build_router_from_route_functions(route_functions: Vec<RouteFunction>, prot: i32) -> Router {
    let mut router = Router::new();
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

        if prot == 0 && meta.auth_level <= 0 {
            router = add_route_to_router(router, route_func, &help_text);
        }
    }

    router
}

pub fn routes() -> Router {
    let route_functions = load_routes_from_dir("./json_routes");
    build_router_from_route_functions(route_functions, 0)
        // MANUAL ROUTES.
        // Extension 1, System Resource Monitor.
        .route("/procmon", get(system_usage_handler))
        .nest_service("/static", ServeDir::new("statics"))
}

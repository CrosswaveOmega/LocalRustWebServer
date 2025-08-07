///! # API CONFIG
///! Base Configuration of all possible RouteFunctions
///! Definable in the JSON
///! `RouteFunction` is an enum that represents different behaviors
///! for handling HTTP requests, depending on the `function_type`
///! specified in the associated data.
use serde::Deserialize;
use std::collections::HashMap;

use crate::auth::users::AuthSession;
use crate::myapi::handlers::{
    api_caller_wrapped, get_logs_handler_wrapped, normal_page_template_handler,
    normal_page_template_handler_secure,
};
use crate::myapi::shell_script_run::{run_command_handler, run_command_handler_secure};

use axum::routing::get;
fn default_description() -> String {
    "No description, please set one for this route in /json_routes".to_string()
}

fn default_help_order() -> i32 {
    256
}

/// Base struct with all the common parameters for each json route.
#[derive(Deserialize, Debug, Clone)]
pub struct RouteMeta {
    /// route- the get route
    pub route: String,
    /// title- the title
    pub title: String,
    /// description- the description, seen on the help page
    #[serde(default = "default_description")]
    pub description: String,

    /// help_order, order displayed on the help page.  default 0.
    #[serde(default = "default_help_order")]
    pub help_order: i32,
    /// template_num- the html template number to use.  by default, it's 0.
    #[serde(default)]
    pub template_num: i32,
    /// auth_level- the authorization level required.  by default, it's 0.
    #[serde(default)]
    pub auth_level: i32,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ApiEndpointConfig {
    //The API Path for this endpoint
    pub path: String,
    #[serde(default)]
    //The default parameters for this endpoint, if they exist at all
    pub default_params: HashMap<String, String>,
}

/// Enumeration for the JSON structures associated with each possible RouteFunction
///
/// This enum is used to decode a JSON file where each variant corresponds to a specific
/// route function identified by a `function_type`, as well as call the appropiate handler
/// for each Function type
#[derive(Deserialize, Debug, Clone)]
#[serde(tag = "function_type")]
pub enum RouteFunction {
    /// Enum for selecting the route function based on
    /// the function_type param

    #[serde(rename = "normal_page")]
    NormalPage {
        /// Normal page.  Just send in a usually body
        /// and/or optionally a special html template.
        ///

        #[serde(flatten)]
        meta: RouteMeta,
        /// body- a String to display.
        body: String,
    },
    #[serde(rename = "help_page")]
    HelpPage {
        /// Automatically generated help page
        /// populated using the route, title, and description
        /// of each item.
        ///
        #[serde(flatten)]
        meta: RouteMeta,
    },

    #[serde(rename = "run_command")]
    RunCommand {
        /// For running a specific .sh script.  
        ///
        /// Intended for use on linux systems,
        #[serde(flatten)]
        meta: RouteMeta,

        /// lock_file_path- required, file path to an aquirable lock file.
        lock_file_path: String,
        /// log_file_path- required, file path to a specific log file that the script's output will be piped to.
        log_file_path: String,

        /// script_file_path- required, file path to a .sh file to be run in bash.
        script_file_path: String,
    },

    #[serde(rename = "get_logs")]
    GetLogs {
        /// use tail on a specific log file
        /// within the log_file_types list.
        /// Used as/endpoint?log=integerhere

        #[serde(flatten)]
        meta: RouteMeta,
        /// log_file_types- list of log file paths.
        log_file_types: Option<Vec<String>>,
    },

    #[serde(rename = "call_api")]
    ApiCaller {
        /// For calling a specific external api endpoint
        /// must always pass in the "endpoint" parameter
        #[serde(flatten)]
        meta: RouteMeta,
        /// Base url for api
        base_url: String,
        /// Mapping from endpoint keyword to path
        endpoints: HashMap<String, ApiEndpointConfig>,
    },
}

impl RouteFunction {
    pub fn into_route(self, help_text: &str) -> (String, axum::routing::MethodRouter) {
        match self {
            RouteFunction::NormalPage { meta, body } => {
                let title = meta.title.clone();
                let body = body.clone();
                let template = meta.template_num;

                if meta.auth_level <= 0 {
                    let route = get(move || {
                        normal_page_template_handler(title.clone(), body.clone(), template)
                    });

                    (meta.route.clone(), route)
                } else {
                    let route = get(move |auth_session: AuthSession| {
                        normal_page_template_handler_secure(
                            auth_session,
                            title.clone(),
                            body.clone(),
                            template,
                        )
                    });

                    (meta.route.clone(), route)
                }
            }

            RouteFunction::HelpPage { meta } => {
                let title = meta.title.clone();
                let body = help_text.to_string();
                let template = 0;

                let route = get(move || {
                    normal_page_template_handler(title.clone(), body.clone(), template)
                });

                (meta.route.clone(), route)
            }

            RouteFunction::RunCommand {
                meta,
                lock_file_path,
                log_file_path,
                script_file_path,
            } => {
                let lock = lock_file_path.clone();
                let log = log_file_path.clone();
                let script = script_file_path.clone();
                let title = meta.title.clone();
                let template = meta.template_num;
                if meta.auth_level <= 0 {
                    let route = get(move || {
                        run_command_handler(
                            lock.clone(),
                            log.clone(),
                            script.clone(),
                            title.clone(),
                            template,
                        )
                    });

                    (meta.route.clone(), route)
                } else {
                    let route = get(move |auth_session: AuthSession| {
                        run_command_handler_secure(
                            auth_session,
                            lock.clone(),
                            log.clone(),
                            script.clone(),
                            title.clone(),
                            template,
                        )
                    });

                    (meta.route.clone(), route)
                }
            }

            RouteFunction::GetLogs {
                meta,
                log_file_types,
            } => {
                let title = meta.title.clone();
                let logs = log_file_types.clone();

                let route =
                    get(move |query| get_logs_handler_wrapped(query, logs.clone(), title.clone()));

                (meta.route.clone(), route)
            }

            RouteFunction::ApiCaller {
                meta,
                base_url,
                endpoints,
            } => {
                let base_url = base_url.clone();
                let endpoints = endpoints.clone();

                let route = get(move |query| {
                    api_caller_wrapped(query, base_url.clone(), endpoints.clone())
                });

                (meta.route.clone(), route)
            }
        }
    }
}

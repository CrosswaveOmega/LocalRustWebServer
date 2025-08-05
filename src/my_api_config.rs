use std::collections::HashMap;

use serde::Deserialize;

/// Enumeration representing the different types of route functions.
///
/// This enum is used to decode a JSON file where each variant corresponds to a specific
/// route function identified by a `function_type`.
///

fn default_description() -> String {
    "No description, please set one for this route in /json_routes".to_string()
}

fn default_help_order() -> i32 {
    256
}

#[derive(Deserialize, Debug, Clone)]
pub struct RouteMeta {
    /// Base class with all the common parameters for each route.
    /// those being
    /// route- the get route
    /// title- the title
    /// description- the description, seen on the help page
    /// template_num- the html template number to use.  by default, it's 0.
    pub route: String,
    pub title: String,
    #[serde(default = "default_description")]
    pub description: String,
    #[serde(default)]
    pub template_num: i32,
    #[serde(default = "default_help_order")]
    pub help_order: i32,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ApiEndpointConfig {
    //The API Path for this endpoint
    pub path: String,
    #[serde(default)]
    //The default parameters for this endpoint, if they exist at all
    pub default_params: HashMap<String, String>,
}

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
        /// body- a String to display.
        #[serde(flatten)]
        meta: RouteMeta,
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
        /// Intended for use on linux systems.
        ///
        ///
        /// lock_file_path- required, a file path to an aquirable lock file.
        /// log_file_path- required, file path to a specific log file that the script's output will be piped to.
        /// script_file_path- required, a .sh file to be run in bash.
        #[serde(flatten)]
        meta: RouteMeta,
        lock_file_path: String,
        log_file_path: String,
        script_file_path: String,
    },

    #[serde(rename = "get_logs")]
    GetLogs {
        /// use tail on a specific log file
        /// within the log_file_types list.
        ///
        /// log_file_types- list of log file paths.
        /// /endpoint?log=integerhere
        #[serde(flatten)]
        meta: RouteMeta,
        log_file_types: Option<Vec<String>>,
    },

    #[serde(rename = "call_api")]
    ApiCaller {
        /// For calling a specific external api endpoint
        ///
        #[serde(flatten)]
        meta: RouteMeta,
        /// Base url for api
        base_url: String,
        /// Mapping from endpoint keyword to path
        endpoints: HashMap<String, ApiEndpointConfig>,
    },
}

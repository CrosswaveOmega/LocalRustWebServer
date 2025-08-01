use serde::Deserialize;

/// Enumeration representing the different types of route functions.
///
/// This enum is used to decode a JSON file where each variant corresponds to a specific
/// route function identified by a `function_type`.
///

fn default_description() -> String {
    "No description".to_string()
}
/// Base class with all the common parameters for each route.
///
#[derive(Deserialize, Debug, Clone)]
pub struct RouteMeta {
    pub route: String,
    pub title: String,
    #[serde(default = "default_description")] // Set a default description
    pub description: String,
    #[serde(default)]
    pub template_num: i32,
}

#[derive(Deserialize, Debug, Clone)]
#[serde(tag = "function_type")]
pub enum RouteFunction {
    #[serde(rename = "normal_page")]
    NormalPage {
        #[serde(flatten)]
        meta: RouteMeta,
        body: String,
    },
    #[serde(rename = "help_page")]
    HelpPage {
        #[serde(flatten)]
        meta: RouteMeta,
    },

    #[serde(rename = "run_command")]
    RunCommand {
        #[serde(flatten)]
        meta: RouteMeta,
        lock_file_path: String,
        log_file_path: String,
        script_file_path: String,
    },

    #[serde(rename = "get_logs")]
    GetLogs {
        #[serde(flatten)]
        meta: RouteMeta,
        log_file_types: Option<Vec<String>>,
    },
}
/*
#[derive(Deserialize, Debug, Clone)]
#[serde(tag = "function_type")]
pub enum RouteFunction {
    /// Static page with body content.
    #[serde(rename = "normal_page")]
    NormalPage {
        route: String,
        title: String,
        body: String,
    },

    /// Load in a specific HTML template.
    #[serde(rename = "normal_page_template")]
    NormalPageTemplate {
        route: String,
        title: String,
        body: String,
        template_num: i32,
    },

    /// Run a command with lock and log files.
    #[serde(rename = "run_command")]
    RunCommand {
        route: String,
        title: String,
        lock_file_path: String,
        log_file_path: String,
        script_file_path: String,
        #[serde(default)]
        template_num: i32,
    },

    /// Fetch logs, with optional log file types list.
    #[serde(rename = "get_logs")]
    GetLogs {
        route: String,
        title: String,
        log_file_types: Option<Vec<String>>,
    },
    // Add more variants as needed...
}*/

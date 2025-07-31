use serde::Deserialize;

#[derive(Deserialize, Debug, Clone)]
#[serde(tag = "function_type")]
pub enum RouteFunction {
    // Static page with body content
    #[serde(rename = "normal_page")]
    NormalPage {
        route: String,
        title: String,
        body: String,
    },
    //load in a specific HTML template
    #[serde(rename = "normal_page_template")]
    NormalPageTemplate {
        route: String,
        title: String,
        body: String,
        template_num:i32,
    },

    // Run a command with lock and log files
    #[serde(rename = "run_command")]
    RunCommand {
        route: String,
        title: String,
        lock_file_path: String,
        log_file_path: String,
        script_file_path: String,
    },

    // Fetch logs, with optional log file types list
    #[serde(rename = "get_logs")]
    GetLogs {
        route: String,
        title: String,
        log_file_types: Option<Vec<String>>,
    },

    // Add more variants as needed...
}

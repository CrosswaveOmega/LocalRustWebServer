use axum::http::{HeaderValue, header};
use axum::response::{IntoResponse, Response};
use mime::TEXT_HTML_UTF_8;
use std::collections::HashMap;
use std::fs;
use std::path::Path;
use std::sync::OnceLock;
use tera::{Context, Tera};
use tracing;

static TERA: OnceLock<Tera> = OnceLock::new();
static TEMPLATE_MAP: OnceLock<HashMap<i32, String>> = OnceLock::new();

const TEMPLATE_CONFIG_PATH: &str = "./templates";

/// Initializes the global Tera engine by loading all templates under ./templates
/// Should be called once at startup.
pub fn init_tera() {
    let tera = Tera::new("templates/**/*").expect("Failed to initialize Tera");
    if TERA.set(tera).is_err() {
        tracing::warn!("Tera was already initialized");
    } else {
        tracing::info!("Tera initialized with templates from ./templates/**/*");
    }
}

/// Retrieves a reference to the global Tera instance.
/// Panics if it hasn't been initialized yet.
pub fn get_tera() -> &'static Tera {
    TERA.get()
        .expect("Tera is not initialized. Call init_tera() first.")
}

/// Read the contents of a json file, and
/// add it into mapped.
fn parse_and_extend_template_map(
    file_path: &Path,
    mapped: &mut HashMap<i32, String>,
) -> Result<(), String> {
    let raw = fs::read_to_string(file_path)
        .map_err(|e| format!("Failed to read {}: {}", file_path.display(), e))?;

    let parsed: HashMap<String, String> = serde_json::from_str(&raw)
        .map_err(|e| format!("Failed to parse {}: {}", file_path.display(), e))?;

    for (k, v) in parsed {
        if let Ok(key) = k.parse::<i32>() {
            tracing::info!("Loaded template: {} => {}", key, v);
            mapped.insert(key, v);
        } else {
            tracing::warn!("Skipping invalid template key '{}'", k);
        }
    }

    Ok(())
}

/// Loads the template configuration from `./templates/template_config.json`
/// and any additional `template_config.json` files found in the `templates` directory
/// into the global `TEMPLATE_MAP`.
///
/// This function expects the JSON files to map stringified integers (as keys)
/// to template file names. Keys are parsed into `i32`, and only valid entries
/// are included in the final map.
///
/// The function will panic if any config file is missing or malformed.
/// It will also panic if `TEMPLATE_MAP` has already been set.
pub fn load_template_config() {
    tracing::info!("Loading Templates from /templates");
    if !Path::new(TEMPLATE_CONFIG_PATH).exists() {
        tracing::error!("Warning: ./templates is not a valid path.");
        return;
    }

    let template_dir = Path::new(TEMPLATE_CONFIG_PATH);

    let mut mapped = HashMap::new();

    let initial_config_path = template_dir.join("template_config.json");
    parse_and_extend_template_map(&initial_config_path, &mut mapped)
        .expect("Failed to load ./templates/template_config.json");

    tracing::info!("Loading in /templates/template_config.json");

    // Load additional .json files
    let entries = fs::read_dir(TEMPLATE_CONFIG_PATH).expect("Failed to read ./templates directory");
    for entry in entries {
        let entry = entry.expect("Failed to read entry");
        let path = entry.path();
        if path.is_file()
            && path.extension().unwrap_or_default() == "json"
            && path != initial_config_path
        {
            if let Err(e) = parse_and_extend_template_map(&path, &mut mapped) {
                tracing::warn!("{}", e);
            }
        }
    }
    for (key, value) in &mapped {
        tracing::info!("Template config {}: {}", key, value);
    }
    init_tera();
    TEMPLATE_MAP.set(mapped).expect("TEMPLATE_MAP already set");
}

/// Trait for rendering a value into an HTML string using a specified template.
///
/// This trait supports dynamic HTML generation using a shared template system.
/// Templates are populated by substituting `{{ title }}` and `{{ body }}` markers.
pub trait RenderHtml {
    fn render_html(self, template_type: i32) -> String;
    fn render_html_from_int(self, template_type: i32) -> String
    where
        Self: Sized,
    {
        self.render_html(template_type)
    }
}

/// Implementations for RenderHTML with different types of strings
impl RenderHtml for (&str, &str) {
    fn render_html(self, template_type: i32) -> String {
        let template_id = template_type;

        let template_name = TEMPLATE_MAP
            .get()
            .and_then(|m| m.get(&template_id))
            .cloned()
            .unwrap_or_else(|| "template.html".to_string());

        // get tera instance
        let tera = get_tera();

        // initalize context
        let mut context = Context::new();
        context.insert("title", self.0);
        context.insert("body", self.1);

        context.insert("username", "");
        match tera.render(&template_name, &context) {
            Ok(html) => html,
            Err(err) => {
                tracing::error!("Template rendering failed for {}: {}", template_name, err);
                format!("<h1>{}</h1><div>{}</div>", self.0, self.1)
            }
        }
    }
}
impl RenderHtml for (&str, &str, &str) {
    fn render_html(self, template_type: i32) -> String {
        let template_id = template_type;

        let template_name = TEMPLATE_MAP
            .get()
            .and_then(|m| m.get(&template_id))
            .cloned()
            .unwrap_or_else(|| "template.html".to_string());

        // get tera instance
        let tera = get_tera();

        // initalize context
        let mut context = Context::new();
        context.insert("title", self.0);
        context.insert("username", self.2);

        // render body with context
        // Just cloning tera for now...
        let rendered_body = match tera.clone().render_str(self.1, &context) {
            Ok(result) => result,
            Err(err) => {
                tracing::error!("Inner body rendering failed: {}", err);
                self.1.to_string()
            }
        };

        context.insert("body", &rendered_body);

        match tera.render(&template_name, &context) {
            Ok(html) => html,
            Err(err) => {
                tracing::error!("Template rendering failed for {}: {}", template_name, err);
                format!("<h1>{}</h1><div>{}</div>", self.0, rendered_body)
            }
        }
    }
}

impl RenderHtml for (String, &str) {
    fn render_html(self, template_type: i32) -> String {
        (self.0.as_str(), self.1).render_html(template_type)
    }
}
impl RenderHtml for (String, String) {
    fn render_html(self, template_type: i32) -> String {
        (self.0.as_str(), self.1.as_str()).render_html(template_type)
    }
}
impl RenderHtml for (String, String, String) {
    fn render_html(self, template_type: i32) -> String {
        (self.0.as_str(), self.1.as_str(), self.2.as_str()).render_html(template_type)
    }
}

impl RenderHtml for (&str, String) {
    fn render_html(self, template_type: i32) -> String {
        (self.0, self.1.as_str()).render_html(template_type)
    }
}

impl RenderHtml for &str {
    fn render_html(self, template_type: i32) -> String {
        ("The Gnomelab", self).render_html(template_type)
    }
}

/// a generic HTML response.
///
/// This struct will automatically add a link to the 98.css stylesheet
/// and set the Content-Type: text/html.
#[derive(Clone, Copy, Debug)]
#[must_use]
pub struct HtmlV<T>(pub T);

impl<T> IntoResponse for HtmlV<T>
where
    T: Into<String>,
{
    fn into_response(self) -> Response {
        let html = self.0.into();
        (
            [(
                header::CONTENT_TYPE,
                HeaderValue::from_static(TEXT_HTML_UTF_8.as_ref()),
            )],
            html,
        )
            .into_response()
    }
}

impl<T> From<T> for HtmlV<T> {
    fn from(inner: T) -> Self {
        Self(inner)
    }
}

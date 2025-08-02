
use axum::http::{HeaderValue, header};
use axum::response::{IntoResponse, Response};
use mime::TEXT_HTML_UTF_8;
use std::collections::HashMap;
use std::fs;
use std::path::Path;
use std::sync::OnceLock;
use tracing;

static TEMPLATE_MAP: OnceLock<HashMap<i32, String>> = OnceLock::new();

/// Loads the template configuration from `./templates/template_config.json`
/// into the global `TEMPLATE_MAP`.
///
/// This function expects the JSON file to map stringified integers (as keys)
/// to template file names. Keys are parsed into `i32`, and only valid entries
/// are included in the final map.
///
/// The function will panic if the config file is missing or malformed.
/// It will also panic if `TEMPLATE_MAP` has already been set.
pub fn load_template_config() {

    if !Path::new("./templates").exists() {
        eprintln!("Warning: ./templates is not a valid path.");
    }

    let raw = fs::read_to_string("./templates/template_config.json")
        .expect("Missing ./templates/template_config.json");

    let parsed: HashMap<String, String> =
        serde_json::from_str(&raw).expect("Failed to parse template_config.json");

    let mapped = parsed
        .into_iter()
        .filter_map(|(k, v)| k.parse::<i32>().ok().map(|key| (key, v)))
        .collect::<HashMap<_, _>>();

    for (key, value) in &mapped {
        tracing::info!("{}: {}", key, value);
    }

    TEMPLATE_MAP.set(mapped).expect("TEMPLATE_MAP already set");
}

pub trait RenderHtml {
    /// Trait for rendering a value into an HTML string using a specified template.
    ///
    /// This trait supports dynamic HTML generation using a shared template system.
    /// Templates are populated by substituting `{{ title }}` and `{{ body }}` markers.
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

        let template_path = format!("./templates/{}", template_name);
        let template_string =
            fs::read_to_string(&template_path).unwrap_or_else(|_| "{{ body|safe }}".to_string());

        template_string
            .replace("{{ title }}", self.0)
            .replace("{{ title|safe }}", self.0)
            .replace("{{ body }}", self.1)
            .replace("{{ body|safe }}", self.1)
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

/// An HTML response.
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

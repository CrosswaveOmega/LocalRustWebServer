use crate::{
    htmlv::get_tera,
    my_api_config::RouteFunction,
    myapi::{add_route_to_router, build_help_page_html, load_routes_from_dir},
};

use crate::auth::users::AuthSession;
use axum::{
    Router,
    http::StatusCode,
    response::{Html, IntoResponse},
    routing::get,
};
use axum_messages::{Message, Messages};
use tera::Context;

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

pub fn router() -> Router<()> {
    let route_functions = load_routes_from_dir("./json_routes");
    build_secure_router_from_route_functions(route_functions, 1)
}

pub fn build_secure_router_from_route_functions(
    route_functions: Vec<RouteFunction>,
    prot: i32,
) -> Router {
    let mut router = Router::new().route("/", get(self::get::protected));
    let help_text = build_help_page_html(route_functions.clone());

    for route_func in route_functions {
        let meta = match &route_func {
            RouteFunction::NormalPage { meta, .. }
            | RouteFunction::HelpPage { meta, .. }
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

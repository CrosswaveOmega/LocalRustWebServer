//! # Usage examples
//!
//! File in json_routes
//! ```
//! {"function_type": "normal_page",
//! "route": "/help",
//! "title": "Help Page",
//! "body": "help.html"
//! }
//! ```

mod add_user;
mod app;
mod auth;
mod certs;
mod config;
mod htmlv;
mod logging;
mod my_api_config;
mod myapi;
mod procmon;

use std::env;

use add_user::{adduser_from_prompt};
use app::RustyWebApp;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    logging::init_logging();
    let args: Vec<String> = env::args().collect();

    if let Some(cmd) = args.get(1) {
        if cmd == "add-user" {
            println!("Adding user...");

            return adduser_from_prompt().await;
        }
    }

    // default application launch
    RustyWebApp::new().await?.run().await?;
    Ok(())
}
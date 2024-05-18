mod catalog;

use axum::{serve, Router};
use catalog::get_catalog;
use leprecon::{signals::shutdown_signal, utils::configure_tracing};
use std::{env, error::Error, sync::OnceLock};
use tokio::net::TcpListener;

// Host variables
static HOST: OnceLock<String> = OnceLock::new();
static LOG_LEVEL: OnceLock<String> = OnceLock::new();

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    // Initialize env variables
    init_env();

    // Configure logging
    configure_tracing(LOG_LEVEL.get().unwrap());

    // Build application and listen to incoming requests.
    let app: Router = build_app();
    let listener: TcpListener = TcpListener::bind(HOST.get().unwrap()).await?;

    // Run the app.
    serve(listener, app)
        .with_graceful_shutdown(shutdown_signal())
        .await?;

    Ok(())
}

// Initialize env variables
fn init_env() {
    HOST.get_or_init(|| env::var("GAME_CATALOG_HOST").unwrap());
    LOG_LEVEL.get_or_init(|| env::var("LOG_LEVEL").unwrap());
}

/// Builds the application.
fn build_app() -> Router {
    Router::new().route("/game/catalog", axum::routing::get(get_catalog))
}

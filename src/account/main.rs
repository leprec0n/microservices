use std::io;

use axum::{
    http::{HeaderValue, Method},
    serve, Router,
};
use leprecon::{headers::htmx_headers, signals::shutdown_signal};
use tokio::net::TcpListener;
use tower::ServiceBuilder;
use tower_http::cors::CorsLayer;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

static ADDRESS: &str = "127.0.0.1:8081"; // !TODO move to global file that gets the value from environment variable.

#[tokio::main]
async fn main() -> io::Result<()> {
    // Configure tracing
    configure_tracing();

    // Build application and listen to incoming requests.
    let app: Router = build_app();
    let listener: TcpListener = TcpListener::bind(ADDRESS).await?;

    // Run the app.
    serve(listener, app)
        .with_graceful_shutdown(shutdown_signal())
        .await?;

    Ok(())
}

/// Configure tracing with tracing_subscriber.
fn configure_tracing() {
    tracing_subscriber::registry()
        .with(tracing_subscriber::fmt::layer())
        .init();
}

/// Builds the application.
fn build_app() -> Router {
    Router::new().layer(
        // Axum recommends to use tower::ServiceBuilder to apply multiple middleware at once, instead of repeatadly calling layer.
        // https://docs.rs/axum/latest/axum/middleware/index.html#applying-multiple-middleware
        ServiceBuilder::new().layer(
            CorsLayer::new()
                .allow_methods([Method::GET])
                .allow_origin(HeaderValue::from_static("http://127.0.0.1:80"))
                .allow_headers(htmx_headers()),
        ),
    )
}

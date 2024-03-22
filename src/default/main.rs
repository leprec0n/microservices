use std::{io, time::Duration};

use axum::{
    extract::Query,
    http::{HeaderValue, Method},
    response::Html,
    routing::get,
    serve, Router,
};
use leprecon::{headers::htmx_headers, signals::shutdown_signal};
use serde::Deserialize;
use tokio::{net::TcpListener, time::sleep};
use tower::ServiceBuilder;
use tower_http::cors::CorsLayer;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

static ADDRESS: &str = "127.0.0.1:8080"; // !TODO move to global file that gets the value from environment variable.

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
    Router::new()
        .route("/", get(root))
        .route("/loading", get(loading))
        .layer(
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

#[derive(Deserialize, Debug)]
struct Name {
    #[serde(default)]
    name: String,
}

async fn root(Query(q): Query<Name>) -> Html<String> {
    tracing::debug!("Request from {name}", name = q.name);
    Html(format!("<h1>Homepage for {name}</h1>", name = q.name))
}

async fn loading() -> Html<&'static str> {
    let duration = Duration::from_secs(3);
    sleep(duration).await;
    Html("<div>IT WORKED!</div>")
}

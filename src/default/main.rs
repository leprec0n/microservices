use std::{io, time::Duration};

use axum::{
    extract::Query,
    http::{HeaderName, HeaderValue, Method},
    response::Html,
    routing::get,
    serve, Router,
};
use leprecon::signals::shutdown_signal;
use serde::Deserialize;
use tokio::{net::TcpListener, time::sleep};
use tower::ServiceBuilder;
use tower_http::cors::CorsLayer;

static ADDRESS: &str = "127.0.0.1:8080"; // !TODO move to global file that gets the value from environment variable.

#[tokio::main]
async fn main() -> io::Result<()> {
    // Build application
    let app: Router = build_app();

    // Run the app
    let listener: TcpListener = TcpListener::bind(ADDRESS).await?;
    serve(listener, app)
        .with_graceful_shutdown(shutdown_signal())
        .await?;

    Ok(())
}

fn build_app() -> Router {
    let origin: HeaderValue = HeaderValue::from_static("http://127.0.0.1:80"); // !TODO move to global file that gets the value from environment variable.

    // Allowed cors headerDeserializers from origin !TODO might move to global file
    let cors_headers: Vec<HeaderName> = vec![
        HeaderName::from_static("hx-current-url"),
        HeaderName::from_static("hx-request"),
        HeaderName::from_static("hx-target"),
        HeaderName::from_static("hx-trigger"),
    ];

    return Router::new()
        .route("/", get(root))
        .route("/loading", get(loading))
        .layer(
            // Axum recommends to use tower::ServiceBuilder to apply multiple middleware at once, instead of repeatadly calling layer.
            // https://docs.rs/axum/latest/axum/middleware/index.html#applying-multiple-middleware
            ServiceBuilder::new().layer(
                CorsLayer::new()
                    .allow_methods([Method::GET])
                    .allow_origin(origin)
                    .allow_headers(cors_headers),
            ),
        );
}

#[derive(Deserialize)]
struct Name {
    name: String,
}

async fn root(Query(q): Query<Name>) -> Html<String> {
    Html(format!("<h1>Homepage for {name}</h1>", name = q.name))
}

async fn loading() -> Html<&'static str> {
    let duration = Duration::from_secs(3);
    sleep(duration).await;
    Html("<div>IT WORKED!</div>")
}

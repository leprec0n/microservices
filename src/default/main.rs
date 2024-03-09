use std::{
    fmt::{self, Debug},
    io,
    str::FromStr,
};

use axum::{
    extract::Query,
    http::{HeaderName, HeaderValue, Method},
    response::Html,
    routing::get,
    serve, Router,
};
use serde::{de, Deserialize, Deserializer};
use tokio::net::TcpListener;
use tower::ServiceBuilder;
use tower_http::cors::CorsLayer;

static ADDRESS: &str = "127.0.0.1:8080"; // !TODO move to global file that gets the value from environment variable. test

#[tokio::main]
async fn main() -> io::Result<()> {
    let origin: HeaderValue = HeaderValue::from_static("http://127.0.0.1:80"); // !TODO move to global file that gets the value from environment variable.

    // Allowed cors headers from origin
    let cors_headers: Vec<HeaderName> = vec![
        HeaderName::from_static("hx-current-url"),
        HeaderName::from_static("hx-request"),
        HeaderName::from_static("hx-target"),
        HeaderName::from_static("hx-trigger"),
    ];

    // Build application with routes
    let app = Router::new()
        // GET /
        .route("/home", get(home))
        .layer(
            // Axum recommends creating multiple layers via service builder inside a layer.
            ServiceBuilder::new().layer(
                CorsLayer::new()
                    .allow_methods([Method::GET])
                    .allow_origin(origin)
                    .allow_headers(cors_headers),
            ),
        );

    // Run the app
    let listener = TcpListener::bind(ADDRESS).await?;
    serve(listener, app).await?;

    Ok(())
}

// Derive from serde desirialize.
#[derive(Deserialize)]
struct Test {
    // Handles empty or non existing query parameters.
    #[serde(default, deserialize_with = "empty_string_as_none")]
    test: Option<String>,
}

fn empty_string_as_none<'de, D, T>(de: D) -> Result<Option<T>, D::Error>
where
    D: Deserializer<'de>,
    T: FromStr,
    T::Err: fmt::Display,
{
    let opt = Option::<String>::deserialize(de)?;
    match opt.as_deref() {
        None | Some("") => Ok(None),
        Some(s) => FromStr::from_str(s).map_err(de::Error::custom).map(Some),
    }
}

impl Debug for Test {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Test").field("test", &self.test).finish()
    }
}

async fn home(Query(test): Query<Test>) -> Html<&'static str> {
    println!("{:?}", test);
    Html("<h1>Hello World!</h1>")
}

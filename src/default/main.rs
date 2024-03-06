use std::{fmt::{self, Debug}, io, str::FromStr};

use axum::{extract::Query, response::Html, routing::get, serve, Router};
use serde::{de, Deserialize, Deserializer};
use tokio::net::TcpListener;

static ADDRESS: &str = "127.0.0.1:8080"; // !TODO move to global file that gets the value from environment variable.

#[tokio::main]
async fn main() -> io::Result<()> {
    // Build application with routes
    let app = Router::new()
        // GET /
        .route("/", get(root));

    // Run the app
    let listener = TcpListener::bind(ADDRESS).await?;
    serve(listener, app).await?;

    Ok(())
}

// Derive from serde desirialize.
#[derive(Deserialize)]
struct Test {
    #[serde(default, deserialize_with = "empty_string_as_none")] // Handles empty or non existing query parameters.
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

async fn root(Query(test): Query<Test>) -> Html<&'static str> {
    println!("{:?}", test);
    Html("<h1>Hello World!</h1>")
}

use std::{
    env,
    error::Error,
    sync::{Arc, OnceLock},
};

use axum::{http::HeaderValue, serve, Router};
use email_verification::email_verification;
use leprecon::{
    auth::{get_valid_jwt, request::fetch_jwks, Keys, JWT},
    header::htmx_headers,
    signals::shutdown_signal,
    utils::configure_tracing,
};
use reqwest::Method;
use tokio::{net::TcpListener, sync::Mutex};
use tokio_postgres::NoTls;
use tower::ServiceBuilder;
use tower_http::cors::CorsLayer;
use user::{create_user, user_balance};

mod email_verification;
mod embedded;
mod model;
mod user;

// Host variables
static HOST: OnceLock<String> = OnceLock::new();
static LOG_LEVEL: OnceLock<String> = OnceLock::new();
static ALLOW_ORIGIN: OnceLock<String> = OnceLock::new();

// DB variables
static ACCOUNT_CONN: OnceLock<String> = OnceLock::new();

// Auth variables
static AUTH_HOST: OnceLock<String> = OnceLock::new();
static CLIENT_ID: OnceLock<String> = OnceLock::new();
static CLIENT_SECRET: OnceLock<String> = OnceLock::new();
static CLIENT_AUD: OnceLock<String> = OnceLock::new();
static AUTH_KEYS: OnceLock<Keys> = OnceLock::new();

// VALKEY variables
static VALKEY_CONN: OnceLock<String> = OnceLock::new();

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    // Http client
    let req_client: reqwest::Client = reqwest::Client::new();

    // Initialize env variables
    init_env(&req_client).await;

    // Configure logging
    configure_tracing(LOG_LEVEL.get().unwrap());

    // DB client
    let (mut db_client, connection) =
        tokio_postgres::connect(ACCOUNT_CONN.get().unwrap(), NoTls).await?;

    tokio::spawn(async move {
        if let Err(e) = connection.await {
            panic!("Connection error: {}", e);
        }
    });

    // Run migrations
    embedded::migrations::runner()
        .run_async(&mut db_client)
        .await?;

    // Get valid access token
    let client: redis::Client = redis::Client::open(VALKEY_CONN.get().unwrap().as_str())?;
    let mut con: redis::aio::MultiplexedConnection =
        client.get_multiplexed_async_connection().await?;

    let jwt: Arc<Mutex<JWT>> = Arc::new(Mutex::new(
        get_valid_jwt(
            &mut con,
            &req_client,
            AUTH_HOST.get().unwrap(),
            CLIENT_ID.get().unwrap(),
            CLIENT_SECRET.get().unwrap(),
        )
        .await?,
    ));

    // Build application and listen to incoming requests.
    let app: Router = build_app(Arc::clone(&jwt));
    let listener: TcpListener = TcpListener::bind(HOST.get().unwrap()).await?;

    // Run the app.
    serve(listener, app)
        .with_graceful_shutdown(shutdown_signal())
        .await?;

    Ok(())
}

// Initialize env variables
async fn init_env(req_client: &reqwest::Client) {
    HOST.get_or_init(|| env::var("HOST").unwrap());
    LOG_LEVEL.get_or_init(|| env::var("LOG_LEVEL").unwrap());
    ALLOW_ORIGIN.get_or_init(|| env::var("ALLOW_ORIGIN").unwrap());

    ACCOUNT_CONN.get_or_init(|| env::var("ACCOUNT_CONN").unwrap());

    AUTH_HOST.get_or_init(|| env::var("AUTH_HOST").unwrap());
    CLIENT_ID.get_or_init(|| env::var("CLIENT_ID").unwrap());
    CLIENT_SECRET.get_or_init(|| env::var("CLIENT_SECRET").unwrap());
    CLIENT_AUD.get_or_init(|| env::var("CLIENT_AUD").unwrap());

    let keys: Keys = match fetch_jwks(req_client, AUTH_HOST.get().unwrap()).await {
        Ok(v) => v,
        Err(e) => panic!("Cannot fetch jwks: {:?}", e),
    };

    AUTH_KEYS.get_or_init(|| keys);

    VALKEY_CONN.get_or_init(|| env::var("VALKEY_CONN").unwrap());
}

/// Builds the application.
fn build_app(state: Arc<Mutex<JWT>>) -> Router {
    Router::new()
        .route(
            "/account/email/verification",
            axum::routing::post(email_verification),
        )
        .route("/account/user/balance", axum::routing::get(user_balance))
        .route("/account/user", axum::routing::post(create_user))
        .with_state(state)
        .layer(
            // Axum recommends to use tower::ServiceBuilder to apply multiple middleware at once, instead of repeatadly calling layer.
            // https://docs.rs/axum/latest/axum/middleware/index.html#applying-multiple-middleware
            ServiceBuilder::new().layer(
                CorsLayer::new()
                    .allow_methods([Method::GET])
                    .allow_origin(HeaderValue::from_static(ALLOW_ORIGIN.get().unwrap()))
                    .allow_headers(htmx_headers()),
            ),
        )
}

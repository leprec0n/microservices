use std::{collections::HashMap, io, str::FromStr, sync::OnceLock};

use axum::{
    extract::State,
    http::{HeaderValue, Method, StatusCode},
    response::Html,
    routing::post,
    serve, Form, Router,
};
use leprecon::{
    auth::{
        self, create_certificate, decode_token, send_email_verification, token_from_auth_provider,
        valid_jwt_from_db,
    },
    db::generate_db_conn,
    headers::htmx_headers,
    signals::shutdown_signal,
    utils,
};
use tokio::net::TcpListener;
use tokio_postgres::NoTls;
use tower::ServiceBuilder;
use tower_http::cors::CorsLayer;
use tracing::{debug, warn, Level};
use tracing_subscriber::{
    fmt::writer::MakeWriterExt, layer::SubscriberExt, util::SubscriberInitExt,
};

// Host variables
static HOST: OnceLock<String> = OnceLock::new();
static LOG_LEVEL: OnceLock<String> = OnceLock::new();

// DB variables
static DB_HOST: OnceLock<String> = OnceLock::new();
static DB_NAME: OnceLock<String> = OnceLock::new();
static DB_USER: OnceLock<String> = OnceLock::new();
static DB_PASSWORD: OnceLock<String> = OnceLock::new();

// Auth variables
static AUTH_HOST: OnceLock<String> = OnceLock::new();
static CLIENT_ID: OnceLock<String> = OnceLock::new();
static CLIENT_SECRET: OnceLock<String> = OnceLock::new();
static CLIENT_AUD: OnceLock<String> = OnceLock::new();

static AUTH_CERT: OnceLock<String> = OnceLock::new(); // !TODO Fetch at beginning https://doc.rust-lang.org/std/sync/struct.OnceLock.html

#[tokio::main]
async fn main() -> io::Result<()> {
    // Initialize env variables
    init_env();

    // Configure tracing
    configure_tracing();

    // Get valid access token
    let db_params: HashMap<&str, &String> = HashMap::from([
        ("host", DB_HOST.get().unwrap()),
        ("db", DB_NAME.get().unwrap()),
        ("user", DB_USER.get().unwrap()),
        ("password", DB_PASSWORD.get().unwrap()),
    ]);

    let (client, connection) =
        match tokio_postgres::connect(&generate_db_conn(&db_params), NoTls).await {
            Ok(v) => v,
            Err(e) => panic!("{:?}", e),
        };

    tokio::spawn(async move {
        if let Err(e) = connection.await {
            warn!("Connection error: {}", e);
        }
    });

    let jwt: auth::JWT = match valid_jwt_from_db(&client).await {
        Some(v) => v,
        None => {
            token_from_auth_provider(
                &client,
                AUTH_HOST.get().unwrap(),
                CLIENT_ID.get().unwrap(),
                CLIENT_SECRET.get().unwrap(),
            )
            .await
        }
    };

    // Build application and listen to incoming requests.
    let app: Router = build_app(jwt);
    let listener: TcpListener = TcpListener::bind(HOST.get().unwrap()).await?;

    // Run the app.
    serve(listener, app)
        .with_graceful_shutdown(shutdown_signal())
        .await?;

    Ok(())
}

// Initialize env variables
fn init_env() {
    HOST.get_or_init(|| utils::get_env_var("HOST"));
    LOG_LEVEL.get_or_init(|| utils::get_env_var("LOG_LEVEL"));

    DB_HOST.get_or_init(|| utils::get_env_var("DB_HOST"));
    DB_NAME.get_or_init(|| utils::get_env_var("DB_NAME"));
    DB_USER.get_or_init(|| utils::get_env_var("DB_USER"));
    DB_PASSWORD.get_or_init(|| utils::get_env_var("DB_PASSWORD"));

    AUTH_HOST.get_or_init(|| utils::get_env_var("AUTH_HOST"));
    CLIENT_ID.get_or_init(|| utils::get_env_var("CLIENT_ID"));
    CLIENT_SECRET.get_or_init(|| utils::get_env_var("CLIENT_SECRET"));
    CLIENT_AUD.get_or_init(|| utils::get_env_var("CLIENT_AUD"));

    AUTH_CERT.get_or_init(|| utils::get_env_var("AUTH_CERT"));
}

/// Configure tracing with tracing_subscriber.
fn configure_tracing() {
    tracing_subscriber::registry()
        .with(tracing_subscriber::fmt::layer().with_writer(
            std::io::stdout.with_max_level(Level::from_str(LOG_LEVEL.get().unwrap()).unwrap()),
        ))
        .init();
}

/// Builds the application.
fn build_app(state: auth::JWT) -> Router {
    Router::new()
        .route("/account/email/verification", post(email_verification))
        .with_state(state)
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

async fn email_verification(
    State(state): State<auth::JWT>,
    Form(params): Form<HashMap<String, String>>,
) -> (StatusCode, Html<&'static str>) {
    // Get id token param
    let id_token: &String = match params.get("id_token") {
        Some(v) => v,
        None => {
            return (
                StatusCode::BAD_REQUEST,
                Html("<span>No parameter id_token</span>"),
            )
        }
    };

    // Decode token
    // !TODO Fetch cert from jwks endpoint
    let claims: auth::Claims = match decode_token(
        create_certificate(AUTH_CERT.get().unwrap()),
        CLIENT_AUD.get().unwrap(),
        id_token,
    ) {
        Ok(v) => v.claims,
        Err(e) => {
            warn!("Cannot decode id token: {:?}", e);
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Html("<span>Cannot decode id_token</span>"),
            );
        }
    };

    // Already verified token
    if claims.email_verified {
        return (StatusCode::OK, Html("<span>Already verified email</span>"));
    }

    // Check if verification email already send

    // Send verification email
    let response = match send_email_verification(
        claims,
        CLIENT_ID.get().unwrap(),
        AUTH_HOST.get().unwrap(),
        &state.access_token,
    )
    .await
    {
        Ok(v) => v,
        Err(e) => {
            warn!("{:?}", e);
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Html("<span>Cannot get text from response</span>"),
            );
        }
    };

    if response.status() != StatusCode::CREATED {
        return (
            StatusCode::INTERNAL_SERVER_ERROR,
            Html("<span>Cannot send verification email</span>"),
        );
    }

    return (StatusCode::OK, Html("<span>Succesfully send email</span>"));
}

// CHANGES TO BE MADE:
// - Limit email sending per user (via cache in gateway?)(daily?)

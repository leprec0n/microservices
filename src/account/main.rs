use std::{collections::HashMap, error::Error, sync::OnceLock};

use axum::{
    extract::State,
    http::{HeaderValue, Method, StatusCode},
    response::Html,
    routing::post,
    serve, Form, Router,
};
use email_verification::{
    db::{create_verification_session, verification_already_send},
    request::send_email_verification,
};
use leprecon::{
    auth::{self, create_certificate, decode_token, get_valid_jwt, request::fetch_jwks, Keys, JWT},
    header::htmx_headers,
    signals::shutdown_signal,
    utils::{self, configure_tracing},
};
use tokio::net::TcpListener;
use tokio_postgres::NoTls;
use tower::ServiceBuilder;
use tower_http::cors::CorsLayer;
use tracing::{error, warn};

mod email_verification;

// Host variables
static HOST: OnceLock<String> = OnceLock::new();
static LOG_LEVEL: OnceLock<String> = OnceLock::new();

// DB variables
static SESSION_CONN: OnceLock<String> = OnceLock::new();

// Auth variables
static AUTH_HOST: OnceLock<String> = OnceLock::new();
static CLIENT_ID: OnceLock<String> = OnceLock::new();
static CLIENT_SECRET: OnceLock<String> = OnceLock::new();
static CLIENT_AUD: OnceLock<String> = OnceLock::new();
static AUTH_KEYS: OnceLock<Keys> = OnceLock::new();

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    // Http client
    let req_client: reqwest::Client = reqwest::Client::new();

    // Initialize env variables
    init_env(&req_client).await;

    // Configure logging
    configure_tracing(LOG_LEVEL.get().unwrap());

    // DB client
    let (db_client, connection) =
        tokio_postgres::connect(SESSION_CONN.get().unwrap(), NoTls).await?;

    tokio::spawn(async move {
        if let Err(e) = connection.await {
            warn!("Connection error: {}", e);
        }
    });

    // Get valid access token
    let jwt: JWT = get_valid_jwt(
        &db_client,
        &req_client,
        AUTH_HOST.get().unwrap(),
        CLIENT_ID.get().unwrap(),
        CLIENT_SECRET.get().unwrap(),
    )
    .await?;

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
async fn init_env(req_client: &reqwest::Client) {
    HOST.get_or_init(|| utils::get_env_var("HOST"));
    LOG_LEVEL.get_or_init(|| utils::get_env_var("LOG_LEVEL"));

    SESSION_CONN.get_or_init(|| utils::get_env_var("SESSION_CONN"));

    AUTH_HOST.get_or_init(|| utils::get_env_var("AUTH_HOST"));
    CLIENT_ID.get_or_init(|| utils::get_env_var("CLIENT_ID"));
    CLIENT_SECRET.get_or_init(|| utils::get_env_var("CLIENT_SECRET"));
    CLIENT_AUD.get_or_init(|| utils::get_env_var("CLIENT_AUD"));

    let keys: Keys = match fetch_jwks(req_client, AUTH_HOST.get().unwrap()).await {
        Ok(v) => v,
        Err(e) => panic!("Cannot fetch jwks: {:?}", e),
    };

    AUTH_KEYS.get_or_init(|| keys);
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
    let claims: auth::Claims = match decode_token(
        create_certificate(&AUTH_KEYS.get().unwrap().keys[0].x5c[0]), // Might not work if certificate is in different position of key
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
        return (
            StatusCode::FORBIDDEN,
            Html("<span>Already verified email</span>"),
        );
    }

    // !TODO Move to state? Only make 1 - x clients
    let (db_client, connection) =
        match tokio_postgres::connect(SESSION_CONN.get().unwrap(), NoTls).await {
            Ok(v) => v,
            Err(e) => panic!("{:?}", e),
        };

    tokio::spawn(async move {
        if let Err(e) = connection.await {
            warn!("Connection error: {}", e);
        }
    });

    if verification_already_send(&db_client, &claims.email).await {
        return (
            StatusCode::TOO_MANY_REQUESTS,
            Html("<span>Already send email</span>"),
        );
    };

    // Send verification email
    let response = match send_email_verification(
        &claims,
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

    if let Err(e) = create_verification_session(&db_client, &claims.email).await {
        error!("Cannot create verification session: {:?}", e)
    }

    (StatusCode::OK, Html("<span>Succesfully send email</span>"))
}

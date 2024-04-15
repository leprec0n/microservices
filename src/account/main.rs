use std::{collections::HashMap, io, str::FromStr, sync::OnceLock};

use axum::{
    extract::State,
    http::{HeaderValue, Method, StatusCode},
    response::Html,
    routing::post,
    serve, Form, Router,
};
use jsonwebtoken::{decode, Algorithm, DecodingKey, TokenData, Validation};
use leprecon::{auth, headers::htmx_headers, signals::shutdown_signal, utils};
use serde::Deserialize;
use tokio::net::TcpListener;
use tokio_postgres::{connect, Client, NoTls};
use tower::ServiceBuilder;
use tower_http::cors::CorsLayer;
use tracing::{debug, info, warn, Level};
use tracing_subscriber::{
    fmt::writer::MakeWriterExt, layer::SubscriberExt, util::SubscriberInitExt,
};

// Host variables
static HOST: OnceLock<String> = OnceLock::new();
static LOG_LEVEL: OnceLock<String> = OnceLock::new();

// DB variables
static DB_HOST: OnceLock<String> = OnceLock::new();
static DB_USER: OnceLock<String> = OnceLock::new();
static DB_NAME: OnceLock<String> = OnceLock::new();
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

    let token = get_auth_token().await;

    // Build application and listen to incoming requests.
    let app: Router = build_app(token);
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
    DB_USER.get_or_init(|| utils::get_env_var("DB_USER"));
    DB_NAME.get_or_init(|| utils::get_env_var("DB_NAME"));
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
fn build_app(state: auth::Token) -> Router {
    Router::new()
        .route("/account/email/verification", post(send_email_verification))
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

async fn get_auth_token() -> auth::Token {
    // Connection variables
    let db_connect = &format!(
        "host={db_host} user={db_user} dbname={db_name} password={db_password}",
        db_host = DB_HOST.get().unwrap(),
        db_user = DB_USER.get().unwrap(),
        db_name = DB_NAME.get().unwrap(),
        db_password = DB_PASSWORD.get().unwrap(),
    );

    // Connect to database
    let (client, connection) = match connect(db_connect, NoTls).await {
        Ok(v) => v,
        Err(e) => panic!("{:?}", e),
    };

    // Listen for requests
    tokio::spawn(async move {
        if let Err(e) = connection.await {
            warn!("Connection error: {}", e);
        }
    });

    if let Some(token) = valid_token_from_db(&client).await {
        return token;
    }

    // Send request to auth0
    let client_reqwest = reqwest::Client::new();
    let mut headers = reqwest::header::HeaderMap::new();
    let content_type: HeaderValue = "application/x-www-form-urlencoded".parse().unwrap();
    headers.insert("Content-Type", content_type);

    let auth_host = AUTH_HOST.get().unwrap();
    let token_url = format!("{}/oauth/token", auth_host);
    let audience = format!("{}/api/v2/", auth_host);

    let mut params = HashMap::new();
    params.insert("grant_type", "client_credentials");
    params.insert("client_id", CLIENT_ID.get().unwrap());
    params.insert("client_secret", CLIENT_SECRET.get().unwrap());
    params.insert("audience", &audience);

    let response = match client_reqwest.post(token_url).form(&params).send().await {
        Ok(v) => match v.text().await {
            Ok(v) => v,
            Err(e) => panic!("{:?}", e), // !TODO In function return result
        },
        Err(e) => panic!("{:?}", e),
    };

    info!(response);
    let token = match serde_json::from_str(&response) {
        Ok(v) => v,
        Err(e) => panic!("{:?}", e),
    };

    store_access_token(&client, &token).await;

    token
}

async fn store_access_token(client: &Client, token: &auth::Token) {
    if token_exists(client, token).await {
        return;
    }

    match client
        .query(
            "INSERT INTO account(access_token, expires, scope, token_type) VALUES($1, $2, $3, $4)",
            &[
                &token.access_token,
                &token.expires_in,
                &token.scope,
                &token.token_type,
            ],
        )
        .await
    {
        Ok(_) => println!("Successfully inserted token."), // !TODO Log insert
        Err(e) => panic!("{:?}", e),                       // !TODO Log error
    };
}

async fn valid_token_from_db(client: &Client) -> Option<auth::Token> {
    let res = match client
        .query_one(
            "SELECT * FROM account WHERE expires > now() ORDER BY expires DESC LIMIT 1",
            &[],
        )
        .await
    {
        Ok(v) => v,
        Err(e) => {
            warn!("Cannot execute statement: {:?}", e);
            return None;
        }
    };

    Some(auth::Token {
        access_token: res.get("access_token"),
        expires_in: res.get("expires"),
        scope: res.get("scope"),
        token_type: res.get("token_type"),
    })
}

async fn token_exists(client: &Client, token: &auth::Token) -> bool {
    let res = match client
        .query(
            "SELECT * FROM account WHERE access_token=$1", // INSERT INTO account(access_token) VALUES($1)
            &[&token.access_token],
        )
        .await
    {
        Ok(v) => v,
        Err(e) => panic!("{:?}", e), // !TODO Log error
    };

    if !res.is_empty() {
        debug!("Token already exists!"); // !TODO Log token exists
        return true;
    }

    false
}

#[derive(Debug, Deserialize)]
#[allow(dead_code)]
struct Claims {
    aud: String,
    email: String,
    email_verified: bool,
    exp: u32,
    iat: u32,
    iss: String,
    name: String,
    nickname: String,
    picture: String,
    sid: String,
    sub: String,
    updated_at: String,
}

async fn send_email_verification(
    State(state): State<auth::Token>,
    Form(params): Form<HashMap<String, String>>,
) -> (StatusCode, Html<&'static str>) {
    // !TODO Extract to functions
    // Get id token
    let token = match params.get("id_token") {
        Some(v) => v,
        None => {
            return (
                StatusCode::BAD_REQUEST,
                Html("<span>No parameter id_token</span>"),
            )
        }
    };

    // !TODO Move and get from endpoint
    // Create cert
    let mut cert_str: String = "-----BEGIN CERTIFICATE-----\n".to_owned();
    cert_str.push_str(&AUTH_CERT.get().unwrap());
    cert_str.push_str("\n-----END CERTIFICATE-----");

    // Create key from pem
    let key = match DecodingKey::from_rsa_pem(cert_str.as_bytes()) {
        Ok(v) => v,
        Err(e) => {
            warn!("Cannot decode key: {e}");
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Html("<span>Cannot decode id_token</span>"),
            );
        }
    };

    let mut validation = Validation::new(Algorithm::RS256);
    validation.set_audience(&[CLIENT_AUD.get().unwrap()]);

    // Decode token
    let token_message: TokenData<Claims> = match decode(token, &key, &validation) {
        Ok(v) => v,
        Err(e) => {
            warn!("Cannot extract claim from token: {e}");
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Html("<span>Cannot extract claim</span>"),
            );
        }
    };

    if token_message.claims.email_verified {
        return (StatusCode::OK, Html("<span>Already verified email</span>"));
    }

    // Prep request
    let client = reqwest::Client::new();

    let mut headers = reqwest::header::HeaderMap::new();
    let content_type: HeaderValue = match "application/json".parse() {
        Ok(v) => v,
        Err(e) => {
            warn!("{:?}", e);
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Html("<span>Invalid header value</span>"),
            );
        }
    };

    headers.insert("Content-Type", content_type.clone());
    headers.insert("Accept", content_type);

    let mut map: HashMap<&str, &str> = HashMap::new();
    map.insert("user_id", &token_message.claims.sub);
    map.insert("client_id", CLIENT_ID.get().unwrap());

    // Send request
    let response = match client
        .post(AUTH_HOST.get().unwrap().to_owned() + "/api/v2/jobs/verification-email")
        .json(&map)
        .bearer_auth(state)
        .send()
        .await
    {
        Ok(v) => match v.text().await {
            Ok(v) => v,
            Err(e) => {
                warn!("{:?}", e);
                return (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    Html("<span>Cannot get text from response</span>"),
                );
            }
        },
        Err(e) => {
            warn!("{:?}", e);
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Html("<span>Request unsuccesfull</span>"),
            );
        }
    };

    debug!("{}", response);

    (StatusCode::OK, Html("<span>Verification email send</span>"))
}

// CHANGES TO BE MADE:
// - Limit email sending per user (via cache in gateway?)(daily?)
// - Move env variables to top as global static

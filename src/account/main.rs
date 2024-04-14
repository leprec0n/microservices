use std::{
    collections::HashMap,
    env,
    fmt::{self, Display, Formatter},
    io,
};

use axum::{
    extract::State,
    http::{HeaderValue, Method, StatusCode},
    response::Html,
    routing::post,
    serve, Form, Router,
};
use chrono::{DateTime, Duration, Local};
use jsonwebtoken::{decode, Algorithm, DecodingKey, TokenData, Validation};
use leprecon::{headers::htmx_headers, signals::shutdown_signal};
use serde::{Deserialize, Deserializer};
use tokio::net::TcpListener;
use tokio_postgres::{connect, Client, NoTls};
use tower::ServiceBuilder;
use tower_http::cors::CorsLayer;
use tracing::{debug, warn, Level};
use tracing_subscriber::{
    fmt::writer::MakeWriterExt, layer::SubscriberExt, util::SubscriberInitExt,
};

static ADDRESS: &str = "127.0.0.1:8081"; // !TODO move to global file that gets the value from environment variable.

#[derive(Clone, Debug, Deserialize)]
struct Token {
    access_token: String,
    scope: String,
    #[serde(deserialize_with = "deserialize_expires_in")]
    expires_in: DateTime<Local>,
    token_type: String,
}

fn deserialize_expires_in<'de, D>(deserializer: D) -> Result<DateTime<Local>, D::Error>
where
    D: Deserializer<'de>,
{
    let expires_in = i64::deserialize(deserializer)?;

    Ok(Local::now() + Duration::seconds(expires_in))
}

impl Display for Token {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        write!(
            f,
            "access_token: {}, scope: {}, expires_in: {}, token_type: {}",
            self.access_token, self.scope, self.expires_in, self.token_type
        )
    }
}

#[tokio::main]
async fn main() -> io::Result<()> {
    // Configure tracing
    configure_tracing();

    let env_variables = set_env_variables();
    let token = get_auth_token(&env_variables).await;

    let state = (token, env_variables);

    // Build application and listen to incoming requests.
    let app: Router = build_app(state);
    let listener: TcpListener = TcpListener::bind(ADDRESS).await?;

    // Run the app.
    serve(listener, app)
        .with_graceful_shutdown(shutdown_signal())
        .await?;

    Ok(())
}

fn set_env_variables() -> HashMap<String, String> {
    let mut map: HashMap<String, String> = HashMap::new();
    let auth_host = match env::var_os("AUTH_HOST") {
        Some(v) => v.into_string().unwrap(),
        None => panic!("AUTH_HOST not set!"),
    };

    let client_id = match env::var_os("CLIENT_ID") {
        Some(v) => v.into_string().unwrap(),
        None => panic!("CLIENT_ID not set!"),
    };

    let client_secret = match env::var_os("CLIENT_SECRET") {
        Some(v) => v.into_string().unwrap(),
        None => panic!("CLIENT_SECRET not set!"),
    };

    let db_host = match env::var_os("DB_HOST") {
        Some(v) => v.into_string().unwrap(),
        None => panic!("DB_HOST not set!"),
    };

    let db_user = match env::var_os("DB_USER") {
        Some(v) => v.into_string().unwrap(),
        None => panic!("DB_USER not set!"),
    };

    let db_password = match env::var_os("DB_PASSWORD") {
        Some(v) => v.into_string().unwrap(),
        None => panic!("DB_PASSWORD not set!"),
    };

    let db_name = match env::var_os("DB_NAME") {
        Some(v) => v.into_string().unwrap(),
        None => panic!("DB_NAME not set!"),
    };

    map.insert(String::from("auth_host"), auth_host.clone());
    map.insert(String::from("client_id"), client_id);
    map.insert(String::from("client_secret"), client_secret);
    map.insert(
        String::from("grant_type"),
        String::from("client_credentials"),
    );
    map.insert(String::from("audience"), auth_host + "/api/v2/");
    map.insert(String::from("db_host"), db_host);
    map.insert(String::from("db_user"), db_user);
    map.insert(String::from("db_password"), db_password);
    map.insert(String::from("db_name"), db_name);

    map
}

/// Configure tracing with tracing_subscriber.
fn configure_tracing() {
    tracing_subscriber::registry()
        .with(
            tracing_subscriber::fmt::layer()
                .with_writer(std::io::stdout.with_max_level(Level::DEBUG)), // !TODO Replace with env variable
        )
        .init();
}

/// Builds the application.
fn build_app(state: (Token, HashMap<String, String>)) -> Router {
    Router::new()
        .route("/email/verification", post(send_email_verification))
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

async fn get_auth_token(env_variables: &HashMap<String, String>) -> Token {
    let db_host = match env_variables.get("db_host") {
        Some(v) => v.to_owned(),
        None => todo!(),
    };

    let db_user = match env_variables.get("db_user") {
        Some(v) => v.to_owned(),
        None => todo!(),
    };

    let db_password = match env_variables.get("db_password") {
        Some(v) => v.to_owned(),
        None => todo!(),
    };

    let db_name = match env_variables.get("db_name") {
        Some(v) => v.to_owned(),
        None => todo!(),
    };

    let db_connect =
        &format!("host={db_host} user={db_user} password={db_password} dbname={db_name}");

    let (client, connection) = match connect(db_connect, NoTls).await {
        Ok(v) => v,
        Err(e) => panic!("{:?}", e),
    };

    tokio::spawn(async move {
        if let Err(e) = connection.await {
            eprintln!("connection error: {}", e);
        }
    });

    if let Some(token) = get_valid_token(&client).await {
        return token;
    }

    let client_reqwest = reqwest::Client::new();
    let mut headers = reqwest::header::HeaderMap::new();
    let content_type: HeaderValue = match "application/x-www-form-urlencoded".parse() {
        Ok(v) => v,
        Err(e) => {
            panic!("{:?}", e); // !TODO Log error
        }
    };
    headers.insert("Content-Type", content_type);

    let auth_host = match env_variables.get("auth_host") {
        Some(v) => v.to_owned(),
        None => todo!(),
    };

    let response = match client_reqwest
        .post(auth_host + "/oauth/token")
        .form(&env_variables)
        .send()
        .await
    {
        Ok(v) => match v.text().await {
            Ok(v) => v,
            Err(e) => panic!("{:?}", e),
        },
        Err(e) => panic!("{:?}", e),
    };

    let token = match serde_json::from_str(&response) {
        Ok(v) => v,
        Err(e) => panic!("{:?}", e),
    };

    store_access_token(&client, &token).await;

    token
}

async fn store_access_token(client: &Client, token: &Token) {
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

async fn get_valid_token(client: &Client) -> Option<Token> {
    let res = match client
        .query_one(
            "SELECT * FROM account WHERE expires > now() ORDER BY expires DESC LIMIT 1",
            &[],
        )
        .await
    {
        Ok(v) => v,
        Err(e) => {
            println!("{:?}", e); // !TODO Log error

            return None;
        }
    };

    let token = Token {
        access_token: res.get(1),
        expires_in: res.get(2),
        scope: res.get(3),
        token_type: res.get(4),
    };

    Some(token)
}

async fn token_exists(client: &Client, token: &Token) -> bool {
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
    State(state): State<(Token, HashMap<String, String>)>,
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
    let cert = env::var_os("AUTH_CERT").unwrap().into_string().unwrap();
    let mut cert_str: String = "-----BEGIN CERTIFICATE-----\n".to_owned();
    cert_str.push_str(&cert);
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
    let client_aud = env::var_os("CLIENT_AUD").unwrap().into_string().unwrap();
    validation.set_audience(&[client_aud]);

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

    let client_id = match state.1.get("client_id") {
        Some(v) => v,
        None => {
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Html("<span>Client id not found in state</span>"),
            );
        }
    };

    let mut map = HashMap::new();
    map.insert("user_id", token_message.claims.sub);
    map.insert("client_id", client_id.to_owned());

    let auth_host = match state.1.get("auth_host") {
        Some(v) => v,
        None => {
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Html("<span>Auth host variable not found in state</span>"),
            );
        }
    };

    // Send request
    let response = match client
        .post(auth_host.to_owned() + "/api/v2/jobs/verification-email")
        .json(&map)
        .bearer_auth(state.0.access_token)
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

use std::{
    collections::HashMap,
    env,
    fmt::{self, Display, Formatter},
    io,
};

use axum::{
    extract::State,
    http::{HeaderValue, Method, StatusCode},
    routing::post,
    serve, Form, Router,
};
use chrono::{DateTime, Duration, Local};
use leprecon::{headers::htmx_headers, signals::shutdown_signal};
use serde::{Deserialize, Deserializer};
use tokio::net::TcpListener;
use tokio_postgres::{connect, Client, NoTls};
use tower::ServiceBuilder;
use tower_http::cors::CorsLayer;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

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

    map.insert(String::from("auth_host"), auth_host.clone());
    map.insert(String::from("client_id"), client_id);
    map.insert(String::from("client_secret"), client_secret);
    map.insert(
        String::from("grant_type"),
        String::from("client_credentials"),
    );
    map.insert(
        String::from("audience"),
        String::from(auth_host + "/api/v2/"),
    );

    return map;
}

/// Configure tracing with tracing_subscriber.
fn configure_tracing() {
    tracing_subscriber::registry()
        .with(tracing_subscriber::fmt::layer())
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
    let (client, connection) = match connect(
        "", // !TODO Set connection string through env variables (find in .env)
        NoTls,
    )
    .await
    {
        Ok(v) => v,
        Err(e) => panic!("{:?}", e), // !TODO Log error
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

    let auth_host = match env_variables.clone().remove("auth_host") {
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
    if token_exists(&client, &token).await {
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
        .query_one("SELECT * FROM account ORDER BY expires DESC LIMIT 1", &[])
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

    return Some(token);
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

    if res.len() >= 1 {
        println!("Token already exists!"); // !TODO Log token exists
        return true;
    }

    return false;
}

#[derive(Debug, Deserialize)]
struct User {
    id: String,
}

impl Display for User {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        write!(f, "ID: {}", self.id)
    }
}

async fn send_email_verification(
    State(state): State<(Token, HashMap<String, String>)>,
    Form(user): Form<User>,
) -> StatusCode {
    println!("{:?}", state);
    let client = reqwest::Client::new();

    let mut headers = reqwest::header::HeaderMap::new();
    let content_type: HeaderValue = match "application/json".parse() {
        Ok(v) => v,
        Err(e) => {
            println!("{:?}", e); // !TODO Log error
            return StatusCode::INTERNAL_SERVER_ERROR;
        }
    };

    headers.insert("Content-Type", content_type.clone());
    headers.insert("Accept", content_type);

    let client_id = match state.1.get("client_id") {
        Some(v) => v,
        None => {
            println!("No CLIENT_ID env variable"); // !TODO Log no auth host env variable
            return StatusCode::INTERNAL_SERVER_ERROR;
        }
    };

    let mut map = HashMap::new();
    map.insert("user_id", user.id);
    map.insert("client_id", client_id.to_owned());

    let auth_host = match state.1.get("auth_host") {
        Some(v) => v,
        None => {
            println!("No AUTH_HOST env variable"); // !TODO Log no auth host env variable
            return StatusCode::INTERNAL_SERVER_ERROR;
        }
    };

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
                println!("{:?}", e); // !TODO Log error
                return StatusCode::INTERNAL_SERVER_ERROR;
            }
        },
        Err(e) => {
            println!("{:?}", e); // !TODO Log error
            return StatusCode::INTERNAL_SERVER_ERROR;
        }
    };

    println!("{}", response);

    StatusCode::OK
}

// YOU GENIUS IT WORKING, CHECK EMAIL SWIFTY
// CHANGES TO BE MADE:
// - Get bearer token
// - Get client id from front-end
// - Limit email sending per user (in database)(daily?)
// - Set up database
// - Move env variables to top as global static

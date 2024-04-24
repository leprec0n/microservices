use std::{
    collections::HashMap,
    error::Error,
    sync::{Arc, OnceLock},
};

use askama::Template;
use axum::{
    extract::State,
    http::{HeaderValue, Method, StatusCode},
    response::Html,
    serve, Form, Router,
};
use email_verification::{
    db::{create_verification_session, verification_already_send},
    request::send_email_verification,
};
use leprecon::{
    auth::{self, extract_id_token, get_valid_jwt, request::fetch_jwks, Claims, Keys, JWT},
    header::htmx_headers,
    signals::shutdown_signal,
    template::{self, Snackbar},
    utils::{self, configure_tracing},
};
use tokio::{net::TcpListener, sync::Mutex};
use tokio_postgres::NoTls;
use tower::ServiceBuilder;
use tower_http::cors::CorsLayer;
use tracing::{debug, error, info, warn};
use user::{db::get_balance, User};

mod email_verification;
mod embedded;
mod model;
mod user;

// Host variables
static HOST: OnceLock<String> = OnceLock::new();
static LOG_LEVEL: OnceLock<String> = OnceLock::new();

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

    match embedded::migrations::runner()
        .run_async(&mut db_client)
        .await
    {
        Ok(v) => info!("Succesfull migration:\n{:?}", v),
        Err(e) => debug!("Could not finish migration:\n{:?}", e),
    };

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
    HOST.get_or_init(|| utils::get_env_var("HOST"));
    LOG_LEVEL.get_or_init(|| utils::get_env_var("LOG_LEVEL"));

    ACCOUNT_CONN.get_or_init(|| utils::get_env_var("ACCOUNT_CONN"));

    AUTH_HOST.get_or_init(|| utils::get_env_var("AUTH_HOST"));
    CLIENT_ID.get_or_init(|| utils::get_env_var("CLIENT_ID"));
    CLIENT_SECRET.get_or_init(|| utils::get_env_var("CLIENT_SECRET"));
    CLIENT_AUD.get_or_init(|| utils::get_env_var("CLIENT_AUD"));

    let keys: Keys = match fetch_jwks(req_client, AUTH_HOST.get().unwrap()).await {
        Ok(v) => v,
        Err(e) => panic!("Cannot fetch jwks: {:?}", e),
    };

    AUTH_KEYS.get_or_init(|| keys);

    VALKEY_CONN.get_or_init(|| utils::get_env_var("VALKEY_CONN"));
}

/// Builds the application.
fn build_app(state: Arc<Mutex<JWT>>) -> Router {
    Router::new()
        .route(
            "/account/email/verification",
            axum::routing::post(email_verification),
        )
        .route("/account/balance", axum::routing::get(user_balance))
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
    State(state): State<Arc<Mutex<auth::JWT>>>,
    Form(params): Form<HashMap<String, String>>,
) -> (StatusCode, Html<String>) {
    let mut snackbar: Snackbar<'_> = Snackbar {
        title: "Error",
        message: "",
        color: "red",
    };

    let claims: Claims = match extract_id_token(
        params,
        &mut snackbar,
        AUTH_KEYS.get().unwrap(),
        CLIENT_AUD.get().unwrap(),
    ) {
        Ok(v) => v,
        Err(e) => return e,
    };

    // Already verified token
    if claims.email_verified {
        snackbar.message = "Already verified email";
        return (StatusCode::FORBIDDEN, Html(snackbar.render().unwrap()));
    }

    // !TODO Move to state? Only make 1 - x clients
    let (db_client, connection) =
        match tokio_postgres::connect(ACCOUNT_CONN.get().unwrap(), NoTls).await {
            Ok(v) => v,
            Err(e) => panic!("{:?}", e),
        };

    tokio::spawn(async move {
        if let Err(e) = connection.await {
            warn!("Connection error: {}", e);
        }
    });

    if verification_already_send(&db_client, &claims.email).await {
        snackbar.message = "Already send email";
        return (
            StatusCode::TOO_MANY_REQUESTS,
            Html(snackbar.render().unwrap()),
        );
    };

    let mut lock = state.lock().await;
    let req_client = reqwest::Client::new();

    let client: redis::Client = redis::Client::open(VALKEY_CONN.get().unwrap().as_str()).unwrap();
    let mut con = client.get_multiplexed_async_connection().await.unwrap();

    *lock = match get_valid_jwt(
        &mut con,
        &req_client,
        AUTH_HOST.get().unwrap(),
        CLIENT_ID.get().unwrap(),
        CLIENT_SECRET.get().unwrap(),
    )
    .await
    {
        Ok(v) => v,
        Err(e) => {
            error!("Could not get valid jwt: {:?}", e);
            snackbar.message = "Could not process request";
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Html(snackbar.render().unwrap()),
            );
        }
    };

    // Send verification email
    let response = match send_email_verification(
        &req_client,
        &claims,
        CLIENT_ID.get().unwrap(),
        AUTH_HOST.get().unwrap(),
        &lock.access_token,
    )
    .await
    {
        Ok(v) => v,
        Err(e) => {
            warn!("Cannot process email request: {:?}", e);
            snackbar.message = "Could not process request";
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Html(snackbar.render().unwrap()),
            );
        }
    };

    if response.status() != StatusCode::CREATED {
        snackbar.message = "Could not process request";
        error!("Verification email not send");
        return (
            StatusCode::INTERNAL_SERVER_ERROR,
            Html(snackbar.render().unwrap()),
        );
    }

    if let Err(e) = create_verification_session(&db_client, &claims.email).await {
        error!("Cannot create verification session: {:?}", e)
    }

    snackbar.title = "Succes";
    snackbar.message = "Succesfully send email";
    snackbar.color = "green";
    (StatusCode::OK, Html(snackbar.render().unwrap()))
}

async fn user_balance(Form(params): Form<HashMap<String, String>>) -> (StatusCode, Html<String>) {
    let mut snackbar: Snackbar<'_> = Snackbar {
        title: "Error",
        message: "",
        color: "red",
    };

    // Get id token param
    let claims: Claims = match extract_id_token(
        params,
        &mut snackbar,
        AUTH_KEYS.get().unwrap(),
        CLIENT_AUD.get().unwrap(),
    ) {
        Ok(v) => v,
        Err(e) => return e,
    };

    // Get balance from email (result error if not in db)
    // !TODO Move to state? Only make 1 - x clients
    let (db_client, connection) =
        match tokio_postgres::connect(ACCOUNT_CONN.get().unwrap(), NoTls).await {
            Ok(v) => v,
            Err(e) => panic!("{:?}", e),
        };

    tokio::spawn(async move {
        if let Err(e) = connection.await {
            warn!("Connection error: {}", e);
        }
    });

    let bal: User = match get_balance(&claims.email, &db_client).await {
        Ok(v) => v,
        Err(e) => {
            error!("Could not fetch balance: {:?}", e);
            snackbar.message = "Could not get fetch balance!";
            return (StatusCode::BAD_REQUEST, Html(snackbar.render().unwrap()));
        }
    };

    let balance: template::Balance<'_> = template::Balance {
        amount: &bal.balance.to_string(),
        currency: &bal.currency.to_string(),
    };

    (StatusCode::OK, Html(balance.render().unwrap()))
}

mod model;

pub mod db;
use std::collections::HashMap;

use askama::Template;
use axum::{response::Html, Form};
use leprecon::{
    auth::{extract_id_token, Claims},
    template::{self, Snackbar},
};
pub use model::*;
use reqwest::StatusCode;
use tokio_postgres::NoTls;
use tracing::{debug, error, warn};

use crate::{ACCOUNT_CONN, AUTH_KEYS, CLIENT_AUD};

use self::db::{get_user, insert_user};

pub async fn user_information(
    Form(params): Form<HashMap<String, String>>,
) -> (StatusCode, Html<String>) {
    let mut snackbar: Snackbar<'_> = Snackbar {
        title: "Error",
        message: "",
        color: "red",
    };

    let sub: &String = match params.get("sub") {
        Some(v) => v,
        None => {
            snackbar.message = "Could not process request";
            return (StatusCode::BAD_GATEWAY, Html(snackbar.render().unwrap()));
        }
    };

    // !TODO Use connection pool
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

    let user: User = match get_user(&sub, &db_client).await {
        Ok(v) => v,
        Err(e) => {
            debug!("Could not get user: {:?}", e);
            snackbar.message = "Could not process request";
            return (StatusCode::BAD_GATEWAY, Html(snackbar.render().unwrap()));
        }
    };

    let user_template = template::User {
        sub: &user.sub,
        balance: &user.balance,
    };

    (StatusCode::OK, Html(user_template.render().unwrap()))
}

pub async fn create_user(Form(params): Form<HashMap<String, String>>) -> StatusCode {
    let sub = match params.get("sub") {
        Some(v) => v,
        None => return StatusCode::BAD_GATEWAY,
    };

    // !TODO Use connection pool
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

    if let Err(e) = insert_user(sub, &db_client).await {
        error!("Could not insert new user: {:?}", e);
        return StatusCode::INTERNAL_SERVER_ERROR;
    }

    StatusCode::OK
}

pub async fn user_balance(
    Form(params): Form<HashMap<String, String>>,
) -> (StatusCode, Html<String>) {
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

    let bal: User = match get_user(&claims.sub, &db_client).await {
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

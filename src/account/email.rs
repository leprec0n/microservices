use std::collections::HashMap;

use crate::{StateParams, ACCOUNT_CONN, AUTH_HOST, CLIENT_ID, CLIENT_SECRET};
use askama::Template;
use axum::{extract::State, response::Html, Form};
use leprecon::{
    auth::get_valid_jwt,
    template::Snackbar,
    utils::{extract::extract_redis_conn, RedisConn},
};
use reqwest::StatusCode;
use tokio_postgres::NoTls;
use tracing::{error, warn};

use self::{
    db::{create_verification_session, verification_already_send},
    request::send_email_verification,
};

pub mod db;
pub mod request;

pub async fn email_verification(
    State(state): State<StateParams>,
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

    let email_verified: &String = match params.get("email_verified") {
        Some(v) => v,
        None => {
            snackbar.message = "Could not process request";
            return (StatusCode::BAD_GATEWAY, Html(snackbar.render().unwrap()));
        }
    };

    // Already verified token
    if email_verified == "true" {
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

    if verification_already_send(&db_client, sub).await {
        snackbar.message = "Already send email";
        return (
            StatusCode::TOO_MANY_REQUESTS,
            Html(snackbar.render().unwrap()),
        );
    };

    let mut lock = state.0.lock().await;
    let req_client = reqwest::Client::new();

    let redis_conn: RedisConn = match extract_redis_conn(&state.3, &mut snackbar).await {
        Ok(v) => v,
        Err(e) => return e,
    };

    *lock = match get_valid_jwt(
        redis_conn,
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
    let response: reqwest::Response = match send_email_verification(
        &req_client,
        sub,
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

    if let Err(e) = create_verification_session(&db_client, sub).await {
        error!("Cannot create verification session: {:?}", e)
    }

    snackbar.title = "Succes";
    snackbar.message = "Succesfully send email";
    snackbar.color = "green";
    (StatusCode::OK, Html(snackbar.render().unwrap()))
}

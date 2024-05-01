mod model;
mod request;

pub(crate) mod db;

use self::{
    db::{create_verification_session, verification_already_send},
    model::EmailParams,
    request::send_email_verification,
};

use crate::{StateParams, AUTH_HOST, CLIENT_ID, CLIENT_SECRET};

use askama::Template;
use axum::{extract::State, response::Html, Form};
use leprecon::{
    auth::get_valid_jwt,
    template::Snackbar,
    utils::{extract::extract_conn_from_pool, PostgresConn, RedisConn},
};
use reqwest::StatusCode;
use tracing::error;

pub(super) async fn email_verification(
    State(state): State<StateParams>,
    Form(params): Form<EmailParams>,
) -> (StatusCode, Html<String>) {
    let mut snackbar: Snackbar<'_> = Snackbar::default();

    if params.sub.is_empty() {
        return (
            StatusCode::UNPROCESSABLE_ENTITY,
            Html(snackbar.render().unwrap()),
        );
    };

    if params.email_verified.is_empty() {
        return (
            StatusCode::UNPROCESSABLE_ENTITY,
            Html(snackbar.render().unwrap()),
        );
    };

    // Already verified token
    if params.email_verified == "true" {
        snackbar.message = "Already verified email";
        return (StatusCode::BAD_REQUEST, Html(snackbar.render().unwrap()));
    }

    let postgres_conn: PostgresConn = match extract_conn_from_pool(&state.2, &mut snackbar).await {
        Ok(v) => v,
        Err(e) => return e,
    };

    if verification_already_send(&postgres_conn, &params.sub).await {
        snackbar.message = "Already send email";
        return (StatusCode::BAD_REQUEST, Html(snackbar.render().unwrap()));
    };

    let mut lock: tokio::sync::MutexGuard<'_, leprecon::auth::JWT> = state.0.lock().await;
    let req_client: &reqwest::Client = &state.1;

    let redis_conn: RedisConn = match extract_conn_from_pool(&state.3, &mut snackbar).await {
        Ok(v) => v,
        Err(e) => return e,
    };

    snackbar.message = "Could not process request";

    *lock = match get_valid_jwt(
        redis_conn,
        req_client,
        AUTH_HOST.get().unwrap(),
        CLIENT_ID.get().unwrap(),
        CLIENT_SECRET.get().unwrap(),
    )
    .await
    {
        Ok(v) => v,
        Err(e) => {
            error!("Could not get valid jwt: {:?}", e);
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Html(snackbar.render().unwrap()),
            );
        }
    };

    // Send verification email
    let response: reqwest::Response = match send_email_verification(
        req_client,
        &params.sub,
        CLIENT_ID.get().unwrap(),
        AUTH_HOST.get().unwrap(),
        &lock.access_token,
    )
    .await
    {
        Ok(v) => v,
        Err(e) => {
            error!("Cannot process verification email request: {:?}", e);
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Html(snackbar.render().unwrap()),
            );
        }
    };

    if response.status() != StatusCode::CREATED {
        error!("Verification email not send");
        return (
            StatusCode::INTERNAL_SERVER_ERROR,
            Html(snackbar.render().unwrap()),
        );
    }

    if let Err(e) = create_verification_session(&postgres_conn, &params.sub).await {
        error!("Cannot create verification session: {:?}", e)
    }

    snackbar.title = "Succes";
    snackbar.message = "Succesfully send email";
    snackbar.color = "green";

    (StatusCode::OK, Html(snackbar.render().unwrap()))
}

#[cfg(test)]
mod test {
    use std::env;

    use axum::{
        body::{self, Body},
        http::Request,
    };
    use reqwest::{header, Method, StatusCode};
    use tower::ServiceExt;

    use crate::fixture::{initialize, seed_database};

    #[tokio::test]
    async fn test_no_params_provided() {
        let app = initialize().await;
        let response = app
            .oneshot(
                Request::builder()
                    .method(Method::POST)
                    .uri("/account/email/verification")
                    .header(header::CONTENT_TYPE, "application/x-www-form-urlencoded")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::UNPROCESSABLE_ENTITY);
        let bytes = body::to_bytes(response.into_body(), usize::MAX)
            .await
            .unwrap();
        let body_str = String::from_utf8(bytes.to_vec()).unwrap();
        assert!(body_str.contains("Could not process request"));
    }

    #[tokio::test]
    async fn test_already_verified() {
        let app = initialize().await;
        let params = format!("sub=123&email_verified=true");

        let response = app
            .oneshot(
                Request::builder()
                    .method(Method::POST)
                    .uri("/account/email/verification")
                    .header(header::CONTENT_TYPE, "application/x-www-form-urlencoded")
                    .body(params)
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::BAD_REQUEST);
        let bytes = body::to_bytes(response.into_body(), usize::MAX)
            .await
            .unwrap();
        let body_str = String::from_utf8(bytes.to_vec()).unwrap();
        assert!(body_str.contains("Already verified email"));
    }

    #[tokio::test]
    async fn test_already_send_verification_email() {
        let app = initialize().await;
        seed_database().await;
        let params = format!("sub=auth0|0000&email_verified=false");

        let response = app
            .oneshot(
                Request::builder()
                    .method(Method::POST)
                    .uri("/account/email/verification")
                    .header(header::CONTENT_TYPE, "application/x-www-form-urlencoded")
                    .body(params)
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::BAD_REQUEST);
        let bytes = body::to_bytes(response.into_body(), usize::MAX)
            .await
            .unwrap();
        let body_str = String::from_utf8(bytes.to_vec()).unwrap();
        assert!(body_str.contains("Already send email"));
    }

    #[tokio::test]
    async fn test_send_email_invalid_sub() {
        let app = initialize().await;
        let params = format!("sub=123&email_verified=false");

        let response = app
            .oneshot(
                Request::builder()
                    .method(Method::POST)
                    .uri("/account/email/verification")
                    .header(header::CONTENT_TYPE, "application/x-www-form-urlencoded")
                    .body(params)
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::INTERNAL_SERVER_ERROR);
        let bytes = body::to_bytes(response.into_body(), usize::MAX)
            .await
            .unwrap();
        let body_str = String::from_utf8(bytes.to_vec()).unwrap();
        assert!(body_str.contains("Could not process request"));
    }

    #[tokio::test]
    async fn test_send_verification_email() {
        let app = initialize().await;
        let sub = env::var("SUB_NOT_VERIFIED").unwrap();
        let params = format!("sub={sub}&email_verified=false");

        let response = app
            .oneshot(
                Request::builder()
                    .method(Method::POST)
                    .uri("/account/email/verification")
                    .header(header::CONTENT_TYPE, "application/x-www-form-urlencoded")
                    .body(params)
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);
        let bytes = body::to_bytes(response.into_body(), usize::MAX)
            .await
            .unwrap();
        let body_str = String::from_utf8(bytes.to_vec()).unwrap();
        assert!(body_str.contains("Succesfully send email"));
    }
}

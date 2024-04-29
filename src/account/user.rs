pub mod db;
pub mod model;
mod request;

use std::collections::HashMap;

use askama::Template;
use axum::{extract::State, response::Html, Form};
use indexmap::IndexMap;
use leprecon::{
    auth::{get_valid_jwt, AuthParam},
    template::{self, Snackbar},
    utils::{extract::extract_conn_from_pool, PostgresConn, RedisConn},
};
use reqwest::StatusCode;
use tracing::{debug, error};

use crate::{
    email::db::delete_email_sessions, user::db::update_customer_details, StateParams, AUTH_HOST,
    CLIENT_ID, CLIENT_SECRET,
};

use self::{
    db::{
        create_customer_details, customer_details_exist, delete_customer_details, delete_user,
        get_customer_details, get_user, insert_user,
    },
    model::{CustomerDetails, User},
    request::delete_user_from_auth_provider,
};

pub async fn user_information(
    State(state): State<StateParams>,
    Form(auth_param): Form<AuthParam>,
) -> (StatusCode, Html<String>) {
    let mut snackbar: Snackbar<'_> = Snackbar::new();

    if auth_param.sub.is_empty() {
        return (
            StatusCode::UNPROCESSABLE_ENTITY,
            Html(snackbar.render().unwrap()),
        );
    };

    let postgres_conn: PostgresConn = match extract_conn_from_pool(&state.2, &mut snackbar).await {
        Ok(v) => v,
        Err(e) => return e,
    };

    let user: User = match get_user(&auth_param.sub, &postgres_conn).await {
        Ok(v) => v,
        Err(e) => {
            debug!("Could not get user: {:?}", e);
            return (StatusCode::BAD_GATEWAY, Html(snackbar.render().unwrap()));
        }
    };

    let customer_details: CustomerDetails =
        match get_customer_details(&auth_param.sub, &postgres_conn).await {
            Ok(v) => v,
            Err(e) => {
                debug!("Could not get customer details: {:?}", e);
                return (StatusCode::BAD_GATEWAY, Html(snackbar.render().unwrap()));
            }
        };

    let user_template = template::UserInformation {
        account_details: template::AccountDetails {
            sub: user.sub,
            balance: user.balance,
            currency: user.currency.to_string(),
        },
        name_input: template::NameInput {
            inputs: IndexMap::from([
                ("first_name", customer_details.first_name),
                ("middle_name", customer_details.middle_name),
                ("last_name", customer_details.last_name),
            ]),
        },
        address_input: template::AddressInput {
            inputs: IndexMap::from([
                ("postal_code", customer_details.postal_code),
                ("street_name", customer_details.street_name),
                ("street_nr", customer_details.street_nr),
                ("premise", customer_details.premise),
                ("settlement", customer_details.settlement),
                ("country", customer_details.country),
                ("country_code", customer_details.country_code),
            ]),
        },
    };

    (StatusCode::OK, Html(user_template.render().unwrap()))
}

pub async fn create_user(
    State(state): State<StateParams>,
    Form(auth_param): Form<AuthParam>,
) -> (StatusCode, Html<String>) {
    let mut snackbar: Snackbar<'_> = Snackbar::new();

    if auth_param.sub.is_empty() {
        return (
            StatusCode::UNPROCESSABLE_ENTITY,
            Html(snackbar.render().unwrap()),
        );
    };

    let postgres_conn: PostgresConn = match extract_conn_from_pool(&state.2, &mut snackbar).await {
        Ok(v) => v,
        Err(e) => return e,
    };

    if let Err(e) = insert_user(&auth_param.sub, &postgres_conn).await {
        error!("Could not insert new user: {:?}", e);
        return (
            StatusCode::INTERNAL_SERVER_ERROR,
            Html(snackbar.render().unwrap()),
        );
    }

    snackbar.title = "Succes";
    snackbar.message = "Created user sucessfully";
    snackbar.color = "green";

    (StatusCode::OK, Html(snackbar.render().unwrap()))
}

pub async fn update_user_information(
    State(state): State<StateParams>,
    Form(params): Form<HashMap<String, String>>,
) -> (StatusCode, Html<String>) {
    let mut snackbar: Snackbar<'_> = Snackbar::new();

    let sub: &String = match params.get("sub") {
        Some(v) => v,
        None => {
            return (
                StatusCode::UNPROCESSABLE_ENTITY,
                Html(snackbar.render().unwrap()),
            );
        }
    };

    let customer_details: CustomerDetails = CustomerDetails {
        first_name: params.get("first_name").cloned(),
        middle_name: params.get("middle_name").cloned(),
        last_name: params.get("last_name").cloned(),
        postal_code: params.get("postal_code").cloned(),
        street_name: params.get("street_name").cloned(),
        street_nr: params.get("street_nr").cloned(),
        premise: params.get("premise").cloned(),
        settlement: params.get("settlement").cloned(),
        country: params.get("country").cloned(),
        country_code: params.get("country_code").cloned(),
    };

    let postgres_conn: PostgresConn = match extract_conn_from_pool(&state.2, &mut snackbar).await {
        Ok(v) => v,
        Err(e) => return e,
    };

    if customer_details_exist(sub, &postgres_conn).await {
        debug!("Already created customer details entry");
        if let Err(e) = update_customer_details(sub, customer_details, &postgres_conn).await {
            error!("Cannot update customer details entry: {:?}", e);
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Html(snackbar.render().unwrap()),
            );
        }
    } else if let Err(e) = create_customer_details(sub, customer_details, &postgres_conn).await {
        error!("Cannot create customer details entry: {:?}", e);
        return (
            StatusCode::INTERNAL_SERVER_ERROR,
            Html(snackbar.render().unwrap()),
        );
    }

    snackbar.title = "Succes";
    snackbar.message = "Updated personal details succesfully";
    snackbar.color = "green";

    (StatusCode::OK, Html(snackbar.render().unwrap()))
}

pub async fn user_balance(
    State(state): State<StateParams>,
    Form(auth_param): Form<AuthParam>,
) -> (StatusCode, Html<String>) {
    let mut snackbar: Snackbar<'_> = Snackbar::new();

    if auth_param.sub.is_empty() {
        return (
            StatusCode::UNPROCESSABLE_ENTITY,
            Html(snackbar.render().unwrap()),
        );
    };

    let postgres_conn: PostgresConn = match extract_conn_from_pool(&state.2, &mut snackbar).await {
        Ok(v) => v,
        Err(e) => return e,
    };

    let bal: User = match get_user(&auth_param.sub, &postgres_conn).await {
        Ok(v) => v,
        Err(e) => {
            error!("Could not fetch balance: {:?}", e);
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Html(snackbar.render().unwrap()),
            );
        }
    };

    let balance: template::Balance<'_> = template::Balance {
        amount: &bal.balance.to_string(),
        currency: &bal.currency.to_string(),
    };

    (StatusCode::OK, Html(balance.render().unwrap()))
}

pub async fn delete_account(
    State(state): State<StateParams>,
    Form(auth_param): Form<AuthParam>,
) -> (StatusCode, Html<String>) {
    let mut snackbar: Snackbar<'_> = Snackbar::new();

    if auth_param.sub.is_empty() {
        return (
            StatusCode::UNPROCESSABLE_ENTITY,
            Html(snackbar.render().unwrap()),
        );
    };

    let postgres_conn: PostgresConn = match extract_conn_from_pool(&state.2, &mut snackbar).await {
        Ok(v) => v,
        Err(e) => return e,
    };

    if let Err(e) = delete_customer_details(&auth_param.sub, &postgres_conn).await {
        error!("Cannot delete customer details entry: {:?}", e);
        return (
            StatusCode::INTERNAL_SERVER_ERROR,
            Html(snackbar.render().unwrap()),
        );
    }

    if let Err(e) = delete_email_sessions(&auth_param.sub, &postgres_conn).await {
        error!("Cannot delete session entrie(s): {:?}", e);
        return (
            StatusCode::INTERNAL_SERVER_ERROR,
            Html(snackbar.render().unwrap()),
        );
    }

    if let Err(e) = delete_user(&auth_param.sub, &postgres_conn).await {
        error!("Cannot delete user entry: {:?}", e);
        return (
            StatusCode::INTERNAL_SERVER_ERROR,
            Html(snackbar.render().unwrap()),
        );
    }

    let mut lock: tokio::sync::MutexGuard<'_, leprecon::auth::JWT> = state.0.lock().await;
    let req_client: &reqwest::Client = &state.1;

    let redis_conn: RedisConn = match extract_conn_from_pool(&state.3, &mut snackbar).await {
        Ok(v) => v,
        Err(e) => return e,
    };

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

    let res: reqwest::Response = match delete_user_from_auth_provider(
        &auth_param.sub,
        req_client,
        AUTH_HOST.get().unwrap(),
        &lock.access_token,
    )
    .await
    {
        Ok(v) => v,
        Err(e) => {
            error!("Cannot delete user from auth provider: {:?}", e);
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Html(snackbar.render().unwrap()),
            );
        }
    };

    if res.status() != reqwest::StatusCode::NO_CONTENT {
        error!("Could not delete user at auth provider");
        return (
            StatusCode::INTERNAL_SERVER_ERROR,
            Html(snackbar.render().unwrap()),
        );
    }

    snackbar.title = "Succes";
    snackbar.message = "Succesfully deleted account";
    snackbar.color = "green";

    (StatusCode::OK, Html(snackbar.render().unwrap()))
}

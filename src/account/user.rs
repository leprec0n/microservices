pub mod db;
pub mod model;

use std::collections::HashMap;

use askama::Template;
use axum::{response::Html, Form};
use indexmap::IndexMap;
use leprecon::template::{self, Snackbar};
use reqwest::StatusCode;
use tokio_postgres::NoTls;
use tracing::{debug, error, warn};

use crate::ACCOUNT_CONN;

use self::{
    db::{
        create_customer_details, customer_details_exist, get_customer_details, get_user,
        insert_user,
    },
    model::{CustomerDetails, User},
};

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

    let user: User = match get_user(sub, &db_client).await {
        Ok(v) => v,
        Err(e) => {
            debug!("Could not get user: {:?}", e);
            snackbar.message = "Could not process request";
            return (StatusCode::BAD_GATEWAY, Html(snackbar.render().unwrap()));
        }
    };

    let customer_details: CustomerDetails = match get_customer_details(sub, &db_client).await {
        Ok(v) => v,
        Err(e) => {
            debug!("Could not get customer details: {:?}", e);
            snackbar.message = "Could not process request";
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

pub async fn update_user_information(
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
            debug!("No sub provided");
            snackbar.message = "Could not process request";
            return (StatusCode::BAD_REQUEST, Html(snackbar.render().unwrap()));
        }
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

    if customer_details_exist(sub, &db_client).await {
        debug!("Already created customer details entry");
    } else if let Err(e) = create_customer_details(sub, customer_details, &db_client).await {
        error!("Cannot create customer details entry: {:?}", e);
        snackbar.message = "Could not process request";
        return (StatusCode::OK, Html(snackbar.render().unwrap()));
    }

    snackbar.title = "Succes";
    snackbar.message = "Succesfully updates personal details";
    snackbar.color = "green";

    (StatusCode::OK, Html(snackbar.render().unwrap()))
}

pub async fn user_balance(
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
            debug!("No sub provided");
            snackbar.message = "Could not process request";
            return (StatusCode::BAD_REQUEST, Html(snackbar.render().unwrap()));
        }
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

    let bal: User = match get_user(sub, &db_client).await {
        Ok(v) => v,
        Err(e) => {
            error!("Could not fetch balance: {:?}", e);
            snackbar.message = "Could not process request";
            return (StatusCode::BAD_REQUEST, Html(snackbar.render().unwrap()));
        }
    };

    let balance: template::Balance<'_> = template::Balance {
        amount: &bal.balance.to_string(),
        currency: &bal.currency.to_string(),
    };

    (StatusCode::OK, Html(balance.render().unwrap()))
}

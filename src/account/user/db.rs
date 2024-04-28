use std::{error::Error, str::FromStr};

use tokio_postgres::{Client, Row};
use tracing::debug;

use super::model::{Currency, CustomerDetails, User};

pub async fn insert_user(sub: &str, db_client: &Client) -> Result<Vec<Row>, tokio_postgres::Error> {
    db_client
        .query(
            "INSERT INTO users(sub, balance, currency_id) VALUES($1, 0.00, 1)",
            &[&sub],
        )
        .await
}

pub async fn get_user(sub: &str, db_client: &Client) -> Result<User, Box<dyn Error>> {
    let r: Row = db_client
        .query_one("SELECT * FROM users INNER JOIN currencies ON currencies.id = users.currency_id WHERE sub=$1 LIMIT 1", &[&sub])
        .await?;

    Ok(User {
        sub: r.get("sub"),
        balance: r.get("balance"),
        currency: Currency::from_str(r.get("acronym"))?,
    })
}

pub async fn customer_details_exist(sub: &str, db_client: &Client) -> bool {
    match db_client
        .query_one(
            "SELECT * FROM customer_details LEFT JOIN users ON users.id = customer_details.user_id WHERE sub=$1 LIMIT 1",
            &[&sub],
        )
        .await
    {
        Err(e) => {
            debug!("No customer details in db: {:?}", e);
            false
        }
        _ => true,
    }
}

pub async fn get_customer_details(
    sub: &str,
    db_client: &Client,
) -> Result<CustomerDetails, Box<dyn Error>> {
    let r: Row = db_client
        .query_one(
            "SELECT * FROM customer_details RIGHT JOIN users ON users.id = customer_details.user_id WHERE users.sub=$1 LIMIT 1",
            &[&sub],
        )
        .await?;

    Ok(CustomerDetails {
        first_name: r.get("first_name"),
        middle_name: r.get("middle_name"),
        last_name: r.get("last_name"),
        postal_code: r.get("postal_code"),
        street_name: r.get("street_name"),
        street_nr: r.get("street_nr"),
        premise: r.get("premise"),
        settlement: r.get("settlement"),
        country: r.get("country"),
        country_code: r.get("country_code"),
    })
}

pub async fn create_customer_details(
    sub: &str,
    customer_details: CustomerDetails,
    db_client: &Client,
) -> Result<Vec<Row>, tokio_postgres::Error> {
    db_client
        .query(
            "WITH userId AS (SELECT id FROM users WHERE sub = $1) INSERT INTO customer_details(first_name, middle_name, last_name, postal_code, street_name, street_nr, premise, settlement, country, country_code, user_id) VALUES($2, $3, $4, $5, $6, $7, $8, $9, $10, $11, (SELECT id FROM userId))",
            &[&sub, &customer_details.first_name, &customer_details.middle_name, &customer_details.last_name, &customer_details.postal_code, &customer_details.street_name, &customer_details.street_nr, &customer_details.premise, &customer_details.settlement, &customer_details.country, &customer_details.country_code],
        )
        .await
}

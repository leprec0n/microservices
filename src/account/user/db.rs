use std::{error::Error, str::FromStr};

use tokio_postgres::{Client, Row};

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

pub async fn get_customer_details(
    sub: &str,
    db_client: &Client,
) -> Result<CustomerDetails, Box<dyn Error>> {
    let r: Row = db_client
        .query_one(
            "SELECT * FROM customer_details RIGHT JOIN users ON users.id = customer_details.user_id LEFT JOIN currencies ON currencies.id = users.currency_id WHERE users.sub=$1 LIMIT 1",
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
        user: User {
            sub: r.get("sub"),
            balance: r.get("balance"),
            currency: Currency::from_str(r.get("acronym"))?,
        },
    })
}

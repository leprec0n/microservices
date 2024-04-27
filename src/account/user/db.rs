use tokio_postgres::{Client, Row};

use super::{Currency, User};

pub async fn insert_user(sub: &str, db_client: &Client) -> Result<Vec<Row>, tokio_postgres::Error> {
    db_client
        .query(
            "INSERT INTO users(sub, balance, currency_id) VALUES($1, 0.00, 1)",
            &[&sub],
        )
        .await
}

pub async fn get_balance(sub: &str, db_client: &Client) -> Result<User, tokio_postgres::Error> {
    let r: tokio_postgres::Row = db_client
        .query_one("SELECT * FROM users INNER JOIN currencies ON currencies.id = users.currency_id WHERE sub=$1 LIMIT 1", &[&sub])
        .await?;

    let c: Currency = match r.get("acronym") {
        "EUR" => Currency::EUR,
        _ => Currency::EUR,
    };
    Ok(User {
        sub: r.get("sub"),
        balance: r.get("balance"),
        currency: c,
    })
}

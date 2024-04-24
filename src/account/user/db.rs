use tokio_postgres::Client;

use super::{Currency, User};

pub async fn get_balance(email: &str, db_client: &Client) -> Result<User, tokio_postgres::Error> {
    let r: tokio_postgres::Row = db_client
        .query_one("SELECT * FROM users INNER JOIN currencies ON currencies.id = users.currency_id WHERE email=$1 LIMIT 1", &[&email])
        .await?;

    let c: Currency = match r.get("acronym") {
        "EUR" => Currency::EUR,
        _ => Currency::EUR,
    };
    Ok(User {
        email: r.get("email"),
        balance: r.get("balance"),
        currency: c,
    })
}

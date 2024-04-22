use tokio_postgres::Client;

use super::{Balance, Currency};

pub async fn get_balance(
    email: &str,
    db_client: &Client,
) -> Result<Balance, tokio_postgres::Error> {
    let r: tokio_postgres::Row = db_client
        .query_one("SELECT * FROM balance WHERE email=$1 LIMIT 1", &[&email])
        .await?;

    let c: Currency = match r.get("currency") {
        "EUR" => Currency::EUR,
        _ => Currency::EUR,
    };
    Ok(Balance {
        amount: r.get("balance"),
        currency: c,
    })
}

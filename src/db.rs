use std::collections::HashMap;

use chrono::{Duration, Local};
use tokio_postgres::Row;
use tracing::debug;

pub fn generate_db_conn(params: &HashMap<&str, &String>) -> String {
    format!(
        "postgresql://{user}:{password}@{host}/{db}",
        user = params.get("user").unwrap(),
        password = params.get("password").unwrap(),
        host = params.get("host").unwrap(),
        db = params.get("db").unwrap(),
    )
}

pub async fn verification_already_send(db_client: &tokio_postgres::Client, email: &str) -> bool {
    match db_client
        .query_one(
            "SELECT * FROM email WHERE expires > now() AND email=$1 ORDER BY expires DESC LIMIT 1",
            &[&email],
        )
        .await
    {
        Err(e) => {
            debug!("{:?}", e);
            return false;
        }
        _ => (),
    };

    true
}

pub async fn create_verification_session(
    db_client: &tokio_postgres::Client,
    email: &str,
) -> Result<Vec<Row>, tokio_postgres::Error> {
    let expires = 3600; // 60 minutes

    Ok(db_client
        .query(
            "INSERT INTO email(email, expires) VALUES($1, $2)",
            &[&email, &(Local::now() + Duration::seconds(expires))],
        )
        .await?)
}

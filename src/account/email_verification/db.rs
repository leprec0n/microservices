use chrono::{Duration, Local};
use tokio_postgres::Row;
use tracing::debug;

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

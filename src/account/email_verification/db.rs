use chrono::{Duration, Local};
use tokio_postgres::Row;
use tracing::debug;

use crate::model::SessionType;

pub async fn verification_already_send(db_client: &tokio_postgres::Client, email: &str) -> bool {
    match db_client
        .query_one(
            "SELECT * FROM session INNER JOIN \"user\" ON \"user\".id = session.email_id WHERE expires > now() AND email=$1 AND type=$2 ORDER BY expires DESC LIMIT 1",
            &[&email, &SessionType::Verification.to_string()],
        )
        .await
    {
        Err(e) => {
            debug!("No email verification session in db: {:?}", e);
            false
        }
        _ => true,
    }
}

pub async fn create_verification_session(
    db_client: &tokio_postgres::Client,
    email: &str,
) -> Result<Vec<Row>, tokio_postgres::Error> {
    let expires = 3600; // 60 minutes

    db_client
        .query(
            "WITH userId AS (SELECT id FROM \"user\" WHERE email = $1) INSERT INTO session(expires, type, email_id) VALUES($2, $3, (SELECT id FROM userId))",
            &[&email, &(Local::now() + Duration::seconds(expires)), &SessionType::Verification.to_string()],
        )
        .await
}

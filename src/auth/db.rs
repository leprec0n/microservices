use tokio_postgres::Row;
use tracing::{debug, trace};

use super::JWT;

pub async fn jwt_from_db(client: &tokio_postgres::Client) -> Result<Row, tokio_postgres::Error> {
    client
        .query_one(
            "SELECT * FROM account WHERE expires > now() ORDER BY expires DESC LIMIT 1",
            &[],
        )
        .await
}

pub(crate) async fn store_jwt(client: &tokio_postgres::Client, token: &JWT) {
    if token_exists(client, token).await {
        trace!("JWT already in database");
    }

    match client
        .query(
            "INSERT INTO account(access_token, expires, scope, token_type) VALUES($1, $2, $3, $4)",
            &[
                &token.access_token,
                &token.expires_in,
                &token.scope,
                &token.token_type,
            ],
        )
        .await
    {
        Ok(_) => trace!("Succesfully inserted jwt"),
        Err(e) => panic!("Cannot insert jwt: {:?}", e),
    };
}

async fn token_exists(client: &tokio_postgres::Client, token: &JWT) -> bool {
    match client
        .query_one(
            "SELECT * FROM account WHERE access_token=$1", // INSERT INTO account(access_token) VALUES($1)
            &[&token.access_token],
        )
        .await
    {
        Err(e) => {
            debug!("Token does not exist: {:?}", e);
            false
        }
        _ => true,
    }
}

use tracing::debug;

use super::JWT;

pub(crate) async fn store_jwt(client: &tokio_postgres::Client, token: &JWT) {
    if token_exists(client, token).await {
        debug!("JWT already in database!");
        return;
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
        Ok(_) => println!("Successfully inserted token."), // !TODO Log insert
        Err(e) => panic!("{:?}", e),                       // !TODO Log error
    };
}

pub async fn valid_jwt_from_db(client: &tokio_postgres::Client) -> Option<JWT> {
    let res = match client
        .query_one(
            "SELECT * FROM account WHERE expires > now() ORDER BY expires DESC LIMIT 1",
            &[],
        )
        .await
    {
        Ok(v) => v,
        Err(e) => {
            debug!("Cannot get jwt from db: {:?}", e);
            return None;
        }
    };

    Some(JWT {
        access_token: res.get("access_token"),
        expires_in: res.get("expires"),
        scope: res.get("scope"),
        token_type: res.get("token_type"),
    })
}

async fn token_exists(client: &tokio_postgres::Client, token: &JWT) -> bool {
    let res: Vec<tokio_postgres::Row> = match client
        .query(
            "SELECT * FROM account WHERE access_token=$1", // INSERT INTO account(access_token) VALUES($1)
            &[&token.access_token],
        )
        .await
    {
        Ok(v) => v,
        Err(e) => panic!("{:?}", e),
    };

    if !res.is_empty() {
        debug!("Token already exists!");
        return true;
    }

    false
}

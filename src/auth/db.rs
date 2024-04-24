use std::error::Error;

use redis::AsyncCommands;

use super::JWT;

pub(crate) async fn store_jwt(
    con: &mut redis::aio::MultiplexedConnection,
    token: &JWT,
) -> Result<(), Box<dyn Error>> {
    let v: String = serde_json::to_string(token)?;
    con.hset("session:account", "jwt", v).await?;
    con.expire_at("session:account", token.expires_in.timestamp())
        .await?;

    Ok(())
}

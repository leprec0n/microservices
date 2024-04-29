use std::error::Error;

use bb8_redis::RedisConnectionManager;
use chrono::Local;
use redis::AsyncCommands;
use tracing::debug;

use super::JWT;

pub(crate) async fn get_jwt_from_valkey(
    valkey_conn: &mut bb8_redis::bb8::PooledConnection<'_, RedisConnectionManager>,
) -> Option<JWT> {
    match valkey_conn.hget("session:account", "jwt").await {
        Ok(v) => {
            let value: String = v;
            match serde_json::from_str::<JWT>(&value) {
                Ok(v) => {
                    if v.expires_in > Local::now() {
                        debug!("Fetched jwt from session");
                        return Some(v);
                    }
                }
                Err(e) => debug!("Could not deserialize jwt: {:?}", e),
            };
        }
        Err(e) => debug!("Could not get jwt from session store: {:?}", e),
    };

    None
}

pub(crate) async fn store_jwt(
    mut conn: bb8_redis::bb8::PooledConnection<'_, RedisConnectionManager>,
    token: &JWT,
) -> Result<(), Box<dyn Error>> {
    let v: String = serde_json::to_string(token)?;

    conn.hset("session:account", "jwt", v).await?;
    conn.expire_at("session:account", token.expires_in.timestamp())
        .await?;

    Ok(())
}

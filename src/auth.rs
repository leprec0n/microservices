use std::error::Error;

pub mod db;
pub mod request;

mod model;

pub use model::*;
use redis::AsyncCommands;
use reqwest::StatusCode;
use tracing::debug;

use self::{db::store_jwt, request::jwt_from_auth_provider};

pub async fn get_valid_jwt(
    valkey_con: &mut redis::aio::MultiplexedConnection,
    req_client: &reqwest::Client,
    auth_host: &str,
    client_id: &str,
    client_secret: &str,
) -> Result<JWT, Box<dyn Error>> {
    // Get valid jwt from valkey
    match valkey_con.hget("session:account", "jwt").await {
        Ok(v) => {
            let value: String = v;
            match serde_json::from_str(&value) {
                Ok(v) => {
                    debug!("Fetched jwt from session");
                    return Ok(v);
                }
                Err(e) => debug!("Could not deserialize jwt: {:?}", e),
            };
        }
        Err(e) => debug!("Could not get jwt from session store: {:?}", e),
    };

    // Get new token from provider
    let response: reqwest::Response =
        jwt_from_auth_provider(req_client, auth_host, client_id, client_secret).await?;

    if response.status() != StatusCode::OK {
        Err("StatusCode not OK")?
    }

    let jwt: JWT = response.json().await?;

    // Store jwt in valkey
    store_jwt(valkey_con, &jwt).await?;

    debug!("Fetched jwt from auth provider");

    Ok(jwt)
}

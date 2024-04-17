use std::error::Error;

use jsonwebtoken::{Algorithm, DecodingKey, TokenData, Validation};

pub mod db;
pub mod request;

mod model;

pub use model::*;
use reqwest::StatusCode;
use tracing::debug;

use self::{
    db::{jwt_from_db, store_jwt},
    request::jwt_from_auth_provider,
};

pub async fn get_valid_jwt(
    db_client: &tokio_postgres::Client,
    req_client: &reqwest::Client,
    auth_host: &str,
    client_id: &str,
    client_secret: &str,
) -> Result<JWT, Box<dyn Error>> {
    // Check if valid jwt in db
    match jwt_from_db(&db_client).await {
        Ok(r) => {
            return Ok(JWT {
                access_token: r.get("access_token"),
                expires_in: r.get("expires"),
                scope: r.get("scope"),
                token_type: r.get("token_type"),
            })
        }
        Err(e) => debug!("Could not get jwt from db: {:?}", e),
    };

    // Get new token from provider
    let response: reqwest::Response =
        jwt_from_auth_provider(&req_client, auth_host, client_id, client_secret).await?;

    if response.status() != StatusCode::OK {
        Err("StatusCode not OK")?
    }

    let jwt: JWT = response.json().await?;

    // Store jwt in db
    store_jwt(&db_client, &jwt).await;

    Ok(jwt)
}

pub fn create_certificate(cert_body: &str) -> String {
    format!("-----BEGIN CERTIFICATE-----\n{cert_body}\n-----END CERTIFICATE-----")
}

pub fn decode_token(
    cert: String,
    client_aud: &str,
    token: &str,
) -> Result<TokenData<Claims>, jsonwebtoken::errors::Error> {
    // Create key from pem
    let key = DecodingKey::from_rsa_pem(cert.as_bytes())?;

    // Validation params
    let mut validation = Validation::new(Algorithm::RS256);
    validation.set_audience(&[client_aud]);

    // Decode token
    jsonwebtoken::decode::<Claims>(token, &key, &validation)
}

use jsonwebtoken::{Algorithm, DecodingKey, TokenData, Validation};

pub mod db;
pub mod request;

mod model;

pub use model::*;

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

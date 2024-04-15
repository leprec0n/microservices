use std::{
    collections::HashMap,
    fmt::{self, Display},
};

use axum::http::HeaderValue;
use chrono::{DateTime, Duration, Local};
use jsonwebtoken::{Algorithm, DecodingKey, TokenData, Validation};
use reqwest::{Response, StatusCode};
use serde::{Deserialize, Deserializer};
use tracing::debug;

#[derive(Clone, Debug, Deserialize)]
pub struct JWT {
    pub access_token: String,
    pub scope: String,
    #[serde(deserialize_with = "deserialize_expires_in")]
    pub expires_in: DateTime<Local>,
    pub token_type: String,
}

fn deserialize_expires_in<'de, D>(deserializer: D) -> Result<DateTime<Local>, D::Error>
where
    D: Deserializer<'de>,
{
    let expires_in = i64::deserialize(deserializer)?;
    Ok(Local::now() + Duration::seconds(expires_in))
}

impl Display for JWT {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(
            f,
            "access_token: {}, scope: {}, expires_in: {}, token_type: {}",
            self.access_token, self.scope, self.expires_in, self.token_type
        )
    }
}

pub async fn token_from_auth_provider(
    client_db: &tokio_postgres::Client,
    auth_host: &str,
    client_id: &str,
    client_secret: &str,
) -> JWT {
    // Create headers
    let mut headers = reqwest::header::HeaderMap::new();
    let content_type: HeaderValue =
        HeaderValue::from_str("application/x-www-form-urlencoded").unwrap();
    headers.insert("Content-Type", content_type);

    // Setup
    let token_url = format!("{auth_host}/oauth/token");
    let audience = format!("{auth_host}/api/v2/");

    let params = HashMap::from([
        ("grant_type", "client_credentials"),
        ("client_id", client_id),
        ("client_secret", client_secret),
        ("audience", &audience),
    ]);

    // Request
    let client: reqwest::Client = reqwest::Client::new();
    let response: Response = match client.post(token_url).form(&params).send().await {
        Ok(v) => v,
        Err(e) => panic!("{:?}", e),
    };

    if response.status() != StatusCode::OK {
        panic!("Response unsuccesfull");
    }

    let resp = match response.text().await {
        Ok(v) => v,
        Err(e) => panic!("Cannot get text: {:?}", e),
    };

    // Convert to JWT
    let jwt: JWT = match serde_json::from_str(&resp) {
        Ok(v) => v,
        Err(e) => panic!("{:?}", e),
    };

    // Store in db
    store_jwt(&client_db, &jwt).await;

    jwt
}

async fn store_jwt(client: &tokio_postgres::Client, token: &JWT) {
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

#[derive(Debug, Deserialize)]
#[allow(dead_code)]
pub struct Claims {
    pub aud: String,
    pub email: String,
    pub email_verified: bool,
    pub exp: u32,
    pub iat: u32,
    pub iss: String,
    pub name: String,
    pub nickname: String,
    pub picture: String,
    pub sid: String,
    pub sub: String,
    pub updated_at: String,
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

pub async fn send_email_verification(
    claims: Claims,
    client_id: &String,
    auth_host: &str,
    access_token: &str,
) -> Result<Response, reqwest::Error> {
    // Set headers
    let client = reqwest::Client::new();

    let mut headers = reqwest::header::HeaderMap::new();
    let content_type: HeaderValue = HeaderValue::from_str("application/json").unwrap();
    headers.insert("Content-Type", content_type.clone());
    headers.insert("Accept", content_type);

    // Setup
    let map: HashMap<&str, &String> =
        HashMap::from([("user_id", &claims.sub), ("client_id", client_id)]);

    // Send request
    client
        .post(format!("{auth_host}/api/v2/jobs/verification-email"))
        .json(&map)
        .bearer_auth(access_token)
        .send()
        .await
}

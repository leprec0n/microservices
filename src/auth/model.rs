use chrono::{DateTime, Duration, Local};
use serde::{Deserialize, Deserializer, Serialize};

#[derive(Clone, Deserialize)]
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

#[derive(Deserialize, Serialize)]
#[allow(dead_code)]
pub struct Claims {
    pub aud: String,
    pub email: String,
    pub email_verified: bool,
    pub exp: u64,
    pub iat: u64,
    pub iss: String,
    pub name: String,
    pub nickname: String,
    pub picture: String,
    pub sid: String,
    pub sub: String,
    pub updated_at: String,
}

#[derive(Deserialize)]
pub struct Keys {
    pub keys: Vec<JWKS>,
}

#[derive(Deserialize)]
pub struct JWKS {
    pub kty: String,
    pub r#use: String,
    pub n: String,
    pub e: String,
    pub kid: String,
    pub x5t: String,
    pub x5c: Vec<String>,
    pub alg: String,
}

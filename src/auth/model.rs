use chrono::{DateTime, Duration, Local};
use serde::{Deserialize, Deserializer, Serialize};
use serde_json::Value;

#[derive(Clone, Serialize, Deserialize)]
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
    let value: Value = Deserialize::deserialize(deserializer)?;
    let expires_in = match i64::deserialize(&value) {
        Ok(v) => v,
        Err(_) => {
            return Ok(DateTime::parse_from_rfc3339(value.as_str().unwrap())
                .unwrap()
                .with_timezone(&Local))
        }
    };
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

#[derive(Deserialize, Debug)]
pub struct AuthParam {
    #[serde(default)]
    pub sub: String,
}

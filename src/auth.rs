use std::fmt::{self, Display};

use chrono::{DateTime, Duration, Local};
use serde::{Deserialize, Deserializer};

#[derive(Clone, Debug, Deserialize)]
pub struct Token {
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

impl Display for Token {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(
            f,
            "access_token: {}, scope: {}, expires_in: {}, token_type: {}",
            self.access_token, self.scope, self.expires_in, self.token_type
        )
    }
}

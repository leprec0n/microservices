use std::collections::HashMap;

use axum::http::HeaderValue;
use leprecon::auth::Claims;
use reqwest::Response;

pub async fn send_email_verification(
    claims: &Claims,
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

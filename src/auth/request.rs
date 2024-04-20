use std::collections::HashMap;

use axum::http::HeaderValue;
use reqwest::{Response, StatusCode};

use super::Keys;

pub async fn jwt_from_auth_provider(
    req_client: &reqwest::Client,
    auth_host: &str,
    client_id: &str,
    client_secret: &str,
) -> Result<Response, reqwest::Error> {
    // Create headers
    let mut headers = reqwest::header::HeaderMap::new();
    let content_type: HeaderValue =
        HeaderValue::from_str("application/x-www-form-urlencoded").unwrap();
    headers.insert("Content-Type", content_type);

    // Setup
    let token_url: String = format!("{auth_host}/oauth/token");
    let audience: String = format!("{auth_host}/api/v2/");

    let params: HashMap<&str, &str> = HashMap::from([
        ("grant_type", "client_credentials"),
        ("client_id", client_id),
        ("client_secret", client_secret),
        ("audience", &audience),
    ]);

    // Request
    req_client.post(token_url).form(&params).send().await
}

pub async fn fetch_jwks(
    req_client: &reqwest::Client,
    auth_host: &str,
) -> Result<Keys, reqwest::Error> {
    let response = match req_client
        .get(format!("{}/.well-known/jwks.json", auth_host))
        .send()
        .await
    {
        Ok(v) => v,
        Err(e) => panic!("Could not fetch certificate: {:?}", e),
    };

    if response.status() == StatusCode::OK {
        return response.json().await;
    }

    panic!(
        "Fetching certificate failed with statuscode: {:?}",
        response.status()
    )
}

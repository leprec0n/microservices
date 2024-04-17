use std::collections::HashMap;

use axum::http::HeaderValue;
use reqwest::{Response, StatusCode};

use super::{db::store_jwt, Keys, JWT};

pub async fn token_from_auth_provider(
    req_client: &reqwest::Client,
    db_client: &tokio_postgres::Client,
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
    let response: Response = match req_client.post(token_url).form(&params).send().await {
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
    store_jwt(&db_client, &jwt).await;

    jwt
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

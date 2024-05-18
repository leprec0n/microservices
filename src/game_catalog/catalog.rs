use axum::response::Html;
use reqwest::StatusCode;

pub(super) async fn get_catalog() -> (StatusCode, Html<String>) {
    (StatusCode::OK, Html("Game catalog".to_owned()))
}

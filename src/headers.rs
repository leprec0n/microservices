use axum::http::HeaderName;

/// Returns htmx headers.
pub fn htmx_headers() -> Vec<HeaderName> {
    vec![
        HeaderName::from_static("hx-current-url"),
        HeaderName::from_static("hx-request"),
        HeaderName::from_static("hx-target"),
        HeaderName::from_static("hx-trigger"),
    ]
}

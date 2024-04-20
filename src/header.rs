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

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_header_length() {
        assert_eq!(4, htmx_headers().len());
    }
}

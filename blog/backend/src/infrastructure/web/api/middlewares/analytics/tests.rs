use axum::http::{HeaderMap, HeaderValue};

use super::{normalize_country_code, path_group};

#[test]
fn normalizes_cloudflare_country_header() {
    let mut headers = HeaderMap::new();
    headers.insert("cf-ipcountry", HeaderValue::from_static("de"));
    assert_eq!(normalize_country_code(&headers), "DE");

    headers.insert("cf-ipcountry", HeaderValue::from_static("unknown"));
    assert_eq!(normalize_country_code(&headers), "XX");

    assert_eq!(normalize_country_code(&HeaderMap::new()), "XX");
}

#[test]
fn groups_paths_by_first_segment() {
    assert_eq!(path_group("/posts/latest"), "posts");
    assert_eq!(path_group("/projects/s/demo"), "projects");
    assert_eq!(path_group("/"), "root");
}

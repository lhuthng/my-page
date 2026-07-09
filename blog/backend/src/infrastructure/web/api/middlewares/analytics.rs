use axum::http::HeaderMap;

pub fn normalize_country_code(headers: &HeaderMap) -> String {
    let Some(value) = headers.get("cf-ipcountry").and_then(|value| value.to_str().ok()) else {
        return "XX".to_string();
    };

    let country = value.trim().to_ascii_uppercase();
    if country.len() == 2 && country.chars().all(|ch| ch.is_ascii_alphabetic()) {
        country
    } else {
        "XX".to_string()
    }
}

pub fn path_group(path: &str) -> String {
    path.trim_start_matches('/')
        .split('/')
        .next()
        .filter(|segment| !segment.is_empty())
        .unwrap_or("root")
        .to_string()
}

#[cfg(test)]
mod tests {
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
}

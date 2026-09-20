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
mod tests;

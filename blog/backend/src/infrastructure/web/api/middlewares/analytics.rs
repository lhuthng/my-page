use axum::http::HeaderMap;

pub fn normalize_country_code(headers: &HeaderMap) -> String {
    let Some(value) = headers
        .get("cf-ipcountry")
        .and_then(|value| value.to_str().ok())
    else {
        return "XX".to_string();
    };

    let country = value.trim().to_ascii_uppercase();
    if country.len() == 2 && country.chars().all(|ch| ch.is_ascii_alphabetic()) {
        country
    } else {
        "XX".to_string()
    }
}

/// The only groups the visit beacon may record.
///
/// `POST /analytics/visit` is anonymous and takes the path from the caller, and
/// every distinct first segment used to become a new row keyed by
/// `(day, country, path_group)`. That made the table grow one row per request
/// for anyone willing to send made-up paths. Anything outside this list is
/// recorded as `other`, which keeps cardinality bounded by routes, not callers.
const PATH_GROUPS: &[&str] = &[
    "about",
    "audiobooks",
    "dashboard",
    "games",
    "instrument",
    "login",
    "newsletter",
    "posts",
    "privacy",
    "profiles",
    "projects",
    "reset-password",
    "sandbox",
    "series",
    "tags",
    "verify-email",
];

/// Longest path we bother looking at. Longer input is not a route.
const MAX_PATH_LEN: usize = 256;

pub fn path_group(path: &str) -> String {
    if path.len() > MAX_PATH_LEN {
        return "other".to_string();
    }

    let segment = path
        .trim_start_matches('/')
        .split('/')
        .next()
        .filter(|segment| !segment.is_empty());

    match segment {
        None => "root".to_string(),
        Some(segment) if PATH_GROUPS.contains(&segment) => segment.to_string(),
        Some(_) => "other".to_string(),
    }
}

#[cfg(test)]
mod tests;

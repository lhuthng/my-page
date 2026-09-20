// Link validation shared by the game and project handlers. Error
// construction is injected so each aggregate keeps its own error type.
use crate::helper::string::{validate_http_url, validate_text};

/// Validate a demo URL when one is present. An empty value is how the editor
/// clears the field, so it is passed through untouched.
pub fn validate_demo_url<E>(
    url: Option<String>,
    invalid: impl Fn(String) -> E,
) -> Result<Option<String>, E> {
    match url {
        Some(u) if u.trim().is_empty() => Ok(Some(String::new())),
        Some(u) => Ok(Some(validate_http_url(&u, "Demo URL").map_err(invalid)?)),
        None => Ok(None),
    }
}

/// Validate one `{ label, url }` link pair: the label is bounded text and the
/// URL must be http(s). Link URLs are rendered into an `href`, so a
/// `javascript:` or `data:` value must not be storable in the first place.
pub fn validate_link(label: &str, url: &str) -> Result<(String, String), String> {
    Ok((
        validate_text(label, "Link label", 100)?,
        validate_http_url(url, "Link URL")?,
    ))
}

pub fn replace_range_unicode(s: &mut String, start: usize, size: usize, insert: String) {
    s.replace_range(start..(start + size), insert.as_str());
}

/// Validate and normalize an SEO slug (posts, series, projects, tags).
///
/// Trims, lowercases, then enforces a minimum length, a maximum length, and an
/// allowlist of characters. Empty / whitespace-only / non-ASCII inputs are
/// rejected so URLs and lookups stay sane.
pub fn validate_slug(raw: &str) -> Result<String, String> {
    const MIN: usize = 2;
    const MAX: usize = 100;

    let slug = raw.trim().to_lowercase();

    if slug.len() < MIN {
        return Err(format!("Slug must be at least {MIN} characters."));
    }
    if slug.len() > MAX {
        return Err(format!("Slug must be at most {MAX} characters."));
    }
    if !slug
        .chars()
        .all(|ch| ch.is_ascii_lowercase() || ch.is_ascii_digit() || ch == '-' || ch == '_')
    {
        return Err(
            "Slug may only contain lowercase letters, numbers, hyphens, and underscores.".into(),
        );
    }
    Ok(slug)
}

/// Validate a free-text field: must be non-blank (after trimming) and within
/// `max` characters. Returns the trimmed value on success.
pub fn validate_text(raw: &str, name: &str, max: usize) -> Result<String, String> {
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return Err(format!("{name} must not be empty."));
    }
    if trimmed.chars().count() > max {
        return Err(format!("{name} must be at most {max} characters."));
    }
    Ok(trimmed.to_string())
}

/// Validate an optional long text field (e.g. a bio): all-whitespace is treated
/// as empty. Returns `None` for blank input, `Some` for a trimmed valid value.
pub fn validate_optional_long_text(
    raw: Option<&str>,
    name: &str,
    max: usize,
) -> Result<Option<String>, String> {
    let Some(raw) = raw else { return Ok(None) };
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return Ok(None);
    }
    if trimmed.chars().count() > max {
        return Err(format!("{name} must be at most {max} characters."));
    }
    Ok(Some(trimmed.to_string()))
}

/// Upper bound on a post or project body, in characters.
///
/// Bodies were previously unbounded: the only ceiling was the 100 MB request
/// body limit, so a single row could hold tens of megabytes of text that then
/// had to be rendered on every request.
pub const MAX_BODY_CHARS: usize = 400_000;

/// Validate a post/project body (`content` or `draft`). Unlike `validate_text`
/// an empty body is allowed — a new draft legitimately starts blank — but the
/// length is capped.
pub fn validate_body(raw: &str, name: &str) -> Result<String, String> {
    if raw.chars().count() > MAX_BODY_CHARS {
        return Err(format!(
            "{name} must be at most {MAX_BODY_CHARS} characters."
        ));
    }
    Ok(raw.to_string())
}

/// Validate a user-supplied link, restricting it to http(s).
///
/// Without this a `javascript:` or `data:` URL could be stored and later
/// rendered into an `href`/`src`.
pub fn validate_http_url(raw: &str, name: &str) -> Result<String, String> {
    const MAX: usize = 2048;

    let url = raw.trim();
    if url.is_empty() {
        return Err(format!("{name} must not be empty."));
    }
    if url.len() > MAX {
        return Err(format!("{name} must be at most {MAX} characters."));
    }

    let lowered = url.to_ascii_lowercase();
    if !lowered.starts_with("http://") && !lowered.starts_with("https://") {
        return Err(format!("{name} must be an http:// or https:// URL."));
    }
    // Control characters would let a value break out of the attribute it is
    // rendered into, independently of escaping at the render site.
    if url.chars().any(|ch| ch.is_control()) {
        return Err(format!("{name} must not contain control characters."));
    }
    Ok(url.to_string())
}

/// Clamp an optional page size into `1..=max`, defaulting to `default` when
/// absent.
pub fn clamp_page_size(value: Option<i64>, default: i64, max: i64) -> i64 {
    value.unwrap_or(default).clamp(1, max)
}

/// Clamp an optional offset to non-negative.
pub fn clamp_offset(value: Option<i64>) -> i64 {
    value.unwrap_or(0).max(0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn slug_rejects_empty_and_whitespace() {
        assert!(validate_slug("").is_err());
        assert!(validate_slug("   ").is_err());
        assert!(validate_slug("-").is_err());
    }

    #[test]
    fn slug_rejects_bad_chars() {
        assert!(validate_slug("A B").is_err());
        assert!(validate_slug("über").is_err());
        assert!(validate_slug("a/b").is_err());
        // Uppercase is normalized, not rejected (see `slug_normalizes_valid_input`).
    }

    #[test]
    fn slug_rejects_too_long() {
        assert!(validate_slug(&"a".repeat(101)).is_err());
    }

    #[test]
    fn slug_normalizes_valid_input() {
        assert_eq!(validate_slug("  Hello-World_1  ").unwrap(), "hello-world_1");
        assert_eq!(validate_slug("Abc-DEF").unwrap(), "abc-def");
    }

    #[test]
    fn text_rejects_blank_and_overlength() {
        assert!(validate_text("", "Title", 100).is_err());
        assert!(validate_text("   ", "Title", 100).is_err());
        assert!(validate_text(&"a".repeat(101), "Title", 100).is_err());
    }

    #[test]
    fn text_trims() {
        assert_eq!(validate_text("  Hi  ", "Title", 100).unwrap(), "Hi");
    }

    #[test]
    fn optional_text_blank_becomes_none() {
        assert_eq!(validate_optional_long_text(None, "bio", 500).unwrap(), None);
        assert_eq!(
            validate_optional_long_text(Some("   "), "bio", 500).unwrap(),
            None
        );
        assert_eq!(
            validate_optional_long_text(Some("  hey  "), "bio", 500).unwrap(),
            Some("hey".to_string())
        );
    }

    #[test]
    fn page_clamping() {
        assert_eq!(clamp_page_size(None, 24, 100), 24);
        assert_eq!(clamp_page_size(Some(0), 24, 100), 1);
        assert_eq!(clamp_page_size(Some(-5), 24, 100), 1);
        assert_eq!(clamp_page_size(Some(9999), 24, 100), 100);
        assert_eq!(clamp_offset(None), 0);
        assert_eq!(clamp_offset(Some(-3)), 0);
        assert_eq!(clamp_offset(Some(7)), 7);
    }
}

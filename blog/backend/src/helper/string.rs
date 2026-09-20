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

/// Validate a post/project/game body. Unlike `validate_text` an empty body is
/// allowed — a new entry legitimately starts blank — but the length is capped.
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

/// Derive a URL-safe slug from arbitrary free text.
///
/// Unlike `validate_slug`, which rejects anything outside the allowlist, this
/// normalizes: it lowercases, replaces every run of non-alphanumeric characters
/// with a single hyphen, and trims leading/trailing hyphens. Used for
/// user-facing labels that double as identifiers, such as audiobook tag names,
/// where rejecting "Science Fiction" would be hostile.
///
/// Non-ASCII letters are dropped because the slug allowlist is ASCII-only;
/// a name made entirely of such characters yields an empty slug, which callers
/// must treat as invalid input.
pub fn slugify(raw: &str) -> String {
    let mut slug = String::with_capacity(raw.len());
    let mut pending_separator = false;

    for ch in raw.trim().chars() {
        if ch.is_ascii_alphanumeric() {
            if pending_separator && !slug.is_empty() {
                slug.push('-');
            }
            pending_separator = false;
            slug.push(ch.to_ascii_lowercase());
        } else {
            pending_separator = true;
        }
    }

    slug
}

/// A short random hex token, used to make generated identifiers collision-proof
/// (e.g. media `short_name`, which carries a UNIQUE constraint).
pub fn random_suffix() -> String {
    use rand::RngCore;

    let mut bytes = [0u8; 4];
    rand::rng().fill_bytes(&mut bytes);
    hex::encode(bytes)
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
mod tests;

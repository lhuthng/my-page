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

#[test]
fn slugify_normalizes_free_text() {
    assert_eq!(slugify("Science Fiction"), "science-fiction");
    assert_eq!(slugify("  Audio   Drama  "), "audio-drama");
    assert_eq!(slugify("Sci-Fi / Fantasy"), "sci-fi-fantasy");
    assert_eq!(slugify("C++ & Rust"), "c-rust");
    assert_eq!(slugify("Already-slug_1"), "already-slug-1");
}

#[test]
fn slugify_collapses_and_trims_separators() {
    assert_eq!(slugify("---hello---"), "hello");
    assert_eq!(slugify("a___b"), "a-b");
    assert_eq!(slugify("!!!"), "");
    assert_eq!(slugify(""), "");
}

#[test]
fn slugify_is_idempotent() {
    for input in ["Science Fiction", "Sci-Fi / Fantasy", "Already-slug_1"] {
        let once = slugify(input);
        assert_eq!(slugify(&once), once);
    }
}

#[test]
fn random_suffix_is_eight_hex_chars_and_varies() {
    let first = random_suffix();
    assert_eq!(first.len(), 8);
    assert!(first.chars().all(|c| c.is_ascii_hexdigit()));
    // Not a guarantee, but a constant value here would mean a broken RNG.
    let second = random_suffix();
    assert_ne!(first, second);
}

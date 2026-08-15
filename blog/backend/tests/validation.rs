use backend::helper::string::{
    MAX_BODY_CHARS, clamp_offset, clamp_page_size, validate_body, validate_http_url,
    validate_optional_long_text, validate_slug, validate_text,
};

#[test]
fn body_allows_empty_but_caps_length() {
    // A brand-new draft is legitimately blank.
    assert_eq!(validate_body("", "Draft").unwrap(), "");
    assert!(validate_body(&"a".repeat(MAX_BODY_CHARS), "Draft").is_ok());
    assert!(validate_body(&"a".repeat(MAX_BODY_CHARS + 1), "Draft").is_err());
}

#[test]
fn body_counts_characters_not_bytes() {
    // A multi-byte body under the character cap must not be rejected.
    assert!(validate_body(&"é".repeat(MAX_BODY_CHARS), "Draft").is_ok());
}

#[test]
fn http_url_accepts_http_and_https() {
    assert_eq!(
        validate_http_url("  https://example.com/x  ", "Link URL").unwrap(),
        "https://example.com/x"
    );
    assert!(validate_http_url("http://example.com", "Link URL").is_ok());
    assert!(validate_http_url("HTTPS://EXAMPLE.COM", "Link URL").is_ok());
}

#[test]
fn http_url_rejects_dangerous_schemes() {
    for bad in [
        "javascript:alert(1)",
        "JavaScript:alert(1)",
        "data:text/html;base64,PHNjcmlwdD4=",
        "file:///etc/passwd",
        "//example.com",
        "example.com",
        "",
        "   ",
    ] {
        assert!(
            validate_http_url(bad, "Link URL").is_err(),
            "{bad:?} should have been rejected"
        );
    }
}

#[test]
fn http_url_rejects_control_characters() {
    assert!(validate_http_url("https://example.com/\nx", "Link URL").is_err());
    assert!(validate_http_url("https://example.com/\tx", "Link URL").is_err());
}

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
    assert!(validate_slug("a.b").is_err());
}

#[test]
fn slug_normalizes_uppercase_and_whitespace() {
    assert_eq!(validate_slug("  Hello-World  ").unwrap(), "hello-world");
    assert_eq!(validate_slug("HelloWorld").unwrap(), "helloworld");
}

#[test]
fn slug_rejects_too_long() {
    assert!(validate_slug(&"a".repeat(101)).is_err());
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
fn optional_long_text_blank_becomes_none() {
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
    assert_eq!(clamp_page_size(None, 5, 100), 5);
    assert_eq!(clamp_page_size(Some(0), 24, 100), 1);
    assert_eq!(clamp_page_size(Some(-5), 24, 100), 1);
    assert_eq!(clamp_page_size(Some(9999), 24, 100), 100);
    assert_eq!(clamp_offset(None), 0);
    assert_eq!(clamp_offset(Some(-3)), 0);
    assert_eq!(clamp_offset(Some(7)), 7);
}

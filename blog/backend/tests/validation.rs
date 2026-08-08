use backend::helper::string::{
    clamp_offset, clamp_page_size, validate_optional_long_text, validate_slug, validate_text,
};

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
    assert_eq!(validate_optional_long_text(Some("   "), "bio", 500).unwrap(), None);
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
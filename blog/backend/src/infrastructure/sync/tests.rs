use super::*;

use std::path::PathBuf;

#[test]
fn sync_key_roundtrip() {
    let key = generate_sync_key().unwrap();
    assert!(key.starts_with(SYNC_KEY_PREFIX));
    let secret = key.trim_start_matches(SYNC_KEY_PREFIX);
    assert_eq!(secret.len(), 64);
    assert!(secret.bytes().all(|b| b.is_ascii_hexdigit()));
    assert_eq!(hash_sync_key(&key).len(), 64);
    assert_ne!(generate_sync_key().unwrap(), key);
}

#[test]
fn canonical_media_urls_match_serving_layout() {
    let root = PathBuf::from("./media");
    // Regular media: <root>/<h[0..2]>/<h[2..4]>/<h><ext>
    assert_eq!(
        canonical_media_url("abcdef1234", "image/png", 7, &root).unwrap(),
        "./media/ab/cd/abcdef1234.png"
    );
    // Post cover: directory uses the uploader id, not the post id.
    assert_eq!(
        canonical_media_url(".post.42.abcdef1234", "image/webp", 11, &root).unwrap(),
        "./media/post/11/abcdef1234.webp"
    );
    assert_eq!(
        canonical_media_url(".avt.3.abcdef1234", "image/jpeg", 3, &root).unwrap(),
        "./media/avt/3/abcdef1234.jpeg"
    );
    assert_eq!(
        canonical_media_url(".srs.9.abcdef1234", "video/mp4", 9, &root).unwrap(),
        "./media/srs/9/abcdef1234.mp4"
    );
    // Unparseable file types leave the row untouched.
    assert!(canonical_media_url("abcdef1234", "not/a-type", 1, &root).is_none());
    assert!(canonical_media_url("garbage with spaces", "image/png", 1, &root).is_none());
}

#[test]
fn artifact_key_shape_check() {
    for key in ["", "/abs", "a/../b", "a//b", "a/./b", "a\\b", "."] {
        assert!(
            !is_valid_artifact_key_shape(key),
            "'{key}' must be rejected"
        );
    }
    for key in [
        "v86/games/abc/full.iso",
        "v86/assets/systems/abc/0-262144.img.zst",
        "v86/snapshots/abc/state.zst",
        "v86/saves/1/2/save.zst",
        "jsdos/5/sha.jsdos",
    ] {
        assert!(is_valid_artifact_key_shape(key), "'{key}' must be accepted");
    }
}

//! Secret handling for book-scoped author API keys.
//!
//! Mirrors `infrastructure/sync` on purpose: a random 32-byte secret with a
//! recognizable prefix, only its SHA-256 persisted, shown once at creation.
//! The difference is scope — an `abk_` key unlocks one audiobook, while a
//! `bsk_` key pulls the entire database — which is why they do not share a
//! table or even a prefix.

use rand::RngCore;
use sha2::{Digest, Sha256};

pub const AUTHOR_KEY_PREFIX: &str = "abk_";

/// `abk_<64 hex>`; 256 bits of entropy, generated from the OS RNG.
pub fn generate_author_key() -> String {
    let mut bytes = [0u8; 32];
    rand::rng().fill_bytes(&mut bytes);
    format!("{AUTHOR_KEY_PREFIX}{}", hex::encode(bytes))
}

pub fn hash_author_key(secret: &str) -> String {
    hex::encode(Sha256::digest(secret.as_bytes()))
}

/// A short, non-secret fingerprint for the dashboard's key list, e.g.
/// `abk_3f9c…`. Never enough to reconstruct the key.
pub fn author_key_hint(secret: &str) -> String {
    let prefix_length = AUTHOR_KEY_PREFIX.len() + 8;
    let mut short: String = secret.chars().take(prefix_length).collect();
    short.push('…');
    short
}

/// Turns a lower-cased hex SHA-256 into a form safe to compare. Returns `None`
/// for anything else, so a junk header is a validation error rather than a
/// silent match against a truncated digest.
pub fn normalize_sha256(raw: &str) -> Option<String> {
    let value = raw.trim().to_ascii_lowercase();
    if value.len() == 64 && value.bytes().all(|byte| byte.is_ascii_hexdigit()) {
        Some(value)
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn generated_keys_are_prefixed_hex_and_unique() {
        let a = generate_author_key();
        let b = generate_author_key();
        assert!(a.starts_with(AUTHOR_KEY_PREFIX));
        assert_eq!(a.len(), AUTHOR_KEY_PREFIX.len() + 64);
        assert_ne!(a, b);
        assert!(
            a[AUTHOR_KEY_PREFIX.len()..]
                .bytes()
                .all(|byte| byte.is_ascii_hexdigit())
        );
    }

    #[test]
    fn the_stored_value_is_only_a_digest() {
        let key = generate_author_key();
        let hash = hash_author_key(&key);
        assert_eq!(hash.len(), 64);
        assert!(!hash.contains(&key));
    }

    #[test]
    fn the_hint_cannot_reconstruct_the_key() {
        let key = generate_author_key();
        let hint = author_key_hint(&key);
        assert!(hint.starts_with("abk_"));
        assert!(hint.ends_with('…'));
        assert!(hint.len() < key.len() / 2);
    }

    #[test]
    fn sha256_normalization_rejects_anything_but_a_full_digest() {
        let digest = "b4c0ffee".repeat(8);
        assert_eq!(
            normalize_sha256(&digest.to_ascii_uppercase()),
            Some(digest.clone())
        );
        assert_eq!(normalize_sha256(&format!(" {digest} ")), Some(digest));
        assert_eq!(normalize_sha256("b4c0"), None);
        assert_eq!(normalize_sha256(""), None);
        assert_eq!(normalize_sha256(&"z".repeat(64)), None);
    }
}

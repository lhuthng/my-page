//! Secret handling for game invitation codes.
//!
//! Mirrors `author_keys` on purpose: a random secret, only its SHA-256
//! persisted, shown once at creation, a short hint for the dashboard list. The
//! shape differs because invitation codes are typed by humans: four Crockford
//! base32 groups instead of hex, normalized (upper-cased, `0`/`O` folded
//! together, `1`/`I`/`L` folded together, separators ignored) before hashing,
//! so a code works no matter how it is copied or retyped.

use rand::RngCore;
use sha2::{Digest, Sha256};

/// Crockford base32 without the look-alike letters a human has to disambiguate.
const ALPHABET: &[u8] = b"0123456789ABCDEFGHJKMNPQRSTVWXYZ";
const GROUPS: usize = 4;
const GROUP_LEN: usize = 4;

/// `XXXX-XXXX-XXXX-XXXX`: 80 bits of entropy from the OS RNG. Brute-forcing
/// that through the redeem endpoint's rate limit is not a strategy.
pub fn generate_invitation_code() -> String {
    // 5 usable bits per alphabet character.
    let mut bytes = vec![0u8; GROUPS * GROUP_LEN];
    rand::rng().fill_bytes(&mut bytes);
    let code: String = bytes
        .iter()
        .map(|byte| ALPHABET[(*byte as usize >> 3) % ALPHABET.len()] as char)
        .collect();
    group(&code).join("-")
}

/// Folds look-alike characters and drops separators, so `abcd-efgh-ijkl-mnop`,
/// `ABCD EFGH IJKL MNOP` and a tab-separated `ABCDEF GHIJKLMNOP` all hash to
/// one code.
pub fn normalize_invitation_code(raw: &str) -> Option<String> {
    let folded: String = raw
        .chars()
        .filter(|c| c.is_ascii_alphanumeric())
        .map(|c| c.to_ascii_uppercase())
        .map(|c| match c {
            'O' => '0',
            'I' | 'L' => '1',
            other => other,
        })
        .collect();
    if folded.len() != GROUPS * GROUP_LEN || !folded.bytes().all(|byte| ALPHABET.contains(&byte)) {
        return None;
    }
    Some(folded)
}

pub fn hash_invitation_code(normalized: &str) -> String {
    hex::encode(Sha256::digest(normalized.as_bytes()))
}

/// The last group, for the dashboard's list: enough to tell codes apart,
/// useless for playing.
pub fn code_hint(normalized: &str) -> String {
    normalized.chars().skip((GROUPS - 1) * GROUP_LEN).collect()
}

fn group(code: &str) -> Vec<String> {
    code.chars()
        .collect::<Vec<_>>()
        .chunks(GROUP_LEN)
        .map(|chunk| chunk.iter().collect())
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn generated_codes_are_grouped_and_in_the_alphabet() {
        let code = generate_invitation_code();
        assert_eq!(code.len(), GROUPS * GROUP_LEN + GROUPS - 1);
        let normalized = normalize_invitation_code(&code).expect("own codes normalize");
        assert_eq!(normalized, code.replace('-', ""));
        assert_eq!(code_hint(&normalized), &code[code.len() - GROUP_LEN..]);
    }

    #[test]
    fn look_alikes_and_missing_separators_normalize_to_one_code() {
        let code = generate_invitation_code();
        let normalized = normalize_invitation_code(&code).unwrap();
        // Lower-cased, separators eaten, an O for a 0, an I for a 1.
        let sloppy = code
            .replace('-', "")
            .replace('0', "o")
            .replace('1', "i")
            .to_ascii_lowercase();
        assert_eq!(normalize_invitation_code(&sloppy), Some(normalized.clone()));
        let spaced = code.replace('-', " ");
        assert_eq!(normalize_invitation_code(&spaced), Some(normalized));
    }

    #[test]
    fn junk_is_a_validation_error_not_a_match() {
        assert_eq!(normalize_invitation_code("hello"), None);
        assert_eq!(normalize_invitation_code(""), None);
        assert_eq!(normalize_invitation_code("12345"), None);
        // U is not in Crockford base32.
        assert_eq!(normalize_invitation_code("UUUUUUUUUUUUUUUU"), None);
    }

    #[test]
    fn the_stored_value_is_only_a_digest() {
        let code = generate_invitation_code();
        let hash = hash_invitation_code(&normalize_invitation_code(&code).unwrap());
        assert_eq!(hash.len(), 64);
        assert!(!hash.contains(&code));
    }
}

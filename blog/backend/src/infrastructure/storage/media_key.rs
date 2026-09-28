//! Where a `media` row's bytes live, as one store-independent key.
//!
//! This is the only place the layout is defined. The local disk path is
//! `MEDIA_PATH.join(key)`; audio additionally has a bucket object key, which is
//! the same string under a prefix. That is what makes the two stores
//! interchangeable for audio and the backfill a plain copy.
//!
//! **Only audio leaves the disk.** Images, covers, avatars, video and models
//! have a key and nothing else — see [`audio_object_key`], which returns `None`
//! for them.
//!
//! Four layouts exist, all content-addressed:
//!
//! | Row | `hash` | `key` |
//! | --- | --- | --- |
//! | regular media | `<sha256>` | `<sha[0..2]>/<sha[2..4]>/<sha256><ext>` |
//! | post cover | `.post.<post_id>.<sha256>` | `post/<uploader_id>/<sha256><ext>` |
//! | avatar | `.avt.<user_id>.<sha256>` | `avt/<uploader_id>/<sha256><ext>` |
//! | series cover | `.srs.<user_id>.<sha256>` | `srs/<uploader_id>/<sha256><ext>` |
//!
//! For the three special layouts the id embedded in the hash is the post or
//! user the file is *named* after, while the directory is the *uploader* id.
//! They are usually equal and are not guaranteed to be — which is why
//! `uploader_id` is a parameter here rather than something parsed out of the
//! hash.
//!
//! The key is store-independent: it says nothing about where the bytes sit
//! inside a store. The disk uses it as-is (`MEDIA_PATH.join(key)`); the bucket
//! prefixes it, and only for audio. One key, two stores for audio, no
//! translation to get wrong — which is what keeps the backfill a plain copy.
//!
//! ```text
//! audio, on disk   : MEDIA_PATH/b4/c0/<sha256>.mp3
//! audio, in bucket : retro-games/audio/b4/c0/<sha256>.mp3
//! an image         : MEDIA_PATH/b4/c0/<sha256>.webp   (disk only, forever)
//! ```

use std::str::FromStr;

use crate::domain::entities::media::MediaType;

/// A `media` row resolved to its storage location.
pub struct MediaKey {
    /// Store-relative, `/`-separated: the object key in the media bucket, and
    /// the path below `MEDIA_PATH` on disk.
    pub key: String,
    /// Plain SHA-256 hex digest of the bytes, independent of the layout. Used
    /// as the ETag so it stays stable whichever store answers the request.
    pub sha256: String,
}

/// Derives the storage key for a `media` row.
///
/// Returns `None` when `file_type` is not a known media type, or when `hash`
/// matches none of the layouts above — the row is unservable from either store
/// in both cases, so callers treat `None` as "not found" rather than an error.
pub fn media_key(hash: &str, file_type: &str, uploader_id: i64) -> Option<MediaKey> {
    let extension = MediaType::from_str(file_type).ok()?.get_extension();

    if hash.starts_with(".post.") || hash.starts_with(".avt.") || hash.starts_with(".srs.") {
        // ".<type>.<id>.<sha256>" — splitn(4, '.') yields ["", type, id, sha].
        // The sha256 tail holds no dots, so it survives in one piece.
        let mut parts = hash.splitn(4, '.');
        let _empty = parts.next()?;
        let type_dir = parts.next()?;
        let _id = parts.next()?;
        let sha256 = parts.next()?;
        if sha256.is_empty() {
            return None;
        }
        Some(MediaKey {
            key: format!("{type_dir}/{uploader_id}/{sha256}{extension}"),
            sha256: sha256.to_string(),
        })
    } else if hash.len() >= 4 && hash.bytes().all(|byte| byte.is_ascii_hexdigit()) {
        // The length and hexdigit checks are what keep the slicing below from
        // panicking on a short or multi-byte hash.
        Some(MediaKey {
            key: format!("{}/{}/{hash}{extension}", &hash[0..2], &hash[2..4]),
            sha256: hash.to_string(),
        })
    } else {
        None
    }
}

/// The prefix audio objects carry in the shared bucket.
///
/// Only audiobook audio leaves the disk, so this prefix is the whole of the
/// bucket layout: `v86/` holds the game artifacts, `audio/` holds the audio,
/// and nothing else is in the bucket at all. Images, covers, avatars, video and
/// models stay on `MEDIA_PATH` and are served from there.
///
/// It is deliberately *not* part of [`media_key`]: the key identifies an object
/// and is identical in both stores, while this prefix is a property of the
/// bucket. It has no counterpart on disk either, because `MEDIA_PATH` holds
/// nothing but media — unlike `PROJECT_DEMOS_PATH`, which is shared with the
/// demos and is why `v86/` appears in both stores.
pub const AUDIO_OBJECT_PREFIX: &str = "audio";

/// True when `content_type` is audio.
///
/// This is the **read** rule, and it is deliberately wider than the write rule.
/// The read path has only the row in front of it and never learns which module
/// wrote the file, so it cannot ask "does an audiobook track play this?" — the
/// content type is all it has. It is a real column; `description` is free text
/// (the table holds NULLs and `'undefined'` in it), so nothing there is
/// trustworthy.
///
/// The width is inert rather than wrong. Only audiobook track uploads mirror to
/// the bucket, and `backfill-audio.sh` only ever uploads the media an
/// `audiobook_tracks` row points at — so a media-library mp3 never has an
/// object, and this predicate matching it changes nothing: the request falls
/// through to the disk. What it buys is that where a row is served from is a
/// property of the row, not of which caller is asking.
pub fn is_audio(content_type: &str) -> bool {
    content_type.starts_with("audio/")
}

/// The bucket object key for an audio row: `audio/<key>`. `None` for everything
/// else.
///
/// `None` is not a failure, it is the scope. Only audio is stored in the bucket,
/// so a non-audio row simply has no object key — which is what makes every
/// caller (mirror, serve, sync, backfill) structurally unable to touch an image
/// rather than merely remembering not to. No caller needs an [`is_audio`] guard
/// around this.
pub fn audio_object_key(key: &str, content_type: &str) -> Option<String> {
    is_audio(content_type).then(|| format!("{AUDIO_OBJECT_PREFIX}/{key}"))
}

#[cfg(test)]
mod tests {
    use super::*;

    const SHA: &str = "b4c0ffee1234567890abcdef1234567890abcdef1234567890abcdef12345678";

    #[test]
    fn regular_layout_splits_the_hash_into_two_directories() {
        let key = media_key(SHA, "audio/mpeg", 7).expect("key");
        assert_eq!(key.key, format!("b4/c0/{SHA}.mp3"));
        assert_eq!(key.sha256, SHA);
    }

    #[test]
    fn post_cover_uses_the_uploader_directory_not_the_post_id() {
        // post_id 25, uploader 11 — the directory must be the uploader.
        let key = media_key(&format!(".post.25.{SHA}"), "image/webp", 11).expect("key");
        assert_eq!(key.key, format!("post/11/{SHA}.webp"));
        assert_eq!(key.sha256, SHA);
    }

    #[test]
    fn avatar_and_series_covers_use_their_own_directories() {
        assert_eq!(
            media_key(&format!(".avt.11.{SHA}"), "image/png", 11)
                .expect("key")
                .key,
            format!("avt/11/{SHA}.png")
        );
        assert_eq!(
            media_key(&format!(".srs.4.{SHA}"), "image/jpeg", 4)
                .expect("key")
                .key,
            format!("srs/4/{SHA}.jpeg")
        );
    }

    #[test]
    fn extension_comes_from_the_content_type() {
        assert!(
            media_key(SHA, "video/webm", 1)
                .expect("key")
                .key
                .ends_with(".webm")
        );
        assert!(
            media_key(SHA, "application/vnd.lottie+zip", 1)
                .expect("key")
                .key
                .ends_with(".lottie")
        );
    }

    #[test]
    fn unusable_rows_return_none_instead_of_panicking() {
        // Unknown content type.
        assert!(media_key(SHA, "application/octet-stream", 1).is_none());
        // Too short to slice into two directory levels (used to panic).
        assert!(media_key("ab", "image/png", 1).is_none());
        assert!(media_key("", "image/png", 1).is_none());
        // Not hex, so slicing could land mid-character.
        assert!(media_key("日本語日本語", "image/png", 1).is_none());
        // Truncated special layout.
        assert!(media_key(".post.25", "image/png", 1).is_none());
        assert!(media_key(".avt.11.", "image/png", 1).is_none());
    }

    #[test]
    fn only_audio_gets_an_object_key() {
        // The scope of the whole migration in one assertion: audio has somewhere
        // to go in the bucket, nothing else does.
        assert_eq!(
            audio_object_key(&format!("b4/c0/{SHA}.mp3"), "audio/mpeg"),
            Some(format!("audio/b4/c0/{SHA}.mp3"))
        );
        assert_eq!(
            audio_object_key(&format!("b4/c0/{SHA}.ogg"), "audio/ogg"),
            Some(format!("audio/b4/c0/{SHA}.ogg"))
        );

        assert_eq!(
            audio_object_key(&format!("b4/c0/{SHA}.webp"), "image/webp"),
            None
        );
        assert_eq!(
            audio_object_key(&format!("b4/c0/{SHA}.mp4"), "video/mp4"),
            None
        );
        assert_eq!(
            audio_object_key(&format!("post/11/{SHA}.webp"), "image/webp"),
            None
        );
        assert_eq!(
            audio_object_key(&format!("avt/11/{SHA}.png"), "image/png"),
            None
        );
    }

    #[test]
    fn the_object_key_is_the_disk_key_under_audio_and_nothing_else_changes() {
        // This is what keeps the disk tree where it is: the disk path is
        // `MEDIA_PATH.join(key)` and the object is that same key with a prefix,
        // so no audio file has to move and the backfill is a plain copy.
        let key = media_key(SHA, "audio/mpeg", 3).expect("key");
        let object = audio_object_key(&key.key, "audio/mpeg").expect("audio has an object key");

        assert_eq!(object, format!("audio/{}", key.key));
        assert!(object.ends_with(&key.key));
        assert!(object.ends_with(".mp3"));
    }

    #[test]
    fn an_audiobook_cover_gets_no_object_key() {
        // A file that belongs to an audiobook but is not audio. Routing on "is
        // this audiobook-related" would send it to the bucket.
        assert_eq!(
            audio_object_key(&format!("post/11/{SHA}.webp"), "image/webp"),
            None
        );
    }

    #[test]
    fn every_non_audio_type_in_the_media_table_gets_no_object_key() {
        // The distinct `file_type` values actually present in `media` that are
        // not audio. Every one of these stays on disk.
        for content_type in [
            "application/vnd.lottie+zip",
            "image/gif",
            "image/jpeg",
            "image/png",
            "image/webp",
            "model/gltf-binary",
            "video/mp4",
            "video/webm",
        ] {
            assert_eq!(
                audio_object_key("aa/bb/c.png", content_type),
                None,
                "{content_type} must not be stored in the bucket"
            );
            assert!(!is_audio(content_type));
        }
    }

    #[test]
    fn the_audio_test_needs_its_slash() {
        // Guards against a prefix test that would also swallow a type like
        // `audiobook/...`.
        assert!(!is_audio("audiobook/track"));
        assert!(!is_audio("audio"));
        assert!(!is_audio(""));
    }
}

//! Mirroring audiobook audio into the bucket.
//!
//! The disk copy is written first, by the audiobook track path; when
//! `AUDIO_BACKEND=r2` the same bytes also go to the bucket under the same key
//! with the `audio/` prefix (see `storage::audio_object_key`), so either store
//! can serve a play and the two stay interchangeable.
//!
//! Nothing else is mirrored. Images, covers, avatars, video and models have no
//! object key at all, and the media library upload — which *can* carry an audio
//! file — does not call this, so an mp3 uploaded there stays on disk.

use std::path::Path;

use crate::infrastructure::storage::{MEDIA_CACHE_CONTROL, audio_object_key, media_key};
use crate::infrastructure::web::server::MediaConfig;

/// Copies a just-written audio file into the bucket.
///
/// Called from the audiobook track write path and nowhere else, which is what
/// makes "audiobook audio" the write rule: the media library upload does not
/// call it, so an mp3 uploaded there stays on disk. The early return for a
/// non-audio content type is a second line of defence for that call site — the
/// medium is validated as audio before it arrives — rather than the mechanism.
///
/// `wrote_file` is the caller's own dedup result. When it is false the bytes
/// were already on disk, and content addressing means the object is normally
/// already in the bucket too — but "normally" is not "always", since a row can
/// predate the bucket, so this re-checks with a HEAD rather than assuming.
///
/// Failures are logged, not propagated: while the disk copy is still
/// authoritative a failed mirror costs a bucket gap and nothing else, and
/// `backfill-audio.sh` reports exactly that. Make this fatal in the same change
/// that removes the disk copy.
pub async fn mirror_audio_to_bucket(
    config: &MediaConfig,
    hash: &str,
    content_type: &str,
    uploader_id: i64,
    file_path: &Path,
    wrote_file: bool,
) {
    let Some(bucket) = &config.audio_bucket else {
        return;
    };
    let Some(key) = media_key(hash, content_type, uploader_id) else {
        tracing::warn!(
            hash,
            content_type,
            "audio: no storage key derivable, nothing mirrored"
        );
        return;
    };
    // Not audio => nothing to do. This is the scope, not a failure.
    let Some(object) = audio_object_key(&key.key, content_type) else {
        return;
    };

    if !wrote_file {
        match bucket.client.object_size(&object).await {
            Ok(Some(_)) => return,
            Ok(None) => {}
            Err(error) => {
                tracing::error!(key = %object, %error, "audio: bucket lookup failed");
                return;
            }
        }
    }

    match bucket
        .client
        .put_file_with_metadata(&object, file_path, content_type, MEDIA_CACHE_CONTROL)
        .await
    {
        Ok(()) => tracing::info!(key = %object, "audio: mirrored to bucket"),
        Err(error) => tracing::error!(
            key = %object,
            path = %file_path.display(),
            %error,
            "audio: bucket mirror failed; the disk copy is still authoritative"
        ),
    }
}

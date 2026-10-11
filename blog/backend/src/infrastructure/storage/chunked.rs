//! Audiobook audio, stored as independently compressed zstd chunks.
//!
//! A chapter is split into [`CHUNK_SIZE`] pieces; each is compressed at zstd
//! level 19 on its own and written as its own object, and a small JSON manifest
//! next to them records what was produced — per chunk its raw length, its
//! compressed length and the SHA-256 of the *raw* bytes.
//!
//! Two things follow from compressing each chunk on its own, and both are the
//! reason for the layout:
//!
//! * a reader that wants one byte range reads only the chunks covering it, and
//!   never the whole chapter, so a seek costs one chunk rather than a file;
//! * a level-19 encoder only ever sees [`CHUNK_SIZE`] of input, which bounds
//!   both its memory and its window — relevant because this runs on a 256 MB
//!   VM (see `open_media_file`'s note about OOM kills).
//!
//! The cost is ratio: an independent frame cannot reference the previous one,
//! so chunking gives up a little against compressing the file whole. Measured
//! on the library's own audio (64 kbps mono mp3, already entropy coded) 1 MiB
//! chunks land within 0.2 points of the whole-file number, and both are ~2%.
//!
//! The manifest is the switch. A row with one is chunked and is served from its
//! chunks; a row without one is a plain file and is served exactly as it was
//! before this module existed, which is what makes the migration per-file
//! rather than all-or-nothing.
//!
//! Where the chunks live mirrors the plain file: the disk path is
//! `MEDIA_PATH.join(chunk_key(key, index))` and, for audio on the bucket, the
//! object is that same key under the `audio/` prefix.

use std::path::{Path, PathBuf};

use futures::{Stream, stream};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use tokio::fs;

use super::{
    MEDIA_CACHE_CONTROL, StorageError, audio_object_key,
    media_key::{chunk_dir, chunk_key, manifest_key},
};
use crate::infrastructure::storage::r2::R2Client;
use crate::infrastructure::web::server::MediaConfig;

/// Raw bytes per chunk. Sized so a seek reads about a megabyte, and so a
/// level-19 window stays small on a small machine.
pub const CHUNK_SIZE: u64 = 1024 * 1024;

/// The encoder level. Level 19 is the slowest setting zstd offers, and is
/// affordable only because it runs over a megabyte at a time and off the
/// request path entirely (the client is uploading while it happens).
pub const ZSTD_LEVEL: i32 = 19;

/// Bumped if the manifest's meaning changes. A reader that does not know a
/// version treats the row as unchunked rather than guessing.
pub const MANIFEST_VERSION: u32 = 1;

/// Content type the compressed chunks are stored and served as.
pub const CHUNK_CONTENT_TYPE: &str = "application/zstd";

/// One chunk, as the manifest records it.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ChunkEntry {
    /// SHA-256 of the chunk's *raw* bytes, so a reader can verify what it
    /// decompressed and a re-upload of the same audio can recognise its chunks.
    pub sha256: String,
    /// Length of the chunk as stored (the zstd frame).
    pub compressed: u64,
    /// Length of the chunk after decompression.
    pub raw: u64,
}

/// A chapter's chunk table. Everything a reader needs to turn a byte offset
/// into a chunk, and to reassemble the file, without touching the audio.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ChunkManifest {
    pub version: u32,
    pub chunk_size: u64,
    /// Length of the whole chapter once reassembled. This is the number a
    /// `Content-Range` is expressed against, so it must be the raw length.
    pub raw_size: u64,
    pub level: i32,
    /// The media type of the *reassembled* file, not of a chunk.
    pub content_type: String,
    pub chunks: Vec<ChunkEntry>,
}

impl ChunkManifest {
    /// The raw byte offset the chunk at `index` starts at.
    pub fn raw_offset(&self, index: usize) -> u64 {
        self.chunks.iter().take(index).map(|chunk| chunk.raw).sum()
    }

    /// The chunk holding raw byte `offset`.
    ///
    /// Linear rather than binary searched: a chapter is tens of chunks, and the
    /// table is rebuilt from lengths the manifest already holds, so there is no
    /// prefix sum to keep in sync with the entries.
    pub fn chunk_at(&self, offset: u64) -> usize {
        let mut end = 0_u64;
        for (index, chunk) in self.chunks.iter().enumerate() {
            end += chunk.raw;
            if offset < end {
                return index;
            }
        }
        // Past the last byte: answered with the last chunk, which is how the
        // caller's own clamping already treats an over-long range.
        self.chunks.len().saturating_sub(1)
    }

    /// True when the table describes exactly `self.raw_size` bytes. A manifest
    /// that fails this is treated as absent, so a truncated write can never be
    /// served as if it were whole.
    pub fn is_consistent(&self) -> bool {
        !self.chunks.is_empty()
            && self.chunks.iter().map(|chunk| chunk.raw).sum::<u64>() == self.raw_size
    }
}

/// Splits a chapter into compressed chunks, writes them, and writes the
/// manifest that describes them.
///
/// The chunk objects are written first and the manifest last, so the switch
/// that says "this row is chunked" is only ever set once the chunks it names
/// are already in place. A failure part-way therefore leaves a complete plain
/// file and no manifest — a row that has not been chunked yet — rather than a
/// manifest pointing at chunks that are not there.
pub async fn write(
    config: &MediaConfig,
    key: &str,
    content_type: &str,
    bytes: &[u8],
) -> Result<ChunkManifest, StorageError> {
    let mut chunks = Vec::new();

    for (index, raw) in bytes.chunks(CHUNK_SIZE as usize).enumerate() {
        let compressed = zstd::bulk::compress(raw, ZSTD_LEVEL)
            .map_err(|e| StorageError(format!("chunk {index} of {key}: compress: {e}")))?;

        write_chunk_object(config, &chunk_key(key, index), content_type, &compressed).await?;

        chunks.push(ChunkEntry {
            sha256: hex::encode(Sha256::digest(raw)),
            compressed: compressed.len() as u64,
            raw: raw.len() as u64,
        });
    }

    let manifest = ChunkManifest {
        version: MANIFEST_VERSION,
        chunk_size: CHUNK_SIZE,
        raw_size: bytes.len() as u64,
        level: ZSTD_LEVEL,
        content_type: content_type.to_string(),
        chunks,
    };

    let encoded = serde_json::to_vec(&manifest)
        .map_err(|e| StorageError(format!("manifest for {key}: serialize: {e}")))?;
    write_chunk_object(config, &manifest_key(key), content_type, &encoded).await?;

    Ok(manifest)
}

/// The manifest for a chunked row, or `None` when the row is not chunked.
///
/// `None` is the ordinary answer, not a failure: every row written before this
/// module existed has no manifest, and a truncated or unknown-version manifest
/// is deliberately reported the same way so the caller falls back to the plain
/// file rather than serving a broken reassembly.
pub async fn read_manifest(
    config: &MediaConfig,
    key: &str,
    content_type: &str,
) -> Option<ChunkManifest> {
    let encoded = match read_chunk_object(config, &manifest_key(key), content_type).await {
        Ok(Some(encoded)) => encoded,
        Ok(None) => return None,
        Err(error) => {
            tracing::error!(key, %error, "chunks: manifest could not be read");
            return None;
        }
    };

    let manifest: ChunkManifest = match serde_json::from_slice(&encoded) {
        Ok(manifest) => manifest,
        Err(error) => {
            // Logged, not returned: a manifest this module cannot parse is
            // indistinguishable, from the request's point of view, from a row
            // that was never chunked.
            tracing::error!(key, %error, "chunks: manifest is not readable");
            return None;
        }
    };

    if manifest.version != MANIFEST_VERSION {
        tracing::error!(
            key,
            version = manifest.version,
            "chunks: manifest is a version this build does not know"
        );
        return None;
    }
    if !manifest.is_consistent() {
        tracing::error!(
            key,
            raw_size = manifest.raw_size,
            "chunks: manifest does not describe its own length"
        );
        return None;
    }

    Some(manifest)
}

/// A stream of the *raw* bytes of a chapter between `start` and `end`, both
/// inclusive, assembled from the chunks covering that span.
///
/// Chunks are read and decompressed one at a time as the body is pulled, so a
/// whole-chapter response never holds a whole chapter in memory — the same
/// reason the plain path streams a file rather than reading it.
pub fn range_stream(
    dir: PathBuf,
    client: Option<R2Client>,
    key: String,
    content_type: String,
    manifest: ChunkManifest,
    start: u64,
    end: u64,
) -> impl Stream<Item = Result<Vec<u8>, StorageError>> + Send + 'static {
    stream::try_unfold(start, move |cursor| {
        let dir = dir.clone();
        let client = client.clone();
        let key = key.clone();
        let content_type = content_type.clone();
        let manifest = manifest.clone();

        async move {
            if cursor > end {
                return Ok(None);
            }

            let index = manifest.chunk_at(cursor);
            let entry = manifest.chunks[index].clone();
            let chunk_start = manifest.raw_offset(index);
            let chunk_end = chunk_start + entry.raw - 1;

            let raw = read_chunk(&dir, client.as_ref(), &key, &content_type, index, &entry).await?;

            // Clamp to the requested span: the first and last chunk are usually
            // read only in part.
            let from = cursor.saturating_sub(chunk_start) as usize;
            let to = (end.min(chunk_end) - chunk_start) as usize;
            let slice = raw
                .get(from..=to.min(raw.len().saturating_sub(1)))
                .ok_or_else(|| {
                    StorageError(format!(
                        "chunk {index} of {key}: decompressed {} bytes, needed {}..={to}",
                        raw.len(),
                        from
                    ))
                })?
                .to_vec();

            // The next cursor is one past this chunk's end, not one past the
            // slice, so a partial read of the first chunk still advances to the
            // next one whole.
            Ok(Some((slice, chunk_end + 1)))
        }
    })
}

/// Removes a chunked row's bytes: every chunk, then the manifest.
///
/// The manifest goes last so an interrupted delete leaves the row readable for
/// as long as any of it is still there. Failures are logged rather than
/// returned, matching `discard_medium_files`: the write that needed the space
/// has already committed.
///
/// `content_type` is the *chapter's* type, and is what decides whether these
/// objects have a bucket key at all — only audio does. What each object is
/// stored *as* is a different question, answered by `stored_content_type`.
pub async fn discard(
    config: &MediaConfig,
    key: &str,
    content_type: &str,
    manifest: &ChunkManifest,
) {
    for index in 0..manifest.chunks.len() {
        remove_chunk_object(config, &chunk_key(key, index), content_type).await;
    }
    remove_chunk_object(config, &manifest_key(key), content_type).await;

    // The directory is left behind only when it is not empty, which would mean
    // something is still in there and worth keeping.
    let directory = config.dir.join(chunk_dir(key));
    match fs::remove_dir(&directory).await {
        Ok(()) => {}
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
        Err(error) => tracing::warn!(
            directory = %directory.display(),
            %error,
            "chunks: chunk directory could not be removed"
        ),
    }
}

/// Writes one chunk object: the disk copy always, the bucket copy when audio is
/// configured to live there.
///
/// The bucket write goes through `put_file_with_metadata` rather than
/// `put_object_bytes` because a chunk handed to a client by URL is served by the
/// bucket itself, so the object has to carry its own content type and the
/// immutable cache header.
async fn write_chunk_object(
    config: &MediaConfig,
    object_key: &str,
    content_type: &str,
    bytes: &[u8],
) -> Result<(), StorageError> {
    let path = config.dir.join(object_key);
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).await.map_err(|e| {
            StorageError(format!("{}: create {}: {e}", object_key, parent.display()))
        })?;
    }
    fs::write(&path, bytes)
        .await
        .map_err(|e| StorageError(format!("{object_key}: write {}: {e}", path.display())))?;

    if let Some(bucket) = &config.audio_bucket
        && let Some(object) = chunk_object_key(object_key, content_type)
    {
        bucket
            .client
            .put_file_with_metadata(
                &object,
                &path,
                stored_content_type(object_key),
                MEDIA_CACHE_CONTROL,
            )
            .await
            .map_err(|e| StorageError(format!("{object_key}: bucket write: {e}")))?;
    }

    Ok(())
}

/// Reads one chunk object, preferring the bucket when audio lives there so the
/// two stores stay interchangeable exactly as they are for the plain file.
async fn read_chunk_object(
    config: &MediaConfig,
    object_key: &str,
    content_type: &str,
) -> Result<Option<Vec<u8>>, StorageError> {
    if let Some(bucket) = &config.audio_bucket
        && let Some(object) = audio_object_key(object_key, content_type)
    {
        match bucket.client.get_object(&object).await {
            Ok(bytes) => return Ok(Some(bytes)),
            Err(error) => {
                // Not fatal: the disk copy is still authoritative during the
                // migration, so a bucket that does not hold this object yet is
                // the ordinary case for a row written before it was mirrored.
                tracing::debug!(%object, %error, "chunks: bucket read failed, trying the disk");
            }
        }
    }

    match fs::read(config.dir.join(object_key)).await {
        Ok(bytes) => Ok(Some(bytes)),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(None),
        Err(error) => Err(StorageError(format!("{object_key}: read: {error}"))),
    }
}

/// Reads one chunk's compressed bytes, from either store.
///
/// The stored length is checked against the manifest before returning, so a
/// truncated object is reported as a truncated object rather than as a zstd
/// error from deep inside the frame — or, worse, handed to a client that would
/// have to notice for itself.
async fn fetch_chunk_bytes(
    dir: &Path,
    client: Option<&R2Client>,
    key: &str,
    content_type: &str,
    index: usize,
    entry: &ChunkEntry,
) -> Result<Vec<u8>, StorageError> {
    let object_key = chunk_key(key, index);

    let compressed = match client.and_then(|_| audio_object_key(&object_key, content_type)) {
        Some(object) => match client.expect("checked above").get_object(&object).await {
            Ok(bytes) => bytes,
            Err(error) => {
                tracing::debug!(%object, %error, "chunks: bucket chunk read failed, trying the disk");
                read_disk_chunk(dir, &object_key).await?
            }
        },
        None => read_disk_chunk(dir, &object_key).await?,
    };

    if compressed.len() as u64 != entry.compressed {
        return Err(StorageError(format!(
            "{object_key}: stored {} bytes, manifest says {}",
            compressed.len(),
            entry.compressed
        )));
    }

    Ok(compressed)
}

/// The chunk at `index`, decompressed.
async fn read_chunk(
    dir: &Path,
    client: Option<&R2Client>,
    key: &str,
    content_type: &str,
    index: usize,
    entry: &ChunkEntry,
) -> Result<Vec<u8>, StorageError> {
    let compressed = fetch_chunk_bytes(dir, client, key, content_type, index, entry).await?;

    zstd::bulk::decompress(&compressed, entry.raw as usize)
        .map_err(|e| StorageError(format!("{}: decompress: {e}", chunk_key(key, index))))
}

/// One chunk's compressed bytes exactly as stored, for a caller that decompresses
/// it itself — the browser player runs its own zstd decoder, so what it wants
/// over the wire is the frame rather than the audio.
///
/// Each chunk is an independent zstd frame, which is what makes this possible: a
/// client can fetch any chunk on its own, in any order, without the ones before
/// it. That is also why a client never has to hold a whole chapter in its
/// decoder — it decodes a chunk and moves on.
pub async fn compressed_chunk(
    config: &MediaConfig,
    key: &str,
    content_type: &str,
    index: usize,
    entry: &ChunkEntry,
) -> Result<Vec<u8>, StorageError> {
    fetch_chunk_bytes(
        &config.dir,
        config.audio_bucket.as_ref().map(|bucket| &bucket.client),
        key,
        content_type,
        index,
        entry,
    )
    .await
}

async fn read_disk_chunk(dir: &Path, object_key: &str) -> Result<Vec<u8>, StorageError> {
    fs::read(dir.join(object_key))
        .await
        .map_err(|e| StorageError(format!("{object_key}: read: {e}")))
}

async fn remove_chunk_object(config: &MediaConfig, object_key: &str, content_type: &str) {
    let path = config.dir.join(object_key);
    match fs::remove_file(&path).await {
        Ok(()) => {}
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
        Err(error) => tracing::error!(
            path = %path.display(),
            %error,
            "chunks: chunk file could not be deleted"
        ),
    }

    if let Some(bucket) = &config.audio_bucket
        && let Some(object) = chunk_object_key(object_key, content_type)
        && let Err(error) = bucket.client.delete_object(&object).await
    {
        tracing::error!(%object, %error, "chunks: chunk object could not be deleted");
    }
}

/// The bucket object a chunk or manifest is stored at, under the same `audio/`
/// prefix as the chapter it belongs to — which is what keeps them together for
/// the bucket's lifecycle rules.
///
/// `content_type` is the chapter's, not the object's, and that distinction is
/// load-bearing: [`audio_object_key`] keys on "is this audio", so passing a
/// chunk's own stored type (`application/zstd`) makes it return `None` and the
/// bucket write silently never happens. Everything stays on the disk, the
/// bucket looks half migrated, and a deployment that reads from the bucket
/// fails every play. `chunks_are_stored_under_the_audio_prefix` is the test
/// that keeps that from coming back.
fn chunk_object_key(object_key: &str, content_type: &str) -> Option<String> {
    audio_object_key(object_key, content_type)
}

/// What an object is stored *as*: a chunk is a zstd frame, the manifest is JSON.
/// Only the key says which is which, because both sit under the chapter's key.
fn stored_content_type(object_key: &str) -> &'static str {
    if object_key.ends_with(".zm") {
        "application/json"
    } else {
        CHUNK_CONTENT_TYPE
    }
}

/// A directory that only this module's tests use.
#[cfg(test)]
fn test_dir(name: &str) -> PathBuf {
    std::env::temp_dir().join(format!("chunked-{name}-{}", uuid::Uuid::new_v4().simple()))
}

#[cfg(test)]
fn test_config(name: &str) -> MediaConfig {
    MediaConfig {
        dir: test_dir(name),
        allowed_file_types: Vec::new(),
        allowed_avatar_types: Vec::new(),
        allowed_cover_types: Vec::new(),
        allowed_audio_types: Vec::new(),
        audio_bucket: None,
    }
}

#[cfg(test)]
async fn collect(
    stream: impl Stream<Item = Result<Vec<u8>, StorageError>>,
) -> Result<Vec<u8>, StorageError> {
    use futures::StreamExt;

    let parts: Vec<Vec<u8>> = stream
        .collect::<Vec<_>>()
        .await
        .into_iter()
        .collect::<Result<_, _>>()?;
    Ok(parts.concat())
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Bytes that compress neither perfectly nor not at all, so a chunk is
    /// exercised as bytes rather than as a run of zeros.
    fn pseudo_audio(len: usize) -> Vec<u8> {
        let mut state = 0x2545_F491_4F6C_DD1D_u64;
        (0..len)
            .map(|_| {
                state = state
                    .wrapping_mul(6364136223846793005)
                    .wrapping_add(1442695040888963407);
                (state >> 33) as u8
            })
            .collect()
    }

    fn stream_of(
        config: &MediaConfig,
        key: &str,
        manifest: &ChunkManifest,
        start: u64,
        end: u64,
    ) -> impl Stream<Item = Result<Vec<u8>, StorageError>> + Send + 'static {
        range_stream(
            config.dir.clone(),
            None,
            key.to_string(),
            "audio/mpeg".to_string(),
            manifest.clone(),
            start,
            end,
        )
    }

    #[tokio::test]
    async fn chunks_round_trip_through_the_manifest() {
        let config = test_config("round-trip");
        let key = "b4/c0/deadbeef.mp3";
        // Two and a bit chunks, so the last chunk is a short one.
        let raw = pseudo_audio(CHUNK_SIZE as usize * 2 + 1234);

        let manifest = write(&config, key, "audio/mpeg", &raw)
            .await
            .expect("write");

        assert_eq!(manifest.version, MANIFEST_VERSION);
        assert_eq!(manifest.raw_size, raw.len() as u64);
        assert_eq!(manifest.chunks.len(), 3);
        assert_eq!(manifest.chunks[2].raw, 1234);
        assert!(manifest.is_consistent());

        let read = read_manifest(&config, key, "audio/mpeg")
            .await
            .expect("manifest");
        assert_eq!(read, manifest);

        let whole = collect(stream_of(&config, key, &read, 0, read.raw_size - 1))
            .await
            .expect("whole chapter");
        assert_eq!(
            whole, raw,
            "the reassembled chapter must be the original bytes"
        );

        std::fs::remove_dir_all(&config.dir).ok();
    }

    #[tokio::test]
    async fn a_range_reads_only_the_chunks_covering_it() {
        let config = test_config("range");
        let key = "b4/c0/feedface.mp3";
        let raw = pseudo_audio(CHUNK_SIZE as usize * 3);
        let manifest = write(&config, key, "audio/mpeg", &raw)
            .await
            .expect("write");

        // A window that starts inside the second chunk and ends inside the
        // third, which is the shape of a seek.
        let start = CHUNK_SIZE + 4096;
        let end = CHUNK_SIZE * 2 + 512;
        let served = collect(stream_of(&config, key, &manifest, start, end))
            .await
            .expect("range");

        assert_eq!(served.len() as u64, end - start + 1);
        assert_eq!(served, raw[start as usize..=end as usize]);

        // A window wholly inside one chunk.
        let served = collect(stream_of(&config, key, &manifest, 10, 99))
            .await
            .expect("range");
        assert_eq!(served, raw[10..=99]);

        // The last byte, which is the edge the cursor advance is easiest to get
        // wrong on.
        let last = manifest.raw_size - 1;
        let served = collect(stream_of(&config, key, &manifest, last, last))
            .await
            .expect("range");
        assert_eq!(served, vec![raw[last as usize]]);

        std::fs::remove_dir_all(&config.dir).ok();
    }

    #[test]
    fn the_manifest_maps_offsets_onto_chunks() {
        let manifest = ChunkManifest {
            version: MANIFEST_VERSION,
            chunk_size: 3,
            raw_size: 8,
            level: ZSTD_LEVEL,
            content_type: "audio/mpeg".to_string(),
            chunks: vec![
                ChunkEntry {
                    sha256: "a".repeat(64),
                    compressed: 3,
                    raw: 3,
                },
                ChunkEntry {
                    sha256: "b".repeat(64),
                    compressed: 3,
                    raw: 3,
                },
                ChunkEntry {
                    sha256: "c".repeat(64),
                    compressed: 2,
                    raw: 2,
                },
            ],
        };

        assert_eq!(manifest.raw_offset(0), 0);
        assert_eq!(manifest.raw_offset(1), 3);
        assert_eq!(manifest.raw_offset(2), 6);
        assert_eq!(manifest.chunk_at(0), 0);
        assert_eq!(manifest.chunk_at(2), 0);
        assert_eq!(manifest.chunk_at(3), 1);
        assert_eq!(manifest.chunk_at(5), 1);
        assert_eq!(manifest.chunk_at(6), 2);
        assert_eq!(manifest.chunk_at(7), 2);
        // Past the end clamps to the last chunk rather than panicking.
        assert_eq!(manifest.chunk_at(9999), 2);
        assert!(manifest.is_consistent());

        let mut short = manifest.clone();
        short.raw_size = 9;
        assert!(
            !short.is_consistent(),
            "a table that names more bytes than it holds is rejected"
        );
    }

    #[test]
    fn chunks_are_stored_under_the_audio_prefix_not_their_own_type() {
        // The regression this exists for: deriving the prefix from the object's
        // *stored* type (`application/zstd`) rather than the chapter's makes
        // `audio_object_key` answer `None`, so every chunk stayed on the disk
        // while the bucket looked migrated.
        let chapter = "b4/c0/deadbeef.mp3";

        assert_eq!(
            chunk_object_key(&chunk_key(chapter, 3), "audio/mpeg"),
            Some("audio/b4/c0/deadbeef.mp3.d/00003.zst".to_string())
        );
        assert_eq!(
            chunk_object_key(&manifest_key(chapter), "audio/ogg"),
            Some("audio/b4/c0/deadbeef.mp3.zm".to_string())
        );
        // Not audio, so not in the bucket — the scope the whole module shares.
        assert_eq!(chunk_object_key(&chunk_key(chapter, 0), "image/webp"), None);
        assert_eq!(chunk_object_key(&manifest_key(chapter), "video/mp4"), None);
    }

    #[test]
    fn a_chunk_is_stored_as_zstd_and_the_manifest_as_json() {
        assert_eq!(
            stored_content_type("b4/c0/x.mp3.d/00000.zst"),
            CHUNK_CONTENT_TYPE
        );
        assert_eq!(stored_content_type("b4/c0/x.mp3.zm"), "application/json");
    }

    #[tokio::test]
    async fn a_row_without_a_manifest_reads_as_unchunked() {
        // Every row written before this module existed has no manifest, and a
        // row that is not chunked must cost the plain path nothing.
        let config = test_config("no-manifest");
        assert!(
            read_manifest(&config, "b4/c0/absent.mp3", "audio/mpeg")
                .await
                .is_none()
        );

        // Nor is a manifest that cannot be parsed an error the caller has to
        // handle separately: it reads as unchunked, and the plain file answers.
        let key = "b4/c0/garbled.mp3";
        write_chunk_object(&config, &manifest_key(key), "audio/mpeg", b"not json")
            .await
            .expect("write");
        assert!(read_manifest(&config, key, "audio/mpeg").await.is_none());

        std::fs::remove_dir_all(&config.dir).ok();
    }

    #[tokio::test]
    async fn a_truncated_chunk_is_reported_rather_than_served() {
        let config = test_config("truncated");
        let key = "b4/c0/truncated.mp3";
        let raw = pseudo_audio(4096);
        let manifest = write(&config, key, "audio/mpeg", &raw)
            .await
            .expect("write");

        // Simulates a half-written object: zstd would answer with a frame error
        // from inside the stream, which says nothing about the actual problem.
        fs::write(config.dir.join(chunk_key(key, 0)), b"short")
            .await
            .expect("truncate");

        let error = collect(stream_of(&config, key, &manifest, 0, 9))
            .await
            .expect_err("a truncated chunk must not be served");
        assert!(
            error.0.contains("manifest says"),
            "the error should name the size mismatch, got: {}",
            error.0
        );

        std::fs::remove_dir_all(&config.dir).ok();
    }

    #[tokio::test]
    async fn discarding_removes_every_chunk_and_the_manifest() {
        let config = test_config("discard");
        let key = "b4/c0/discardme.mp3";
        let raw = pseudo_audio(2048);
        let manifest = write(&config, key, "audio/mpeg", &raw)
            .await
            .expect("write");

        discard(&config, key, "audio/mpeg", &manifest).await;

        assert!(!config.dir.join(manifest_key(key)).exists());
        for index in 0..manifest.chunks.len() {
            assert!(!config.dir.join(chunk_key(key, index)).exists());
        }
        assert!(!config.dir.join(chunk_dir(key)).exists());
        assert!(read_manifest(&config, key, "audio/mpeg").await.is_none());

        std::fs::remove_dir_all(&config.dir).ok();
    }
}

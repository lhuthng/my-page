// Content hashing and deterministic dir/name layout for stored media.
use std::path::PathBuf;

use axum::body::Bytes;
use sha2::{Digest, Sha256};

use crate::domain::errors::media::MediaError;

pub struct HashData {
    pub hash: String,
    pub size: i64,
    pub dir_path: PathBuf,
    pub file_path: PathBuf,
}

pub(super) fn generate_dir_and_name(
    root: &PathBuf,
    hash: &String,
    extension: String,
    split: bool,
) -> (PathBuf, String) {
    let dir_path = match split {
        true => {
            let dir1 = &hash[0..2];
            let dir2 = &hash[2..4];
            &root.join(dir1).join(dir2)
        }
        false => root,
    };

    let filename = format!("{}{}", hash, extension);

    (dir_path.to_path_buf(), filename)
}

pub async fn hash_bytes(
    bytes: &Bytes,
    root: &PathBuf,
    extension: String,
    split: bool,
) -> Result<HashData, MediaError> {
    // Uploads reach 100 MB, so hash on the blocking pool instead of pinning
    // an async worker for the whole digest.
    let owned_bytes = bytes.clone();
    let hash = tokio::task::spawn_blocking(move || format!("{:x}", Sha256::digest(&owned_bytes)))
        .await
        .map_err(|e| MediaError::InternalError(e.to_string()))?;
    if hash.len() < 4 {
        return Err(MediaError::InternalError("Hash too short.".to_string()));
    }

    let (dir_path, file_name) = generate_dir_and_name(root, &hash, extension, split);

    let file_path = dir_path.join(file_name);
    let size = bytes.len() as i64;

    Ok(HashData {
        hash,
        size,
        dir_path,
        file_path,
    })
}

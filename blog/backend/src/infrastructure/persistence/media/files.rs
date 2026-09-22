// Multi-file cleanup used by replace/delete flows.
use std::path::PathBuf;

use futures::future::join_all;
use tokio::fs;

use crate::domain::errors::media::MediaError;

pub async fn clean_up_files(file_paths: &[PathBuf]) -> Result<(), MediaError> {
    let futures = file_paths.iter().map(fs::remove_file);
    let mut errors: Vec<String> = Vec::new();
    let results = join_all(futures).await;
    for result in results {
        if let Some(e) = result.err() {
            errors.push(e.to_string());
        }
    }
    if !errors.is_empty() {
        let msg = errors.join(" | ");
        return Err(MediaError::UploadFailed(msg));
    }
    Ok(())
}

// Shared plumbing for v86 upload sessions: the chunked multipart relay,
// part bookkeeping, and the source-asset splitter used by tests.
use std::{
    fs::{self, File},
    io::{Read, Write},
    path::Path,
    sync::Mutex,
};

use axum::body::Bytes;
use sqlx::Row;

use crate::domain::errors::project::ProjectError;
use crate::infrastructure::web::server::AppState;

use super::dto::{ChunkProgress, ChunkUploadResponse};
use super::shared::{ensure_upload_not_expired, storage_error};

pub(super) fn transient_storage_key(kind: &str, upload_id: &str, extension: &str) -> String {
    format!("v86/tmp/{kind}/{upload_id}.{extension}")
}

pub(super) fn parse_part_etags(etags: Option<&str>) -> Vec<(i32, String)> {
    match etags {
        Some(text) if !text.is_empty() => serde_json::from_str::<Vec<String>>(text)
            .unwrap_or_default()
            .into_iter()
            .enumerate()
            .map(|(index, etag)| ((index as i32) + 1, etag))
            .collect(),
        _ => Vec::new(),
    }
}

fn append_part_etag(existing: Option<&str>, etag: &str) -> String {
    let mut etags: Vec<String> = match existing {
        Some(text) if !text.is_empty() => serde_json::from_str(text).unwrap_or_default(),
        _ => Vec::new(),
    };
    etags.push(etag.to_string());
    serde_json::to_string(&etags).unwrap_or_else(|_| "[]".to_string())
}

#[allow(dead_code)]
pub(super) fn split_asset(
    source: &Path,
    destination: &Path,
    chunk_size: u64,
    extension: &str,
    progress: Option<&Mutex<ChunkProgress>>,
    compression_level: i32,
) -> Result<u64, ProjectError> {
    fs::create_dir_all(destination)?;
    let mut input = File::open(source)?;
    let file_len = input.metadata()?.len();
    let total_chunks = file_len.div_ceil(chunk_size);
    if let Some(p) = progress {
        let mut p = p.lock().unwrap();
        p.total_chunks = total_chunks;
        p.completed_chunks = 0;
        p.message = format!("Compressing chunk 0/{total_chunks}");
    }
    let mut start = 0_u64;
    let mut count = 0_u64;
    let mut buffer = vec![0_u8; chunk_size as usize];
    loop {
        let read = input.read(&mut buffer)?;
        if read == 0 {
            break;
        }
        if read < buffer.len() {
            buffer[read..].fill(0);
        }
        let end = start + chunk_size;
        let part_path = destination.join(format!("{start}-{end}.{extension}"));
        let output = File::create(&part_path)?;
        let mut encoder = zstd::stream::write::Encoder::new(output, compression_level)?;
        encoder.write_all(&buffer)?;
        encoder.finish()?;
        start = end;
        count += 1;
        if let Some(p) = progress {
            let mut p = p.lock().unwrap();
            p.completed_chunks = count;
            p.message = format!("Compressing chunk {count}/{total_chunks}");
        }
    }
    if let Some(p) = progress {
        let mut p = p.lock().unwrap();
        p.completed_chunks = count;
        p.total_chunks = total_chunks;
        p.message = format!("Compressing chunk {total_chunks}/{total_chunks}");
    }
    Ok(count)
}

pub(super) async fn append_upload_chunk(
    state: &AppState,
    table: &str,
    upload_id: &str,
    uploader_id: i64,
    chunk_index: u64,
    bytes: Bytes,
) -> Result<ChunkUploadResponse, ProjectError> {
    let query = format!(
        "SELECT expected_size_bytes, received_size_bytes, next_chunk_index, upload_chunk_size_bytes, temp_storage_key, r2_upload_id, r2_part_etags, status, expires_at FROM {table} WHERE id = ? AND uploader_id = ?"
    );
    let row = sqlx::query(&query)
        .bind(upload_id)
        .bind(uploader_id)
        .fetch_optional(&state.project_service.pool)
        .await?
        .ok_or(ProjectError::ProjectNotFound)?;
    let status: String = row.get("status");
    let expected: i64 = row.get("expected_size_bytes");
    let received: i64 = row.get("received_size_bytes");
    let next: i64 = row.get("next_chunk_index");
    let chunk_size: i64 = row.get("upload_chunk_size_bytes");
    let temp_key: String = row.get("temp_storage_key");
    ensure_upload_not_expired(row.get::<String, _>("expires_at").as_str())?;
    if status != "active" || next != chunk_index as i64 {
        return Err(ProjectError::Conflict(
            "The upload chunk is stale or out of order.".to_string(),
        ));
    }
    if bytes.is_empty()
        || bytes.len() as i64 > chunk_size
        || received + bytes.len() as i64 > expected
    {
        return Err(ProjectError::InvalidDemo(
            "Invalid upload chunk size.".to_string(),
        ));
    }

    let storage = &state.storage;
    let multipart_id = row
        .get::<Option<String>, _>("r2_upload_id")
        .ok_or_else(|| {
            ProjectError::InternalError("Upload session is missing its multipart id.".to_string())
        })?;
    let etag = storage
        .upload_part(
            &temp_key,
            &multipart_id,
            (chunk_index as i32) + 1,
            bytes.to_vec(),
        )
        .await
        .map_err(storage_error)?;

    let new_received = received + bytes.len() as i64;
    let new_next = next + 1;
    let new_etags = append_part_etag(
        row.get::<Option<String>, _>("r2_part_etags").as_deref(),
        &etag,
    );
    let update = format!(
        "UPDATE {table} SET received_size_bytes = ?, next_chunk_index = ?, r2_part_etags = ?, updated_at = CURRENT_TIMESTAMP WHERE id = ? AND status = 'active' AND next_chunk_index = ?"
    );
    let changed = sqlx::query(&update)
        .bind(new_received)
        .bind(new_next)
        .bind(&new_etags)
        .bind(upload_id)
        .bind(next)
        .execute(&state.project_service.pool)
        .await?;
    if changed.rows_affected() != 1 {
        let _ = storage.abort_multipart(&temp_key, &multipart_id).await;
        let failed = format!(
            "UPDATE {table} SET status = 'failed', error_message = ?, updated_at = CURRENT_TIMESTAMP WHERE id = ?"
        );
        sqlx::query(&failed)
            .bind("Chunk relay conflict")
            .bind(upload_id)
            .execute(&state.project_service.pool)
            .await
            .ok();
        return Err(ProjectError::Conflict(
            "The upload was changed concurrently.".to_string(),
        ));
    }
    Ok(ChunkUploadResponse {
        received_size_bytes: new_received as u64,
        next_chunk_index: new_next as u64,
    })
}

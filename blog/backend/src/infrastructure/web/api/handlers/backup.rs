use std::io::{Read, Seek, Write};
use std::path::{Path, PathBuf};
use std::sync::Arc;

use axum::{
    body::{Body, Bytes},
    extract::State,
    http::header,
    response::{IntoResponse, Response},
};
use chrono::Utc;
use futures::StreamExt;
use tokio_util::io::ReaderStream;
use uuid::Uuid;
use zip::ZipWriter;
use zip::write::FileOptions;

use crate::domain::errors::backup::BackupError;
use crate::infrastructure::web::server::AppState;
use crate::infrastructure::web::server::DatabaseSource;

fn add_dir_to_zip<W: Write + Seek>(
    zip: &mut ZipWriter<W>,
    dir: &Path,
    zip_prefix: &str,
) -> Result<(), BackupError> {
    if !dir.exists() {
        return Ok(());
    }
    add_dir_recursive(zip, dir, dir, zip_prefix)?;
    Ok(())
}

fn add_dir_recursive<W: Write + Seek>(
    zip: &mut ZipWriter<W>,
    base: &Path,
    current: &Path,
    zip_prefix: &str,
) -> Result<(), BackupError> {
    for entry in std::fs::read_dir(current)? {
        let entry = entry?;
        let path = entry.path();
        let relative = path
            .strip_prefix(base)
            .map_err(|e| BackupError::InternalError(e.to_string()))?;
        let zip_name = format!("{}/{}", zip_prefix, relative.display());

        if entry.file_type()?.is_dir() {
            zip.add_directory(zip_name, FileOptions::default())?;
            add_dir_recursive(zip, base, &path, zip_prefix)?;
        } else if entry.file_type()?.is_file() {
            let mut file = std::fs::File::open(&path)?;
            zip.start_file(zip_name, FileOptions::default())?;
            // Media files reach 100 MB; pipe them straight into the entry
            // instead of buffering each one in a Vec.
            std::io::copy(&mut file, &mut *zip)?;
        }
    }
    Ok(())
}

fn build_backup_zip(
    out_path: &PathBuf,
    db_path: &PathBuf,
    media_dir: &PathBuf,
    demos_dir: &PathBuf,
    prefix: &str,
) -> Result<(), BackupError> {
    let file = std::fs::File::create(out_path)?;
    let mut zip = ZipWriter::new(file);

    // Add database
    if db_path.exists() {
        let relative = db_path
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("blog.db");
        let mut file = std::fs::File::open(db_path)?;
        let mut content = Vec::new();
        file.read_to_end(&mut content)?;
        zip.start_file(
            format!("{}/database/{}", prefix, relative),
            FileOptions::default(),
        )?;
        zip.write_all(&content)?;
    }

    // Add media
    add_dir_to_zip(&mut zip, media_dir, &format!("{}/media", prefix))?;

    // Add project demos
    add_dir_to_zip(&mut zip, demos_dir, &format!("{}/project-demos", prefix))?;

    zip.finish()?;
    Ok(())
}

pub async fn download_backup(
    State(state): State<Arc<AppState>>,
) -> Result<impl IntoResponse, BackupError> {
    let timestamp = Utc::now().format("%Y%m%d_%H%M%S");
    let filename = format!("blog-backup-{}.zip", timestamp);
    let prefix = format!("blog-backup-{}", timestamp);

    let db_path = match &state.config.database_source {
        DatabaseSource::Sqlite { path } => path.clone(),
    };
    let media_dir = state.media_config.dir.clone();
    let demos_dir = state.project_demo_config.dir.clone();

    // Media + demos can add up to hundreds of MB, so the archive is written
    // to a temp file on the blocking pool and streamed back instead of being
    // assembled in RAM on an async worker.
    let temp_path = std::env::temp_dir().join(format!("blog-backup-{}-{}.zip", timestamp, Uuid::new_v4()));
    let build_path = temp_path.clone();
    tokio::task::spawn_blocking(move || {
        build_backup_zip(&build_path, &db_path, &media_dir, &demos_dir, &prefix)
    })
    .await
    .map_err(|e| BackupError::InternalError(e.to_string()))??;

    let file = tokio::fs::File::open(&temp_path).await?;

    // Chain a final step onto the body stream so the temp archive is removed
    // whether the download completes or the client disconnects mid-stream.
    let cleanup_path = temp_path.clone();
    let cleanup = futures::stream::once(async move {
        let _ = tokio::fs::remove_file(&cleanup_path).await;
        Ok::<Bytes, std::io::Error>(Bytes::new())
    });
    let stream = ReaderStream::with_capacity(file, 64 * 1024).chain(cleanup);

    Ok(Response::builder()
        .header(header::CONTENT_TYPE, "application/zip")
        .header(
            header::CONTENT_DISPOSITION,
            format!("attachment; filename=\"{}\"", filename),
        )
        .body(Body::from_stream(stream))
        .unwrap())
}

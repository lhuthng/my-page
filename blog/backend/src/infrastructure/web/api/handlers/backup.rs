use std::io::{Read, Seek, Write};
use std::path::Path;
use std::sync::Arc;

use axum::{
    body::Body,
    extract::State,
    http::header,
    response::{IntoResponse, Response},
};
use chrono::Utc;
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
            let mut content = Vec::new();
            file.read_to_end(&mut content)?;
            zip.start_file(zip_name, FileOptions::default())?;
            zip.write_all(&content)?;
        }
    }
    Ok(())
}

pub async fn download_backup(
    State(state): State<Arc<AppState>>,
) -> Result<impl IntoResponse, BackupError> {
    let timestamp = Utc::now().format("%Y%m%d_%H%M%S");
    let filename = format!("blog-backup-{}.zip", timestamp);
    let prefix = format!("blog-backup-{}", timestamp);

    let cursor = std::io::Cursor::new(Vec::new());
    let mut zip = ZipWriter::new(cursor);

    // Add database
    match &state.config.database_source {
        DatabaseSource::Sqlite { path } => {
            if path.exists() {
                let relative = path
                    .file_name()
                    .and_then(|n| n.to_str())
                    .unwrap_or("blog.db");
                let mut file = std::fs::File::open(path)?;
                let mut content = Vec::new();
                file.read_to_end(&mut content)?;
                zip.start_file(
                    format!("{}/database/{}", prefix, relative),
                    FileOptions::default(),
                )?;
                zip.write_all(&content)?;
            }
        }
    }

    // Add media
    add_dir_to_zip(
        &mut zip,
        &state.media_config.dir,
        &format!("{}/media", prefix),
    )?;

    // Add project demos
    add_dir_to_zip(
        &mut zip,
        &state.project_demo_config.dir,
        &format!("{}/project-demos", prefix),
    )?;

    let cursor = zip.finish()?;
    let bytes = cursor.into_inner();

    Ok(Response::builder()
        .header(header::CONTENT_TYPE, "application/zip")
        .header(
            header::CONTENT_DISPOSITION,
            format!("attachment; filename=\"{}\"", filename),
        )
        .body(Body::from(bytes))
        .unwrap())
}

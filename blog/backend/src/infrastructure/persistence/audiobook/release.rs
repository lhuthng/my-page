use sqlx::{Sqlite, SqlitePool, Transaction};
use tokio::fs;

use crate::infrastructure::persistence::media::MEDIA_REFERENCES;
use crate::infrastructure::storage::{audio_object_key, chunked, media_key};
use crate::infrastructure::web::server::MediaConfig;

pub(super) struct ReleasedMedium {
    hash: String,
    file_type: String,
    uploader_id: i64,
}

impl ReleasedMedium {
    // Recheck ownership after the caller commits and exclude writers until every unlink finishes.
    pub(super) async fn discard(self, pool: &SqlitePool, config: &MediaConfig) {
        let Some(key) = media_key(&self.hash, &self.file_type, self.uploader_id) else {
            tracing::warn!(
                hash = %self.hash,
                file_type = %self.file_type,
                "released medium: no storage key derivable, files left in place"
            );
            return;
        };
        let mut guard = match pool.begin_with("BEGIN IMMEDIATE").await {
            Ok(guard) => guard,
            Err(error) => {
                tracing::warn!(hash = %self.hash, %error, "released medium: lock failed, files left in place");
                return;
            }
        };
        match sqlx::query_scalar::<_, bool>("SELECT EXISTS(SELECT 1 FROM media WHERE hash = ?)")
            .bind(&self.hash)
            .fetch_one(&mut *guard)
            .await
        {
            Ok(false) => {}
            Ok(true) => return,
            Err(error) => {
                tracing::warn!(hash = %self.hash, %error, "released medium: ownership check failed, files left in place");
                return;
            }
        }

        match fs::remove_file(config.dir.join(&key.key)).await {
            Ok(()) => tracing::info!(key = %key.key, "released medium: disk copy deleted"),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
            Err(error) => tracing::error!(
                key = %key.key,
                %error,
                "released medium: disk copy could not be deleted"
            ),
        }

        if let Some(bucket) = &config.audio_bucket
            && let Some(object) = audio_object_key(&key.key, &self.file_type)
            && let Err(error) = bucket.client.delete_object(&object).await
        {
            tracing::error!(
                key = %object,
                %error,
                "released medium: bucket object could not be deleted"
            );
        }

        // Only the manifest's exact chunk keys are deleted, never a bare bucket prefix.
        if let Some(manifest) = chunked::read_manifest(config, &key.key, &self.file_type).await {
            chunked::discard(config, &key.key, &self.file_type, &manifest).await;
        }
        if let Err(error) = guard.rollback().await {
            tracing::warn!(hash = %self.hash, %error, "released medium: cleanup lock could not be released");
        }
    }
}

// Release ownership in the caller's transaction; physical deletion must wait for its commit.
pub(super) async fn release_medium(
    tx: &mut Transaction<'_, Sqlite>,
    media_id: i64,
) -> Result<Option<ReleasedMedium>, sqlx::Error> {
    let row: Option<(String, String, i64)> =
        sqlx::query_as("SELECT hash, file_type, COALESCE(uploader_id, 0) FROM media WHERE id = ?")
            .bind(media_id)
            .fetch_optional(&mut **tx)
            .await?;
    let Some((hash, file_type, uploader_id)) = row else {
        return Ok(None);
    };

    let referenced: bool = sqlx::query_scalar(&format!(
        "SELECT EXISTS(SELECT 1 FROM ({MEDIA_REFERENCES}) WHERE media_id = ?)"
    ))
    .bind(media_id)
    .fetch_one(&mut **tx)
    .await?;
    if referenced {
        return Ok(None);
    }
    if media_key(&hash, &file_type, uploader_id).is_none() {
        tracing::warn!(%hash, %file_type, "released medium: no storage key derivable, ownership left in place");
        return Ok(None);
    }

    let same_bytes_elsewhere: bool =
        sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM media WHERE hash = ? AND id != ?)")
            .bind(&hash)
            .bind(media_id)
            .fetch_one(&mut **tx)
            .await?;

    sqlx::query("DELETE FROM media WHERE id = ?")
        .bind(media_id)
        .execute(&mut **tx)
        .await?;

    Ok((!same_bytes_elsewhere).then_some(ReleasedMedium {
        hash,
        file_type,
        uploader_id,
    }))
}

#[cfg(test)]
mod tests {
    use std::{path::PathBuf, sync::Arc, time::Duration};

    use axum::body::Bytes;
    use sqlx::sqlite::{SqliteConnectOptions, SqliteJournalMode, SqlitePoolOptions};

    use crate::application::{
        commands::media::UploadMediaWithoutDescriptionCommand, services::media::MediaService,
    };
    use crate::domain::entities::media::MediaType;
    use crate::infrastructure::persistence::media::{MediaServiceImpl, hash_bytes};
    use crate::infrastructure::storage::{chunk_key, manifest_key};

    use super::*;

    const AUDIO: &[u8] = b"audio reused by a concurrent upload";

    struct Fixture {
        pool: SqlitePool,
        config: Arc<MediaConfig>,
        media_id: i64,
        files: Vec<(PathBuf, Vec<u8>)>,
    }

    impl Fixture {
        async fn new() -> Self {
            let dir = std::env::temp_dir().join(format!("audio-release-{}", uuid::Uuid::new_v4()));
            fs::create_dir_all(&dir).await.unwrap();
            let pool = SqlitePoolOptions::new()
                .max_connections(2)
                .connect_with(
                    SqliteConnectOptions::new()
                        .filename(dir.join("test.db"))
                        .create_if_missing(true)
                        .journal_mode(SqliteJournalMode::Wal)
                        .busy_timeout(Duration::from_millis(100)),
                )
                .await
                .unwrap();
            sqlx::migrate::Migrator::new(
                PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("migrations"),
            )
            .await
            .unwrap()
            .run(&pool)
            .await
            .unwrap();
            sqlx::query("INSERT INTO users (id, username, email, password_hash) VALUES (1, 'alice', 'a@example.com', 'h')")
                .execute(&pool).await.unwrap();
            let config = Arc::new(MediaConfig {
                dir,
                allowed_file_types: vec![MediaType::AudioMp3],
                allowed_avatar_types: Vec::new(),
                allowed_cover_types: Vec::new(),
                allowed_audio_types: vec![MediaType::AudioMp3],
                audio_bucket: None,
            });
            let hashed = hash_bytes(
                &Bytes::from_static(AUDIO),
                &config.dir,
                ".mp3".to_string(),
                true,
            )
            .await
            .unwrap();
            fs::create_dir_all(&hashed.dir_path).await.unwrap();
            fs::write(&hashed.file_path, AUDIO).await.unwrap();
            let key = media_key(&hashed.hash, "audio/mpeg", 1).unwrap().key;
            chunked::write(&config, &key, "audio/mpeg", AUDIO)
                .await
                .unwrap();
            let media_id = sqlx::query_scalar(
                "INSERT INTO media (hash, short_name, file_name, file_type, url, size, uploader_id)
                 VALUES (?, 'original', 'audio.mp3', 'audio/mpeg', ?, ?, 1) RETURNING id",
            )
            .bind(&hashed.hash)
            .bind(hashed.file_path.to_str().unwrap())
            .bind(AUDIO.len() as i64)
            .fetch_one(&pool)
            .await
            .unwrap();
            let mut files = Vec::new();
            for key in [key.clone(), manifest_key(&key), chunk_key(&key, 0)] {
                let path = config.dir.join(key);
                let bytes = fs::read(&path).await.unwrap();
                files.push((path, bytes));
            }
            Self {
                pool,
                config,
                media_id,
                files,
            }
        }

        async fn release(&self) -> ReleasedMedium {
            let mut tx = self.pool.begin_with("BEGIN IMMEDIATE").await.unwrap();
            let released = release_medium(&mut tx, self.media_id)
                .await
                .unwrap()
                .unwrap();
            self.assert_files().await;
            tx.commit().await.unwrap();
            released
        }

        async fn assert_files(&self) {
            for (path, bytes) in &self.files {
                assert_eq!(&fs::read(path).await.unwrap(), bytes, "{}", path.display());
            }
        }

        async fn cleanup(self) {
            self.pool.close().await;
            fs::remove_dir_all(&self.config.dir).await.unwrap();
        }
    }

    #[tokio::test]
    async fn same_hash_upload_between_commit_and_discard_keeps_all_files() {
        let fx = Fixture::new().await;
        let released = fx.release().await;
        let pool = fx.pool.clone();
        let config = fx.config.clone();
        tokio::spawn(async move {
            MediaServiceImpl::new(pool)
                .bulk_upload(
                    UploadMediaWithoutDescriptionCommand {
                        uploader_id: 1,
                        number_of_files: 1,
                        short_names: vec!["reused".to_string()],
                        file_names: vec!["reused.mp3".to_string()],
                        content_types: vec!["audio/mpeg".to_string()],
                        bytes_list: vec![Bytes::from_static(AUDIO)],
                    },
                    &config,
                )
                .await
                .unwrap();
        })
        .await
        .unwrap();
        let references: i64 = sqlx::query_scalar(&format!(
            "SELECT COUNT(*) FROM ({MEDIA_REFERENCES}) WHERE media_id IS NOT NULL"
        ))
        .fetch_one(&fx.pool)
        .await
        .unwrap();
        assert_eq!(
            references, 0,
            "even an unreferenced media row owns its bytes"
        );

        released.discard(&fx.pool, &fx.config).await;

        fx.assert_files().await;
        fx.cleanup().await;
    }

    #[tokio::test]
    async fn discard_guard_failures_leave_all_files_untouched() {
        for failure in ["query", "lock", "closed-pool"] {
            let fx = Fixture::new().await;
            let released = fx.release().await;
            let mut guard = None;
            match failure {
                "query" => {
                    sqlx::query("DROP TABLE media")
                        .execute(&fx.pool)
                        .await
                        .unwrap();
                }
                "lock" => {
                    guard = Some(fx.pool.begin_with("BEGIN IMMEDIATE").await.unwrap());
                }
                "closed-pool" => fx.pool.close().await,
                _ => unreachable!(),
            }

            released.discard(&fx.pool, &fx.config).await;

            fx.assert_files().await;
            if let Some(guard) = guard {
                guard.rollback().await.unwrap();
            }
            fx.cleanup().await;
        }
    }

    #[tokio::test]
    async fn unmappable_media_keeps_ownership_and_files() {
        for (hash, file_type) in [
            ("unknown-layout", "audio/mpeg"),
            ("aabbccdd", "unknown/type"),
        ] {
            let fx = Fixture::new().await;
            sqlx::query("UPDATE media SET hash = ?, file_type = ? WHERE id = ?")
                .bind(hash)
                .bind(file_type)
                .bind(fx.media_id)
                .execute(&fx.pool)
                .await
                .unwrap();
            let mut tx = fx.pool.begin_with("BEGIN IMMEDIATE").await.unwrap();
            assert!(
                release_medium(&mut tx, fx.media_id)
                    .await
                    .unwrap()
                    .is_none()
            );
            tx.commit().await.unwrap();
            let exists: bool =
                sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM media WHERE id = ?)")
                    .bind(fx.media_id)
                    .fetch_one(&fx.pool)
                    .await
                    .unwrap();
            assert!(exists);
            fx.assert_files().await;
            fx.cleanup().await;
        }
    }

    #[tokio::test]
    async fn a_rolled_back_release_preserves_files() {
        let fx = Fixture::new().await;
        let mut tx = fx.pool.begin_with("BEGIN IMMEDIATE").await.unwrap();
        let released = release_medium(&mut tx, fx.media_id).await.unwrap().unwrap();
        fx.assert_files().await;
        tx.rollback().await.unwrap();

        released.discard(&fx.pool, &fx.config).await;

        fx.assert_files().await;
        fx.cleanup().await;
    }
}

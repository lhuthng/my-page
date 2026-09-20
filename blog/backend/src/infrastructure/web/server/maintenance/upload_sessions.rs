// Orphaned upload-session cleanup, run at boot and periodically.
use std::path::Path;

use crate::infrastructure::storage::ObjectStore;

async fn cleanup_orphaned_uploads(
    pool: &sqlx::SqlitePool,
    storage: &ObjectStore,
    demos_dir: &std::path::Path,
) -> Result<(), Box<dyn std::error::Error>> {
    async fn clean_game_session(
        pool: &sqlx::SqlitePool,
        storage: &ObjectStore,
        demos_dir: &std::path::Path,
        session_id: &str,
    ) {
        use sqlx::Row;
        let row = sqlx::query(
            "SELECT staged_disk_storage_key, disk_reuse FROM project_v86_upload_sessions WHERE id = ?",
        )
        .bind(session_id)
        .fetch_optional(pool)
        .await
        .ok()
        .flatten();
        if let Some(row) = row {
            // Game sessions upload straight to content-addressed keys; only the
            // objects this session uploaded (never shared/reused ones) are
            // removed, matching the abort path.
            let disk_reuse: i64 = row.get("disk_reuse");
            if disk_reuse == 0 {
                if let Some(key) = row.get::<Option<String>, _>("staged_disk_storage_key") {
                    let _ = storage.delete_prefix(&key).await;
                }
            }
            let uploaded_isos: Vec<String> = sqlx::query_scalar(
                "SELECT iso_storage_key FROM project_v86_staged_variants WHERE upload_id = ? AND reuse = 0",
            )
            .bind(session_id)
            .fetch_all(pool)
            .await
            .unwrap_or_default();
            for key in uploaded_isos {
                let _ = storage.delete_object(&format!("{key}/full.iso")).await;
            }
            let _ = tokio::fs::remove_dir_all(demos_dir.join("v86/tmp/build").join(session_id))
                .await;
        }
    }

    async fn clean_system_session(
        pool: &sqlx::SqlitePool,
        storage: &ObjectStore,
        session_id: &str,
    ) {
        use sqlx::Row;
        let row = sqlx::query(
            "SELECT staged_storage_key, reuse FROM v86_system_upload_sessions WHERE id = ?",
        )
        .bind(session_id)
        .fetch_optional(pool)
        .await
        .ok()
        .flatten();
        if let Some(row) = row {
            let reuse: i64 = row.get("reuse");
            if reuse == 0 {
                if let Some(key) = row.get::<Option<String>, _>("staged_storage_key") {
                    // Only delete if no other active session still owns this key.
                    let other_owners: i64 = sqlx::query_scalar(
                        "SELECT COUNT(*) FROM v86_system_upload_sessions
                         WHERE id != ? AND staged_storage_key = ?
                         AND status IN ('active', 'building')",
                    )
                    .bind(session_id)
                    .bind(&key)
                    .fetch_one(pool)
                    .await
                    .unwrap_or(0);
                    if other_owners == 0 {
                        let _ = storage.delete_prefix(&key).await;
                    }
                }
            }
        }
    }

    let rows: Vec<(String, i64, i64)> = sqlx::query_as(
        r#"SELECT s.id, s.system_id, s.expected_current_version
           FROM v86_system_upload_sessions s
           WHERE s.system_id IS NOT NULL
             AND (s.status = 'building'
                  OR (s.status = 'active' AND s.expires_at < datetime('now')))"#,
    )
    .fetch_all(pool)
    .await?;

    for (session_id, system_id, expected_version) in &rows {
        // Drop any version row a crashed build may have reserved before it finished.
        let orphan_version_id: Option<i64> = sqlx::query_scalar(
            "SELECT id FROM v86_system_versions WHERE system_id = ? AND version_number = ?",
        )
        .bind(system_id)
        .bind(expected_version + 1)
        .fetch_optional(pool)
        .await?;
        if let Some(version_id) = orphan_version_id {
            // Grab the content-addressed storage key before dropping the row.
            let orphan_key: Option<String> = sqlx::query_scalar(
                "SELECT storage_key FROM v86_system_versions WHERE id = ?",
            )
            .bind(version_id)
            .fetch_optional(pool)
            .await?;
            sqlx::query("DELETE FROM v86_system_versions WHERE id = ?")
                .bind(version_id)
                .execute(pool)
                .await?;
            // Only free the objects if no other version (e.g. a shared/deduped
            // image) still references the same content-addressed key.
            if let Some(key) = orphan_key {
                let remaining: i64 = sqlx::query_scalar(
                    "SELECT COUNT(*) FROM v86_system_versions WHERE storage_key = ?",
                )
                .bind(&key)
                .fetch_one(pool)
                .await?;
                if remaining == 0 {
                    let _ = storage.delete_prefix(&key).await;
                }
            }
        }

        if *expected_version == 0 {
            sqlx::query("DELETE FROM v86_systems WHERE id = ?")
                .bind(system_id)
                .execute(pool)
                .await?;
        } else {
            sqlx::query(
                "UPDATE v86_systems SET current_version = ?, updated_at = CURRENT_TIMESTAMP WHERE id = ? AND current_version = ?",
            )
            .bind(expected_version)
            .bind(system_id)
            .bind(expected_version + 1)
            .execute(pool)
            .await?;
        }
        sqlx::query(
            "UPDATE v86_system_upload_sessions SET status = 'expired', updated_at = CURRENT_TIMESTAMP WHERE id = ?",
        )
        .bind(session_id)
        .execute(pool)
        .await?;
        clean_system_session(pool, storage, session_id).await;
    }

    let stale_games: Vec<String> = sqlx::query_scalar(
        r#"SELECT id FROM project_v86_upload_sessions
           WHERE status = 'building'
              OR (status = 'active' AND expires_at < datetime('now'))"#,
    )
    .fetch_all(pool)
    .await?;
    for session_id in &stale_games {
        sqlx::query(
            "UPDATE project_v86_upload_sessions SET status = 'expired', updated_at = CURRENT_TIMESTAMP WHERE id = ?",
        )
        .bind(session_id)
        .execute(pool)
        .await?;
        clean_game_session(pool, storage, demos_dir, session_id).await;
    }

    // Finished sessions (consumed/aborted/ready/expired) are never removed
    // otherwise; once they age out of the TTL they are safe to drop.
    let terminal_games: Vec<String> = sqlx::query_scalar(
        r#"SELECT id FROM project_v86_upload_sessions
           WHERE status != 'active' AND status != 'building'
             AND expires_at < datetime('now')"#,
    )
    .fetch_all(pool)
    .await?;
    for session_id in &terminal_games {
        sqlx::query("DELETE FROM project_v86_upload_sessions WHERE id = ?")
            .bind(session_id)
            .execute(pool)
            .await?;
    }

    let stale_systems = rows.len();
    if stale_systems + stale_games.len() > 0 {
        println!(
            "Cleaned up {} orphaned upload session(s)",
            stale_systems + stale_games.len()
        );
    }

    Ok(())
}


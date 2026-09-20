// Game artifact GC: removes orphaned v86/js-dos objects for a game.
use crate::infrastructure::storage::ObjectStore;

pub(super) async fn cleanup_game_artifacts(pool: &sqlx::SqlitePool, storage: &ObjectStore, game_id: i64) {
    let rows: Vec<(String,)> = sqlx::query_as(
        r#"SELECT zip_storage_key FROM game_v86_games WHERE game_id = ?
           UNION ALL SELECT iso_storage_key FROM game_v86_games WHERE game_id = ?
           UNION ALL SELECT COALESCE(disk_storage_key, '') FROM game_v86_games WHERE game_id = ?
           UNION ALL SELECT iso_storage_key FROM game_v86_variants WHERE game_id = ?"#,
    )
    .bind(game_id)
    .bind(game_id)
    .bind(game_id)
    .bind(game_id)
    .fetch_all(pool)
    .await
    .unwrap_or_default();
    let keys: Vec<String> = rows
        .into_iter()
        .map(|(key,)| key)
        .filter(|key| !key.is_empty())
        .collect();

    for key in keys {
        // A prefix shared with another game (same disk or ISO digest) must stay.
        let still_used: i64 = sqlx::query_scalar(
            r#"SELECT (SELECT COUNT(*) FROM game_v86_games WHERE (disk_storage_key = ?1 OR iso_storage_key = ?1) AND game_id != ?2)
                    + (SELECT COUNT(*) FROM game_v86_variants WHERE iso_storage_key = ?1 AND game_id != ?2)"#,
        )
        .bind(&key)
        .bind(game_id)
        .fetch_one(pool)
        .await
        .unwrap_or(1);
        if still_used == 0 {
            let _ = storage.delete_prefix(&key).await;
        }
    }

    let snapshot_keys: Vec<(String,)> = sqlx::query_as(
        "SELECT storage_key FROM game_v86_snapshots WHERE game_id = ?",
    )
    .bind(game_id)
    .fetch_all(pool)
    .await
    .unwrap_or_default();
    for (key,) in snapshot_keys {
        let still_used: i64 = sqlx::query_scalar(
            "SELECT COUNT(*) FROM game_v86_snapshots WHERE storage_key = ? AND game_id != ?",
        )
        .bind(&key)
        .bind(game_id)
        .fetch_one(pool)
        .await
        .unwrap_or(1);
        if still_used == 0 {
            let _ = storage.delete_object(&key).await;
        }
    }

    // A js-dos bundle belongs to exactly one game (game_id is its primary key).
    let jsdos_key: Option<String> = sqlx::query_scalar(
        "SELECT storage_key FROM game_jsdos_bundles WHERE game_id = ?",
    )
    .bind(game_id)
    .fetch_optional(pool)
    .await
    .unwrap_or(None);
    if let Some(key) = jsdos_key {
        let _ = storage.delete_object(&key).await;
    }
}


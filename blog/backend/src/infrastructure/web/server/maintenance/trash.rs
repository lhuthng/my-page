// Expired trash purge: hard-deletes posts past their purge date.
use std::path::Path;

use crate::infrastructure::storage::ObjectStore;

use super::game_artifacts::cleanup_game_artifacts;

async fn purge_expired_trash(
    pool: &sqlx::SqlitePool,
    storage: &ObjectStore,
    demos_dir: &std::path::Path,
) -> Result<(), Box<dyn std::error::Error>> {
    let ids: Vec<i64> = sqlx::query_scalar(
        "SELECT id FROM posts WHERE deleted_at IS NOT NULL AND scheduled_purge_at <= datetime('now')",
    )
    .fetch_all(pool)
    .await?;
    if ids.is_empty() {
        return Ok(());
    }
    for post_id in &ids {
        // best-effort demo dir cleanup for projects/games before cascade
        if let Some((kind, pid)) = sqlx::query_as::<_, (String, i64)>(
            "SELECT 'project', projects.id FROM projects WHERE post_id = ? UNION ALL SELECT 'game', games.id FROM games WHERE post_id = ?",
        )
        .bind(post_id)
        .bind(post_id)
        .fetch_optional(pool)
        .await?
        {
            let dir = if kind == "project" {
                demos_dir.join(pid.to_string())
            } else {
                demos_dir.join(format!("game-{}", pid))
            };
            let _ = tokio::fs::remove_dir_all(&dir).await;
            if kind == "game" {
                cleanup_game_artifacts(pool, storage, pid).await;
            }
        }
        sqlx::query("DELETE FROM posts WHERE id = ?")
            .bind(post_id)
            .execute(pool)
            .await?;
    }
    println!("Purged {} trashed post(s)", ids.len());
    Ok(())
}

/// Best-effort removal of a game's object-store artifacts (js-dos bundle,
/// v86 disk/ISO prefixes, snapshots). Runs before the FK cascade deletes the
/// rows, so keys still referenced by other games are detected and kept —
/// v86 keys are content-addressed and can be shared.

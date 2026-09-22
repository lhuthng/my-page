// Artifact key validation and existence checks for the transfer flow.

pub fn is_valid_artifact_key_shape(key: &str) -> bool {
    !key.is_empty()
        && !key.starts_with('/')
        && !key.contains('\\')
        && key
            .split('/')
            .all(|segment| !segment.is_empty() && segment != "." && segment != "..")
}

/// Validates that a requested artifact key belongs to one of the storage keys
/// recorded in the database (the key itself or a child object of it), so the
/// `/sync/artifact` endpoint can never serve arbitrary paths.
pub async fn artifact_key_exists(pool: &sqlx::SqlitePool, key: &str) -> Result<bool, String> {
    if !is_valid_artifact_key_shape(key) {
        return Ok(false);
    }
    let hits: (i64,) = sqlx::query_as(
        r#"SELECT
             (SELECT EXISTS(SELECT 1 FROM v86_system_versions
                WHERE ?1 = storage_key OR ?1 LIKE storage_key || '/%'))
           + (SELECT EXISTS(SELECT 1 FROM game_v86_games
                WHERE ?1 = zip_storage_key OR ?1 = iso_storage_key
                   OR ?1 LIKE iso_storage_key || '/%'
                   OR (disk_storage_key IS NOT NULL AND ?1 LIKE disk_storage_key || '/%')))
           + (SELECT EXISTS(SELECT 1 FROM game_v86_variants
                WHERE ?1 = iso_storage_key OR ?1 LIKE iso_storage_key || '/%'))
           + (SELECT EXISTS(SELECT 1 FROM game_v86_snapshots WHERE ?1 = storage_key))
           + (SELECT EXISTS(SELECT 1 FROM game_v86_saves WHERE ?1 = storage_key))
           + (SELECT EXISTS(SELECT 1 FROM game_jsdos_bundles WHERE ?1 = storage_key))"#,
    )
    .bind(key)
    .fetch_one(pool)
    .await
    .map_err(|e| e.to_string())?;
    Ok(hits.0 > 0)
}

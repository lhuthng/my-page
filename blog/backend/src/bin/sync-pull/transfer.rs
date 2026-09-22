// Manifest-driven section transfers: media, demo trees, game artifacts, and
// the database snapshot swap. Each takes a `Session` plus its slice of the
// manifest and updates a shared `Counters`.
use std::collections::HashSet;
use std::path::{Path, PathBuf};

use backend::infrastructure::sync::{ArtifactEntry, DemoDir, MediaEntry, fix_imported_database};
use sqlx::SqlitePool;
use sqlx::sqlite::{SqliteConnectOptions, SqliteJournalMode, SqliteSynchronous};

use super::download::{Fetched, collect_local_files, download_to_file, prune_extras};
use super::rewrite::format_bytes;

type Api<'a> = dyn Fn(&str) -> String + 'a;

/// Everything a section transfer needs to reach the source: the HTTP client,
/// the resolved per-section URL builder, and the sync key.
pub struct Session<'a> {
    pub client: &'a reqwest::Client,
    pub api: &'a Api<'a>,
    pub key: &'a str,
}

/// Transfer counters shared by the three section transfers.
#[derive(Default)]
pub struct Counters {
    pub downloaded: usize,
    pub skipped: usize,
    pub missing: usize,
    pub transferred: u64,
}

/// Downloads every media file listed in the manifest, then prunes when asked.
pub async fn sync_media(
    session: &Session<'_>,
    media_dir: &Path,
    entries: &[MediaEntry],
    prune: bool,
    c: &mut Counters,
) -> Result<(), String> {
    let mut expected = HashSet::new();
    let count = entries.len();
    for entry in entries {
        let target = media_dir.join(&entry.path);
        expected.insert(entry.path.clone());
        match download_to_file(
            session.client,
            (session.api)(&format!("media/{}", entry.hash)).as_str(),
            session.key,
            &target,
            Some(entry.size.max(0) as u64),
            true,
            false,
        )
        .await
        {
            Ok(Fetched::Downloaded) => {
                c.downloaded += 1;
                c.transferred += entry.size.max(0) as u64;
            }
            Ok(Fetched::Current) => c.skipped += 1,
            Ok(Fetched::Missing) => {
                c.missing += 1;
                println!("  missing on source: {}", entry.path);
            }
            Err(e) => return Err(e),
        }
        let done = c.downloaded + c.skipped;
        if done.is_multiple_of(200) && done < count {
            println!("  {done}/{count} …");
        }
    }
    println!(
        "  {} downloaded, {} already up to date",
        c.downloaded, c.skipped
    );
    if prune {
        let (removed, freed) = prune_extras(media_dir, &expected, false).await;
        println!(
            "  pruned {removed} extra file(s), freed {}",
            format_bytes(freed)
        );
    }
    Ok(())
}

/// Downloads every project/game demo file tree, then prunes when asked.
pub async fn sync_demo_dirs(
    session: &Session<'_>,
    demos_dir: &Path,
    project_demos: &[DemoDir],
    game_demos: &[DemoDir],
    prune: bool,
    c: &mut Counters,
) -> Result<(), String> {
    let mut expected_per_dir: Vec<(PathBuf, HashSet<String>)> = Vec::new();
    for (kind, dirs) in [("project", project_demos), ("game", game_demos)] {
        for dir in dirs {
            let base = match kind {
                "project" => demos_dir.join(dir.id.to_string()),
                _ => demos_dir.join(format!("game-{}", dir.id)),
            };
            let mut local = HashSet::new();
            for file in &dir.files {
                local.insert(file.path.clone());
                let target = base.join(&file.path);
                let section = format!("demo/{kind}/{}/{}", dir.id, file.path);
                match download_to_file(
                    session.client,
                    (session.api)(&section).as_str(),
                    session.key,
                    &target,
                    Some(file.size),
                    true,
                    false,
                )
                .await
                {
                    Ok(Fetched::Downloaded) => {
                        c.downloaded += 1;
                        c.transferred += file.size;
                    }
                    Ok(Fetched::Current) => c.skipped += 1,
                    Ok(Fetched::Missing) => {
                        c.missing += 1;
                        println!("  missing on source: {section}");
                    }
                    Err(e) => return Err(e),
                }
            }
            expected_per_dir.push((base, local));
        }
    }
    println!(
        "  {} downloaded (cumulative), {} already up to date",
        c.downloaded, c.skipped
    );
    if prune {
        let mut removed = 0usize;
        let mut freed = 0u64;
        for (base, local) in &expected_per_dir {
            let (r, f) = prune_extras(base, local, false).await;
            removed += r;
            freed += f;
        }
        println!(
            "  pruned {removed} extra file(s), freed {}",
            format_bytes(freed)
        );
    }
    Ok(())
}

pub async fn sync_artifacts(
    session: &Session<'_>,
    demos_dir: &Path,
    entries: &[ArtifactEntry],
    prune: bool,
    c: &mut Counters,
) -> Result<(), String> {
    let mut expected = HashSet::new();
    for entry in entries {
        expected.insert(entry.key.clone());
        let target = demos_dir.join(&entry.key);
        match download_to_file(
            session.client,
            (session.api)(&format!("artifact/{}", entry.key)).as_str(),
            session.key,
            &target,
            Some(entry.size),
            true,
            false,
        )
        .await
        {
            Ok(Fetched::Downloaded) => {
                c.downloaded += 1;
                c.transferred += entry.size;
            }
            Ok(Fetched::Current) => c.skipped += 1,
            Ok(Fetched::Missing) => {
                c.missing += 1;
                println!("  missing on source: {}", entry.key);
            }
            Err(e) => return Err(e),
        }
    }
    println!(
        "  {} downloaded (cumulative), {} already up to date",
        c.downloaded, c.skipped
    );
    if prune {
        // Anything under v86/ or jsdos/ that the manifest does not list
        // is a leftover (including transient v86/tmp files).
        let mut local = HashSet::new();
        for prefix in ["v86", "jsdos"] {
            let dir = demos_dir.join(prefix);
            if !dir.is_dir() {
                continue;
            }
            for (path, _) in collect_local_files(&dir) {
                if let Ok(relative) = path.strip_prefix(demos_dir) {
                    local.insert(relative.to_string_lossy().to_string());
                }
            }
        }
        let mut removed = 0usize;
        let mut freed = 0u64;
        for key in local {
            if expected.contains(&key) {
                continue;
            }
            let size = tokio::fs::metadata(demos_dir.join(&key))
                .await
                .map(|m| m.len())
                .unwrap_or(0);
            if tokio::fs::remove_file(demos_dir.join(&key)).await.is_ok() {
                removed += 1;
                freed += size;
            }
        }
        println!(
            "  pruned {removed} extra file(s), freed {}",
            format_bytes(freed)
        );
    }
    Ok(())
}

/// Downloads the source database snapshot, rewrites prod paths for the local
/// roots, and swaps it in (previous database kept aside as `*.pre-sync-*`).
pub async fn sync_database(
    session: &Session<'_>,
    db_path: &Path,
    media_dir: &Path,
    demos_dir: &Path,
    database_size_bytes: u64,
) -> Result<(), String> {
    println!("\n── Database ──");
    let snapshot = db_path.with_extension(format!(
        "{}.sync-tmp",
        db_path
            .extension()
            .and_then(|e| e.to_str())
            .unwrap_or_default()
    ));
    download_to_file(
        session.client,
        (session.api)("database").as_str(),
        session.key,
        &snapshot,
        None,
        false,
        false,
    )
    .await
    .map_err(|e| format!("database download: {e}"))?;

    // Rewrite prod paths for this machine's roots before the swap.
    let options = SqliteConnectOptions::new()
        .filename(&snapshot)
        .journal_mode(SqliteJournalMode::Wal)
        .synchronous(SqliteSynchronous::Normal);
    let pool = SqlitePool::connect_with(options)
        .await
        .map_err(|e| format!("open downloaded database: {e}"))?;
    let summary = fix_imported_database(&pool, media_dir, demos_dir)
        .await
        .map_err(|e| format!("fix imported database: {e}"))?;
    // Fold the WAL back into the main file so the rename below yields a
    // self-contained database, then drop any sidecar files SQLite left.
    sqlx::query("PRAGMA wal_checkpoint(TRUNCATE)")
        .execute(&pool)
        .await
        .map_err(|e| format!("checkpoint snapshot: {e}"))?;
    pool.close().await;

    // Save the current database aside, then swap the synced one in.
    if db_path.is_file() {
        let save_aside = db_path.with_extension(format!(
            "{}.pre-sync-{}",
            db_path
                .extension()
                .and_then(|e| e.to_str())
                .unwrap_or_default(),
            chrono::Utc::now().format("%Y%m%d_%H%M%S")
        ));
        tokio::fs::rename(&db_path, &save_aside)
            .await
            .map_err(|e| format!("save aside {}: {e}", db_path.display()))?;
        println!("  previous database kept as {}", save_aside.display());
    }
    for stale in [
        db_path.with_extension("db-wal"),
        db_path.with_extension("db-shm"),
        snapshot.with_extension("sync-tmp-wal"),
        snapshot.with_extension("sync-tmp-shm"),
    ] {
        let _ = tokio::fs::remove_file(&stale).await;
    }
    tokio::fs::rename(&snapshot, &db_path)
        .await
        .map_err(|e| format!("activate {}: {e}", db_path.display()))?;
    println!(
        "  imported {} (media urls fixed: {}, project demo urls: {}, game demo urls: {})",
        format_bytes(database_size_bytes),
        summary.media_urls_fixed,
        summary.project_demo_urls_fixed,
        summary.game_demo_urls_fixed
    );
    Ok(())
}

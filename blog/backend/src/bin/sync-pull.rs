//! Pulls a production (or any source) blog environment into this machine
//! using a sync key issued from the admin dashboard:
//!
//!   cargo run --bin sync-pull -- --url https://huuthangle.site --key @sync.key
//!
//! Steps: fetch manifest -> replace the local database (with a save-aside of
//! the current one) -> rewrite prod paths for the local MEDIA_PATH /
//! PROJECT_DEMOS_PATH -> download media, demo files, and game artifacts
//! (skipping files that already exist with the right size). Idempotent: safe
//! to re-run after an interrupted sync.
//!
//! Direction policy: this tool only ever writes to the machine it runs on.
//! Pushing to a production environment is deliberately not implemented.

use std::collections::HashSet;
use std::path::{Path, PathBuf};
use std::time::Instant;

use backend::infrastructure::sync::{fix_imported_database, SyncManifest};
use futures::StreamExt;
use sqlx::sqlite::{SqliteConnectOptions, SqliteJournalMode, SqliteSynchronous};
use sqlx::SqlitePool;
use tokio::io::AsyncWriteExt;

#[derive(Default)]
struct Args {
    url: Option<String>,
    key: Option<String>,
    env_file: Option<PathBuf>,
    db_path: Option<PathBuf>,
    media_dir: Option<PathBuf>,
    demos_dir: Option<PathBuf>,
    skip: HashSet<String>,
    prune: bool,
    dry_run: bool,
    yes: bool,
}

fn print_usage() {
    println!(
        "Usage: cargo run --bin sync-pull -- --url <site-url> --key <bsk_…|@file> [options]

Required:
  --url <site-url>        Public URL of the source site (requests go to {{url}}/api/sync/…).
  --key <key|@file>       Sync key from the admin dashboard, or @path to a file holding it.

Options:
  --env <path>            Backend .env to read DATABASE_URL / MEDIA_PATH / PROJECT_DEMOS_PATH
                          from (default: ./backend/.env then ./.env).
  --db <path>             Override the local SQLite database path.
  --media-dir <path>      Override the local media directory.
  --demos-dir <path>      Override the local project-demos directory.
  --skip <what>           Skip a section: media, demos, artifacts (repeatable).
  --prune                 Delete local files that are absent from the source manifest.
  --dry-run               Show what would happen without writing anything.
  --yes                   Skip the confirmation prompt (the overwrite warning
                          is still printed)."
    );
}

fn parse_args() -> Result<Args, String> {
    let mut args = Args::default();
    let mut iter = std::env::args().skip(1);
    while let Some(flag) = iter.next() {
        let mut value = |name: &str| -> Result<String, String> {
            iter.next().ok_or_else(|| format!("{name} needs a value"))
        };
        match flag.as_str() {
            "--url" => args.url = Some(value("--url")?),
            "--key" => args.key = Some(value("--key")?),
            "--env" => args.env_file = Some(PathBuf::from(value("--env")?)),
            "--db" => args.db_path = Some(PathBuf::from(value("--db")?)),
            "--media-dir" => args.media_dir = Some(PathBuf::from(value("--media-dir")?)),
            "--demos-dir" => args.demos_dir = Some(PathBuf::from(value("--demos-dir")?)),
            "--skip" => {
                let what = value("--skip")?;
                if !matches!(what.as_str(), "media" | "demos" | "artifacts") {
                    return Err(format!(
                        "--skip must be media, demos, or artifacts (got '{what}')"
                    ));
                }
                args.skip.insert(what);
            }
            "--prune" => args.prune = true,
            "--dry-run" => args.dry_run = true,
            "--yes" => args.yes = true,
            "--help" | "-h" => {
                print_usage();
                std::process::exit(0);
            }
            other => return Err(format!("unknown argument '{other}' (--help shows usage)")),
        }
    }
    Ok(args)
}

/// Reads KEY=VALUE lines from a .env file without touching real env vars.
fn read_env_file(path: &Path) -> Result<(Option<String>, Option<String>, Option<String>), String> {
    let mut database_url = None;
    let mut media_path = None;
    let mut demos_path = None;
    let content = std::fs::read_to_string(path)
        .map_err(|e| format!("cannot read {}: {e}", path.display()))?;
    for line in content.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        let Some((key, value)) = line.split_once('=') else {
            continue;
        };
        let value = value.trim().trim_matches('"').to_string();
        match key.trim() {
            "DATABASE_URL" => database_url = Some(value),
            "MEDIA_PATH" => media_path = Some(value),
            "PROJECT_DEMOS_PATH" => demos_path = Some(value),
            _ => {}
        }
    }
    Ok((database_url, media_path, demos_path))
}

fn resolve_key(key: &str) -> Result<String, String> {
    if let Some(path) = key.strip_prefix('@') {
        let content =
            std::fs::read_to_string(path).map_err(|e| format!("cannot read key file {path}: {e}"))?;
        Ok(content.trim().to_string())
    } else {
        Ok(key.to_string())
    }
}

/// Mirrors the backend's DATABASE_URL parsing: `sqlite:<path>`.
fn database_path_from_url(url: &str) -> Result<PathBuf, String> {
    url.strip_prefix("sqlite:")
        .map(PathBuf::from)
        .ok_or_else(|| format!("unsupported DATABASE_URL '{url}' (only sqlite: is supported)"))
}

fn format_bytes(size: u64) -> String {
    const UNITS: [&str; 5] = ["B", "KiB", "MiB", "GiB", "TiB"];
    let mut value = size as f64;
    let mut unit = 0;
    while value >= 1024.0 && unit < UNITS.len() - 1 {
        value /= 1024.0;
        unit += 1;
    }
    if unit == 0 {
        format!("{size} B")
    } else {
        format!("{value:.1} {}", UNITS[unit])
    }
}

/// Downloads `url` into `target` (temp file + rename). Returns Ok(false) when
/// the file already exists with the expected size, Ok(true) when it was
/// (or would be) downloaded.
async fn download_to_file(
    client: &reqwest::Client,
    url: &str,
    key: &str,
    target: &Path,
    expected_size: Option<u64>,
    dry_run: bool,
) -> Result<bool, String> {
    if let Some(size) = expected_size
        && target.is_file()
        && let Ok(meta) = tokio::fs::metadata(target).await
        && meta.len() == size
    {
        return Ok(false);
    }
    if dry_run {
        return Ok(true);
    }
    if let Some(parent) = target.parent() {
        tokio::fs::create_dir_all(parent)
            .await
            .map_err(|e| format!("create {}: {e}", parent.display()))?;
    }
    let response = client
        .get(url)
        .bearer_auth(key)
        .send()
        .await
        .map_err(|e| format!("GET {url}: {e}"))?;
    if !response.status().is_success() {
        let status = response.status();
        let body = response.text().await.unwrap_or_default();
        return Err(format!("GET {url}: {status} {}", body.trim()));
    }
    let temp = target.with_extension(format!(
        "{}.sync-tmp",
        target
            .extension()
            .and_then(|e| e.to_str())
            .unwrap_or_default()
    ));
    let mut file = tokio::fs::File::create(&temp)
        .await
        .map_err(|e| format!("create {}: {e}", temp.display()))?;
    let mut stream = response.bytes_stream();
    while let Some(chunk) = stream.next().await {
        let bytes = chunk.map_err(|e| format!("stream {url}: {e}"))?;
        file.write_all(&bytes)
            .await
            .map_err(|e| format!("write {}: {e}", temp.display()))?;
    }
    file.flush()
        .await
        .map_err(|e| format!("flush {}: {e}", temp.display()))?;
    drop(file);
    tokio::fs::rename(&temp, target)
        .await
        .map_err(|e| format!("finalize {}: {e}", target.display()))?;
    Ok(true)
}

fn collect_local_files(dir: &Path) -> Vec<(PathBuf, u64)> {
    let mut out = Vec::new();
    let mut stack = vec![dir.to_path_buf()];
    while let Some(current) = stack.pop() {
        let Ok(entries) = std::fs::read_dir(&current) else {
            continue;
        };
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                stack.push(path);
            } else if path.is_file() {
                let size = entry.metadata().map(|m| m.len()).unwrap_or(0);
                out.push((path, size));
            }
        }
    }
    out
}

async fn prune_extras(local_dir: &Path, expected: &HashSet<String>, dry_run: bool) -> (usize, u64) {
    let mut removed = 0usize;
    let mut freed = 0u64;
    for (path, size) in collect_local_files(local_dir) {
        let Ok(relative) = path.strip_prefix(local_dir) else {
            continue;
        };
        if !expected.contains(&relative.to_string_lossy().to_string()) {
            freed += size;
            removed += 1;
            if !dry_run {
                let _ = tokio::fs::remove_file(&path).await;
            }
        }
    }
    (removed, freed)
}

#[tokio::main]
async fn main() {
    if let Err(error) = run().await {
        eprintln!("\nsync-pull failed: {error}");
        std::process::exit(1);
    }
}

async fn run() -> Result<(), String> {
    let args = parse_args()?;

    // ── Resolve configuration ────────────────────────────────────────────
    let env_file = match &args.env_file {
        Some(path) => Some(path.clone()),
        None => ["backend/.env", ".env"]
            .iter()
            .map(PathBuf::from)
            .find(|p| p.is_file()),
    };
    let (env_db, env_media, env_demos) = match &env_file {
        Some(path) => {
            let values = read_env_file(path)?;
            println!("Using env file {}", path.display());
            values
        }
        None => {
            println!("No .env file found; pass --db/--media-dir/--demos-dir explicitly.");
            (None, None, None)
        }
    };

    let db_path = match (&args.db_path, &env_db) {
        (Some(path), _) => path.clone(),
        (None, Some(url)) => database_path_from_url(url)?,
        (None, None) => return Err("no database path: pass --db or --env".to_string()),
    };
    let media_dir = args
        .media_dir
        .clone()
        .or_else(|| env_media.clone().map(PathBuf::from))
        .ok_or("no media directory: pass --media-dir or --env")?;
    let demos_dir = args
        .demos_dir
        .clone()
        .or_else(|| env_demos.clone().map(PathBuf::from))
        .ok_or("no project-demos directory: pass --demos-dir or --env")?;

    let base_url = args
        .url
        .clone()
        .ok_or("--url is required (public URL of the source site)")?;
    let base_url = base_url.trim_end_matches('/').to_string();
    let key = resolve_key(
        args.key
            .as_deref()
            .ok_or("--key is required (bsk_… or @file)")?,
    )?;
    if key.is_empty() {
        return Err("the sync key is empty".to_string());
    }

    let client = reqwest::Client::builder()
        .user_agent("blog-sync-pull")
        .build()
        .map_err(|e| e.to_string())?;

    // ── Manifest ─────────────────────────────────────────────────────────
    let started = Instant::now();
    println!("Fetching manifest from {base_url} …");

    // The source is normally reached through the site (frontend proxies
    // /api/* to the backend); hitting a backend directly also works, so try
    // both prefixes and stick with whichever answers.
    let (api, response) = 'probe: {
        for prefix in ["/api", ""] {
            let attempt = client
                .get(format!("{base_url}{prefix}/sync/manifest"))
                .bearer_auth(&key)
                .send()
                .await;
            match attempt {
                Ok(response) if response.status() == reqwest::StatusCode::NOT_FOUND => continue,
                Ok(response) => {
                    let api =
                        move |section: &str| format!("{base_url}{prefix}/sync/{section}");
                    break 'probe (api, response);
                }
                Err(e) => return Err(format!("manifest request: {e}")),
            }
        }
        return Err(format!(
            "no sync endpoints found at {base_url} (tried /api/sync and /sync)"
        ));
    };

    if !response.status().is_success() {
        let status = response.status();
        let body = response.text().await.unwrap_or_default();
        return Err(format!("manifest: {status} {}", body.trim()));
    }
    let manifest: SyncManifest = response.json().await.map_err(|e| e.to_string())?;

    let media_total: u64 = manifest.media.iter().map(|m| m.size.max(0) as u64).sum();
    let demos_total: u64 = manifest
        .project_demos
        .iter()
        .chain(manifest.game_demos.iter())
        .flat_map(|dir| dir.files.iter())
        .map(|f| f.size)
        .sum();
    let artifacts_total: u64 = manifest.artifacts.iter().map(|a| a.size).sum();

    println!();
    println!("Source environment        : {} backend", manifest.storage_backend);
    println!("Database                  : {}", format_bytes(manifest.database_size_bytes));
    println!(
        "Media files               : {:>5}  ({})",
        manifest.media.len(),
        format_bytes(media_total)
    );
    println!(
        "Demo files                : {:>5}  ({}) across {} project / {} game dirs",
        manifest
            .project_demos
            .iter()
            .chain(manifest.game_demos.iter())
            .map(|d| d.files.len())
            .sum::<usize>(),
        format_bytes(demos_total),
        manifest.project_demos.len(),
        manifest.game_demos.len()
    );
    println!(
        "Game artifacts (jsdos/v86): {:>5}  ({})",
        manifest.artifacts.len(),
        format_bytes(artifacts_total)
    );
    println!("Generated at              : {}", manifest.generated_at);
    println!();

    // ── Overwrite warning (printed on every run, even with --yes) ────────
    if args.dry_run {
        println!("Dry run: nothing will be written.");
    } else {
        println!("──────────────────────────────────────────────────────────────");
        println!("  WARNING: this sync OVERWRITES local data to mirror the source.");
        println!("──────────────────────────────────────────────────────────────");
        println!(
            "  • The local database {} will be REPLACED wholesale.",
            db_path.display()
        );
        println!(
            "    Anything that exists only locally — drafts, posts, media,\n    uploads, users — will be GONE. A copy of the current database\n    is kept aside next to it as *.pre-sync-*."
        );
        println!(
            "  • Every synced file under {} and\n    {} is overwritten to match the source:\n    media, demo files, js-dos and v86 artifacts.",
            media_dir.display(),
            demos_dir.display()
        );
        if args.prune {
            println!("  • --prune is set: local files absent from the source are DELETED.");
        }
        println!("  • The source environment is never modified.");
        if !args.yes {
            println!("──────────────────────────────────────────────────────────────");
            print!("Type SYNC to continue: ");
            use std::io::Write as _;
            std::io::stdout().flush().map_err(|e| e.to_string())?;
            let mut answer = String::new();
            std::io::stdin()
                .read_line(&mut answer)
                .map_err(|e| e.to_string())?;
            if answer.trim() != "SYNC" {
                return Err("aborted".to_string());
            }
        } else {
            println!("  Proceeding without prompt (--yes).");
            println!("──────────────────────────────────────────────────────────────");
        }
    }

    let mut downloaded = 0usize;
    let mut skipped = 0usize;
    let mut transferred: u64 = 0;

    // ── Database ─────────────────────────────────────────────────────────
    if args.dry_run {
        println!("[dry-run] would replace database {}", db_path.display());
    } else {
        println!("\n── Database ──");
        let snapshot = db_path.with_extension(format!(
            "{}.sync-tmp",
            db_path
                .extension()
                .and_then(|e| e.to_str())
                .unwrap_or_default()
        ));
        download_to_file(&client, &api("database"), &key, &snapshot, None, false)
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
        let summary = fix_imported_database(&pool, &media_dir, &demos_dir)
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
            format_bytes(manifest.database_size_bytes),
            summary.media_urls_fixed,
            summary.project_demo_urls_fixed,
            summary.game_demo_urls_fixed
        );
    }

    // ── Media ────────────────────────────────────────────────────────────
    if !args.skip.contains("media") {
        println!("\n── Media ──");
        if args.dry_run {
            println!(
                "[dry-run] would sync {} files under {}",
                manifest.media.len(),
                media_dir.display()
            );
        } else {
            let mut expected = HashSet::new();
            let count = manifest.media.len();
            for entry in &manifest.media {
                let target = media_dir.join(&entry.path);
                expected.insert(entry.path.clone());
                match download_to_file(
                    &client,
                    &api(&format!("media/{}", entry.hash)),
                    &key,
                    &target,
                    Some(entry.size.max(0) as u64),
                    false,
                )
                .await
                {
                    Ok(true) => {
                        downloaded += 1;
                        transferred += entry.size.max(0) as u64;
                    }
                    Ok(false) => skipped += 1,
                    Err(e) => return Err(e),
                }
                let done = downloaded + skipped;
                if done % 200 == 0 && done < count {
                    println!("  {done}/{count} …");
                }
            }
            println!("  {downloaded} downloaded, {skipped} already up to date");
            if args.prune {
                let (removed, freed) = prune_extras(&media_dir, &expected, false).await;
                println!(
                    "  pruned {removed} extra file(s), freed {}",
                    format_bytes(freed)
                );
            }
        }
    }

    // ── Demo files ───────────────────────────────────────────────────────
    if !args.skip.contains("demos") {
        println!("\n── Demo files ──");
        let mut expected_per_dir: Vec<(PathBuf, HashSet<String>)> = Vec::new();
        for (kind, dirs) in [
            ("project", &manifest.project_demos),
            ("game", &manifest.game_demos),
        ] {
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
                    match download_to_file(&client, &api(&section), &key, &target, Some(file.size), false)
                        .await
                    {
                        Ok(true) => {
                            downloaded += 1;
                            transferred += file.size;
                        }
                        Ok(false) => skipped += 1,
                        Err(e) => return Err(e),
                    }
                }
                expected_per_dir.push((base, local));
            }
        }
        println!("  {downloaded} downloaded (cumulative), {skipped} already up to date");
        if args.prune {
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
    }

    // ── Artifacts (js-dos bundles + v86 disks/ISOs/snapshots/saves) ──────
    if !args.skip.contains("artifacts") {
        println!("\n── Game artifacts ──");
        if args.dry_run {
            println!(
                "[dry-run] would sync {} artifacts under {}",
                manifest.artifacts.len(),
                demos_dir.display()
            );
        } else {
            let mut expected = HashSet::new();
            for entry in &manifest.artifacts {
                expected.insert(entry.key.clone());
                let target = demos_dir.join(&entry.key);
                match download_to_file(
                    &client,
                    &api(&format!("artifact/{}", entry.key)),
                    &key,
                    &target,
                    Some(entry.size),
                    false,
                )
                .await
                {
                    Ok(true) => {
                        downloaded += 1;
                        transferred += entry.size;
                    }
                    Ok(false) => skipped += 1,
                    Err(e) => return Err(e),
                }
            }
            println!("  {downloaded} downloaded (cumulative), {skipped} already up to date");
            if args.prune {
                // Anything under v86/ or jsdos/ that the manifest does not list
                // is a leftover (including transient v86/tmp files).
                let mut local = HashSet::new();
                for prefix in ["v86", "jsdos"] {
                    let dir = demos_dir.join(prefix);
                    if !dir.is_dir() {
                        continue;
                    }
                    for (path, _) in collect_local_files(&dir) {
                        if let Ok(relative) = path.strip_prefix(&demos_dir) {
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
        }
    }

    println!();
    println!(
        "Done in {:.1}s — {} file(s) downloaded ({}), {} already up to date.",
        started.elapsed().as_secs_f32(),
        downloaded,
        format_bytes(transferred),
        skipped
    );
    if !args.dry_run {
        println!("Restart the local backend so it picks up the new database.");
    }
    Ok(())
}

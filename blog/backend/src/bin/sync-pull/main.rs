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

use std::path::{Path, PathBuf};
use std::time::Instant;

use backend::infrastructure::sync::SyncManifest;

mod cli;
mod download;
mod rewrite;
#[cfg(test)]
mod tests;
mod transfer;

pub use transfer::{Counters, Session};

use cli::parse_args;
use rewrite::{database_path_from_url, format_bytes, read_env_file, resolve_key, update_env_file};

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
                    let api = move |section: &str| format!("{base_url}{prefix}/sync/{section}");
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
    println!(
        "Source environment        : {} backend",
        manifest.storage_backend
    );
    println!(
        "Database                  : {}",
        format_bytes(manifest.database_size_bytes)
    );
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
            "    Anything that exists only locally — drafts, posts, media,\n    uploads, users — will be GONE. A copy of the current database\n    is kept aside next to it as *.pre-sync-*.\""
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

    let session = Session {
        client: &client,
        api: &api,
        key: &key,
    };
    let mut counters = Counters::default();

    // ── Database ─────────────────────────────────────────────────────────
    if args.dry_run {
        println!("[dry-run] would replace database {}", db_path.display());
    } else {
        transfer::sync_database(
            &session,
            &db_path,
            &media_dir,
            &demos_dir,
            manifest.database_size_bytes,
        )
        .await?;
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
            transfer::sync_media(
                &session,
                &media_dir,
                &manifest.media,
                args.prune,
                &mut counters,
            )
            .await?;
        }
    }

    // ── Demo files ───────────────────────────────────────────────────────
    if !args.skip.contains("demos") {
        println!("\n── Demo files ──");
        transfer::sync_demo_dirs(
            &session,
            &demos_dir,
            &manifest.project_demos,
            &manifest.game_demos,
            args.prune,
            &mut counters,
        )
        .await?;
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
            transfer::sync_artifacts(
                &session,
                &demos_dir,
                &manifest.artifacts,
                args.prune,
                &mut counters,
            )
            .await?;
        }
    }

    // ── Local .env ───────────────────────────────────────────────────────
    // Write the layout this pull actually used back into the .env (paths as
    // local values, storage on fs since artifacts land on disk), so the local
    // backend serves what was just synced. Secrets and other keys are kept.
    if args.dry_run {
        println!("[dry-run] would update .env with the resolved paths");
    } else {
        let env_target = env_file.clone().unwrap_or_else(|| PathBuf::from(".env"));
        if !env_target.is_file() {
            let example = env_target
                .parent()
                .unwrap_or(Path::new("."))
                .join("example.env");
            if example.is_file() {
                std::fs::copy(&example, &env_target).map_err(|e| {
                    format!(
                        "seed {} from {}: {e}",
                        env_target.display(),
                        example.display()
                    )
                })?;
                println!("seeded {} from {}", env_target.display(), example.display());
            }
        }
        update_env_file(
            &env_target,
            &[
                ("DATABASE_URL", format!("sqlite:{}", db_path.display())),
                ("MEDIA_PATH", media_dir.to_string_lossy().into_owned()),
                (
                    "PROJECT_DEMOS_PATH",
                    demos_dir.to_string_lossy().into_owned(),
                ),
                ("STORAGE_BACKEND", "fs".to_string()),
            ],
        )?;
        println!(
            "updated {} (DATABASE_URL, MEDIA_PATH, PROJECT_DEMOS_PATH, STORAGE_BACKEND=fs)",
            env_target.display()
        );
        let env_now = std::fs::read_to_string(&env_target).unwrap_or_default();
        if env_now.contains("REPLACE_WITH_YOUR_JWT_SECRET") {
            println!(
                "  note: set a real JWT_SECRET in {} before running the backend",
                env_target.display()
            );
        }
    }

    println!();
    println!(
        "Done in {:.1}s — {} file(s) downloaded ({}), {} already up to date, {} missing on source.",
        started.elapsed().as_secs_f32(),
        counters.downloaded,
        format_bytes(counters.transferred),
        counters.skipped,
        counters.missing
    );
    if !args.dry_run {
        println!("Restart the local backend so it picks up the new database.");
    }
    Ok(())
}

//! Backfills and reconciles the audio bucket against the database.
//!
//! **Scope: the audio an audiobook track plays, and nothing else.** That is the
//! media rows referenced by `audiobook_tracks.media_id` — an indexed foreign
//! key, and the only trustworthy definition available:
//!
//!   * the content type alone is too wide. It also matches a media-library
//!     sound file (`l-game-loop`) and the media of tracks that have since been
//!     replaced, which `replace_track_medium` deliberately leaves in place
//!     rather than deleting;
//!   * `media.description` is free text — the table holds NULLs and
//!     `'undefined'` in it — so it cannot be trusted either.
//!
//! Audio that no track plays is counted and reported, never uploaded. Images,
//! covers, avatars, video and models are not audio at all and are not looked at.
//!
//! The storage key cannot be expressed in SQL — the extension comes from a
//! content-type mapping and the three special layouts split the hash — so the
//! row query below is deliberately the dumb one (`media` rows with a hash) and
//! the scope and the key are both decided in Rust, by `is_audio` and
//! `media_key`.
//!
//!   cd blog/backend
//!   cargo run --bin backfill-audio                # report + reconcile, no writes
//!   cargo run --bin backfill-audio -- --upload    # copy the missing objects up
//!
//! **On the VM this runs inside the backend container, not from a checkout.** It
//! ships in the image (`Dockerfile`), built in CI and pulled — never built on the
//! VM, which has no Rust toolchain and no R2 CLI. The wrapper the VM calls is
//! `./backfill-audio.sh`, which is a `docker compose exec` and nothing else:
//!
//!   cd ~/MyPage/blog/backend && ./backfill-audio.sh --upload
//!
//! That is the whole point of doing it here rather than in shell: the key comes
//! from `storage::media_key` and the object metadata from the same code as the
//! write path, so there is no second implementation of the layout to drift.
//! A shell re-implementation is a real trap — `audio/mpeg` is `.mp3`, not
//! `.mpeg`, so deriving the extension as `".${file_type#audio/}"` names a file
//! that does not exist.
//!
//! `--upload` only ever writes objects that the bucket is missing or holds at
//! the wrong size, and it never deletes anything. Exits non-zero when a track's
//! bytes are not recoverable from the bucket, so it can gate the cutover.
//!
//! It addresses the bucket through the `R2_*` variables and **not** through
//! `AUDIO_BACKEND`: filling the bucket is a separate concern from serving out of
//! it, and this is meant to run *before* that switch is flipped.
//!
//! The bucket is shared with the v86 artifacts, so everything here is scoped to
//! the `audio/` prefix — the listing, the uploads, and the orphan report.

use std::collections::{HashMap, HashSet};
use std::path::PathBuf;
use std::process::ExitCode;

use sqlx::SqlitePool;

use backend::infrastructure::storage::{
    AUDIO_OBJECT_PREFIX, MEDIA_CACHE_CONTROL, S3Settings, audio_object_key, is_audio, media_key,
    r2::R2Client,
};

/// How many examples of each problem to print before summarizing the rest.
const EXAMPLE_LIMIT: usize = 20;

struct Args {
    env_file: Option<PathBuf>,
    media_dir: Option<PathBuf>,
    upload: bool,
}

struct Row {
    hash: String,
    file_type: String,
    uploader_id: i64,
    size: u64,
    /// Whether an `audiobook_tracks` row points at this media row.
    is_track: bool,
}

#[tokio::main]
async fn main() -> ExitCode {
    match run().await {
        Ok(code) => code,
        Err(message) => {
            eprintln!("\nbackfill-audio failed: {message}");
            ExitCode::FAILURE
        }
    }
}

async fn run() -> Result<ExitCode, String> {
    let args = parse_args()?;

    // ── Configuration ────────────────────────────────────────────────────
    let env_file = match &args.env_file {
        Some(path) => Some(path.clone()),
        None => ["backend/.env", ".env"]
            .iter()
            .map(PathBuf::from)
            .find(|path| path.is_file()),
    };
    if let Some(path) = &env_file {
        dotenvy::from_filename(path).map_err(|e| format!("read {}: {e}", path.display()))?;
        println!("Using env file {}", path.display());
    }

    let media_dir = match args.media_dir {
        Some(dir) => dir,
        None => PathBuf::from(
            std::env::var("MEDIA_PATH")
                .map_err(|_| "MEDIA_PATH is not set: pass --env <path> or export it".to_string())?,
        ),
    };

    // Built from the `R2_*` variables rather than from `MediaConfig`, so this
    // does **not** require `AUDIO_BACKEND=r2`. The point of the backfill is to
    // fill the bucket *before* the serving switch is flipped, so the flip has
    // nothing left to fall back to; requiring the switch would invert that and
    // make the bucket load-bearing while it is still empty.
    let bucket = R2Client::from_settings(
        S3Settings::require_r2_env()
            .map_err(|missing| format!("no bucket to backfill: {missing}"))?,
    );

    let db_url = std::env::var("DATABASE_URL")
        .map_err(|_| "DATABASE_URL is not set: pass --env <path> or export it".to_string())?;
    let pool = SqlitePool::connect(&db_url)
        .await
        .map_err(|e| format!("connect {db_url}: {e}"))?;

    // ── Inputs ───────────────────────────────────────────────────────────
    // One pass over `media`, carrying the relational fact alongside the row.
    // The `EXISTS` uses `idx_audiobook_tracks_media`; whether a row is *audio*
    // is decided in Rust by `is_audio`, so that rule lives in exactly one place
    // and is not restated here as a `LIKE`.
    let rows: Vec<Row> = sqlx::query_as::<_, (String, String, i64, i64, i64)>(
        "SELECT m.hash, m.file_type, COALESCE(m.uploader_id, 0), COALESCE(m.size, 0),
                EXISTS (SELECT 1 FROM audiobook_tracks t WHERE t.media_id = m.id)
         FROM media m
         WHERE m.hash IS NOT NULL AND m.hash != ''",
    )
    .fetch_all(&pool)
    .await
    .map_err(|e| format!("read media rows: {e}"))?
    .into_iter()
    .map(|(hash, file_type, uploader_id, size, is_track)| Row {
        hash,
        file_type,
        uploader_id,
        size: size.max(0) as u64,
        is_track: is_track != 0,
    })
    .collect();

    // Split the audio rows into the ones this binary is for and the ones it
    // deliberately leaves alone. Anything that is not audio at all drops out
    // here and is never mentioned again.
    let mut targets: Vec<&Row> = Vec::new();
    let mut skipped_audio = 0usize;
    let mut skipped_bytes = 0u64;
    for row in &rows {
        if !is_audio(&row.file_type) {
            continue;
        }
        if !row.is_track {
            skipped_audio += 1;
            skipped_bytes += row.size;
            continue;
        }
        targets.push(row);
    }

    println!("Bucket                    : {}", bucket.bucket);
    println!("Prefix                    : {AUDIO_OBJECT_PREFIX}/");
    println!("Media directory           : {}", media_dir.display());
    println!("Media rows                : {}", rows.len());
    println!(
        "Played by a track         : {} audio row(s), {}",
        targets.len(),
        human(targets.iter().map(|row| row.size).sum())
    );
    if skipped_audio > 0 {
        println!(
            "Audio nothing plays       : {skipped_audio} row(s), {} — left on disk",
            human(skipped_bytes)
        );
    }

    // One listing for the whole run; uploads below update it in place. Scoped
    // to the prefix so the v86 artifacts sharing the bucket never appear.
    let mut objects: HashMap<String, u64> = bucket
        .list_prefix(AUDIO_OBJECT_PREFIX)
        .await
        .map_err(|e| format!("list {AUDIO_OBJECT_PREFIX}/: {e}"))?
        .into_iter()
        .collect();
    println!("Objects under the prefix  : {}", objects.len());
    println!();

    // ── Walk the rows ────────────────────────────────────────────────────
    let mut unmappable: Vec<String> = Vec::new();
    let mut not_audio: Vec<String> = Vec::new();
    let mut missing_on_disk: Vec<String> = Vec::new();
    let mut missing_in_bucket: Vec<String> = Vec::new();
    let mut size_mismatch: Vec<String> = Vec::new();
    let mut claimed: HashSet<String> = HashSet::new();
    let mut uploaded = 0usize;
    let mut uploaded_bytes = 0u64;

    for row in &targets {
        let Some(key) = media_key(&row.hash, &row.file_type, row.uploader_id) else {
            unmappable.push(format!("{} ({})", row.hash, row.file_type));
            continue;
        };
        // A track's medium is validated as audio when it is uploaded, so this
        // cannot normally fire. If it does, the row is broken rather than out
        // of scope, and saying so beats skipping it silently.
        let Some(object) = audio_object_key(&key.key, &row.file_type) else {
            not_audio.push(format!("{} ({})", row.hash, row.file_type));
            continue;
        };
        claimed.insert(object.clone());

        let disk_path = media_dir.join(&key.key);
        let on_disk = tokio::fs::metadata(&disk_path)
            .await
            .ok()
            .map(|meta| meta.len());

        // The disk copy is the better authority on the expected size; the
        // column is only used when the file is gone.
        let expected = on_disk.or((row.size > 0).then_some(row.size));
        let mut in_bucket = objects.get(&object).copied();
        let mut ok = in_bucket.is_some() && expected.is_none_or(|size| in_bucket == Some(size));

        if !ok
            && args.upload
            && let Some(size) = on_disk
        {
            bucket
                .put_file_with_metadata(&object, &disk_path, &row.file_type, MEDIA_CACHE_CONTROL)
                .await
                .map_err(|e| format!("upload {object}: {e}"))?;
            objects.insert(object.clone(), size);
            in_bucket = Some(size);
            ok = expected.is_none_or(|expected| in_bucket == Some(expected));
            uploaded += 1;
            uploaded_bytes += size;
        }

        if on_disk.is_none() {
            missing_on_disk.push(object.clone());
        }
        if !ok {
            match in_bucket {
                None => missing_in_bucket.push(object),
                Some(_) => size_mismatch.push(object),
            }
        }
    }

    // Objects nothing points at. Audio is never replaced in place and never
    // deleted, so this should be empty; a non-empty list means a manual
    // `aws s3 rm` or an interrupted upload.
    let orphans: Vec<String> = objects
        .keys()
        .filter(|key| !claimed.contains(*key))
        .cloned()
        .collect();

    // ── Report ───────────────────────────────────────────────────────────
    if uploaded > 0 {
        println!(
            "Uploaded                  : {uploaded} object(s), {}",
            human(uploaded_bytes)
        );
        println!();
    }

    let unrecoverable =
        unmappable.len() + not_audio.len() + missing_in_bucket.len() + size_mismatch.len();
    let report = |title: &str, items: &[String]| {
        if items.is_empty() {
            return;
        }
        println!("{title} ({}):", items.len());
        for item in items.iter().take(EXAMPLE_LIMIT) {
            println!("  {item}");
        }
        if items.len() > EXAMPLE_LIMIT {
            println!("  … and {} more", items.len() - EXAMPLE_LIMIT);
        }
        println!();
    };

    report("Track keys not derivable", &unmappable);
    report("Track media that is not audio", &not_audio);
    report("Missing on disk", &missing_on_disk);
    report("Missing in bucket", &missing_in_bucket);
    report("Wrong size in bucket", &size_mismatch);
    report("In bucket but no track", &orphans);

    if missing_on_disk.is_empty() {
        println!("Every track's audio has a disk copy.");
    }
    if unrecoverable == 0 {
        println!(
            "Every track's audio is in the bucket at the right size. The disk copy is no longer load-bearing."
        );
        return Ok(ExitCode::SUCCESS);
    }

    println!(
        "{unrecoverable} row(s) are NOT recoverable from the bucket. Do not delete the disk copy."
    );
    Ok(ExitCode::FAILURE)
}

fn parse_args() -> Result<Args, String> {
    let mut args = Args {
        env_file: None,
        media_dir: None,
        upload: false,
    };

    let mut argv = std::env::args().skip(1);
    while let Some(arg) = argv.next() {
        match arg.as_str() {
            "--upload" => args.upload = true,
            "--env" => {
                args.env_file = Some(PathBuf::from(argv.next().ok_or("--env needs a path")?));
            }
            "--media-dir" => {
                args.media_dir = Some(PathBuf::from(
                    argv.next().ok_or("--media-dir needs a path")?,
                ));
            }
            "--help" | "-h" => {
                println!("{HELP}");
                std::process::exit(0);
            }
            other => return Err(format!("unknown argument '{other}'\n\n{HELP}")),
        }
    }

    Ok(args)
}

const HELP: &str = "\
Backfill and reconcile the audio bucket against the database.

Usage: cargo run --bin backfill-audio -- [options]

  --upload              copy objects the bucket is missing or holds at the
                        wrong size (default: report only, no writes)
  --env <path>          backend .env to read DATABASE_URL / MEDIA_PATH / R2_*
                        from (default: backend/.env, then .env)
  --media-dir <path>    override MEDIA_PATH

Scope is the audio an audiobook track plays: media referenced by
audiobook_tracks.media_id. Audio that no track plays is counted but not
uploaded, and images, covers, avatars, video and models are not considered at
all. Only the audio/ prefix is read or written, so the v86 artifacts sharing the
bucket are left alone. Exits non-zero when a track's bytes are not recoverable
from the bucket.

The bucket comes from R2_*, not from AUDIO_BACKEND, so this runs before the
serving switch is flipped.";

fn human(bytes: u64) -> String {
    const UNITS: [&str; 4] = ["B", "KiB", "MiB", "GiB"];
    let mut value = bytes as f64;
    let mut unit = 0;
    while value >= 1024.0 && unit + 1 < UNITS.len() {
        value /= 1024.0;
        unit += 1;
    }
    if unit == 0 {
        format!("{bytes} B")
    } else {
        format!("{value:.1} {}", UNITS[unit])
    }
}

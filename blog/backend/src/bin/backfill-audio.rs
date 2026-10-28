use std::collections::{BTreeMap, HashMap, HashSet};
use std::path::{Path, PathBuf};
use std::process::ExitCode;
use std::str::FromStr;

use sha2::{Digest, Sha256};
use sqlx::{SqliteConnection, sqlite::SqliteConnectOptions, sqlite::SqlitePoolOptions};

use backend::infrastructure::persistence::media::MEDIA_REFERENCES;
use backend::infrastructure::storage::{
    AUDIO_OBJECT_PREFIX, MEDIA_CACHE_CONTROL, S3Settings, audio_object_key, is_audio, media_key,
    r2::{ListedObject, R2Client},
};

const MIN_AGE_SECONDS: i64 = 24 * 60 * 60;
const TRACK_HANDLE_PREFIX: &str = "abt-";

#[derive(Debug, Default)]
struct Args {
    env_file: Option<PathBuf>,
    media_dir: Option<PathBuf>,
    upload: bool,
    prune: Option<String>,
}

#[derive(Clone)]
struct Row {
    hash: String,
    file_type: String,
    uploader_id: i64,
    size: u64,
    short_name: String,
    is_track: bool,
    in_use: bool,
}

struct Plan {
    objects: Vec<ListedObject>,
    unclaimed: Vec<String>,
}

type Inventory = BTreeMap<String, ListedObject>;

#[tokio::main]
async fn main() -> ExitCode {
    match run().await {
        Ok(code) => code,
        Err(message) => {
            eprintln!("backfill-audio failed: {message}");
            ExitCode::FAILURE
        }
    }
}

async fn run() -> Result<ExitCode, String> {
    let args = parse_args(std::env::args().skip(1))?;
    let env_file = args.env_file.clone().or_else(|| {
        ["backend/.env", ".env"]
            .iter()
            .map(PathBuf::from)
            .find(|path| path.is_file())
    });
    if let Some(path) = &env_file {
        dotenvy::from_filename(path).map_err(|e| format!("read {}: {e}", path.display()))?;
    }
    let media_dir = match &args.media_dir {
        Some(dir) => dir.clone(),
        None => PathBuf::from(
            std::env::var("MEDIA_PATH")
                .map_err(|_| "MEDIA_PATH is not set: pass --env <path> or export it")?,
        ),
    };
    let settings = S3Settings::require_r2_env()
        .map_err(|missing| format!("no bucket to backfill: {missing}"))?;
    let endpoint = settings.endpoint.clone();
    let bucket = R2Client::from_settings(settings);
    let db_url = std::env::var("DATABASE_URL")
        .map_err(|_| "DATABASE_URL is not set: pass --env <path> or export it")?;
    let writes = args.upload || args.prune.is_some();
    let options = SqliteConnectOptions::from_str(&db_url)
        .map_err(|e| format!("database options: {e}"))?
        .create_if_missing(false)
        .read_only(!writes);
    let pool = SqlitePoolOptions::new()
        .max_connections(1)
        .connect_with(options)
        .await
        .map_err(|e| format!("connect database: {e}"))?;
    // Upload writers take this same lock before writing bytes, not just before inserting rows.
    let mut tx = pool
        .begin_with(if writes { "BEGIN IMMEDIATE" } else { "BEGIN" })
        .await
        .map_err(|e| format!("lock database: {e}"))?;
    let rows = read_rows(&mut tx).await?;
    let prefix = format!("{AUDIO_OBJECT_PREFIX}/");
    let mut objects = inventory(
        bucket
            .list_prefix_with_metadata(&prefix)
            .await
            .map_err(|e| format!("list {prefix}: {e}"))?,
    )?;

    println!("Bucket: {}", bucket.bucket);
    println!("Prefix: {prefix}");
    println!(
        "Media rows: {}; bucket objects: {}",
        rows.len(),
        objects.len()
    );
    let unrecoverable = reconcile(&rows, &objects, &bucket, &media_dir, args.upload).await?;
    if args.upload {
        objects = inventory(
            bucket
                .list_prefix_with_metadata(&prefix)
                .await
                .map_err(|e| format!("list after upload: {e}"))?,
        )?;
    }
    let plan = build_plan(&rows, &objects, chrono::Utc::now().timestamp())?;
    let fingerprint = fingerprint(&endpoint, &bucket.bucket, &plan)?;
    println!("\nExact R2 deletion plan (key, bytes, ETag, modified_at):");
    for object in &plan.objects {
        println!(
            "{}",
            serde_json::to_string(object).map_err(|e| e.to_string())?
        );
    }
    let bytes: u64 = plan.objects.iter().map(|object| object.size).sum();
    println!("Selected: {} objects, {bytes} bytes", plan.objects.len());
    println!("Fingerprint: {fingerprint}");
    println!(
        "Unknown ownership/layout: {} objects (retained)",
        plan.unclaimed.len()
    );
    for key in plan.unclaimed.iter().take(20) {
        println!("  manual review: {key}");
    }
    println!("Database rows and local files are never removed by this command.");

    if let Some(expected) = &args.prune {
        confirm_plan(expected, &fingerprint)?;
        if unrecoverable > 0 {
            return Err("live audio reconciliation failed; nothing deleted".into());
        }
        let keys: Vec<_> = plan
            .objects
            .iter()
            .map(|object| object.key.clone())
            .collect();
        let deletion = bucket.delete_objects(&keys).await;
        let after = inventory(
            bucket
                .list_prefix_with_metadata(&prefix)
                .await
                .map_err(|e| {
                    format!("post-delete verification unavailable; do not assume success: {e}")
                })?,
        )?;
        let verification = verify_deletion(&objects, &after, &plan);
        let remaining = keys.iter().filter(|key| after.contains_key(*key)).count();
        println!(
            "Selected deleted: {}; selected remaining: {remaining}",
            keys.len() - remaining
        );
        deletion.map_err(|e| {
            format!("deletion stopped (possibly partial): {e}; verification: {verification:?}")
        })?;
        verification?;
        println!("Verified: every selected object is gone; all unselected objects are unchanged.");
    } else {
        println!("Nothing deleted. Review this plan, then pass --prune {fingerprint} to apply it.");
    }
    tx.rollback()
        .await
        .map_err(|e| format!("release database lock: {e}"))?;
    Ok(if unrecoverable == 0 {
        ExitCode::SUCCESS
    } else {
        ExitCode::FAILURE
    })
}

async fn read_rows(conn: &mut SqliteConnection) -> Result<Vec<Row>, String> {
    let query = format!(
        "SELECT m.hash, m.file_type, COALESCE(m.uploader_id, 0), COALESCE(m.size, 0),
                m.short_name,
                EXISTS (SELECT 1 FROM audiobook_tracks t WHERE t.media_id = m.id),
                EXISTS (SELECT 1 FROM ({MEDIA_REFERENCES}) refs WHERE refs.media_id = m.id)
         FROM media m WHERE m.hash IS NOT NULL AND m.hash != ''"
    );
    let records = sqlx::query_as::<_, (String, String, i64, i64, String, bool, bool)>(&query)
        .fetch_all(conn)
        .await
        .map_err(|e| format!("read media references: {e}"))?;
    Ok(records
        .into_iter()
        .map(
            |(hash, file_type, uploader_id, size, short_name, is_track, in_use)| Row {
                hash,
                file_type,
                uploader_id,
                size: size.max(0) as u64,
                short_name,
                is_track,
                in_use,
            },
        )
        .collect())
}

fn inventory(objects: Vec<ListedObject>) -> Result<Inventory, String> {
    let mut result = BTreeMap::new();
    for object in objects {
        if !object.key.starts_with(&format!("{AUDIO_OBJECT_PREFIX}/")) {
            return Err(format!("object outside audio/ boundary: {}", object.key));
        }
        if let Some(previous) = result.insert(object.key.clone(), object.clone())
            && previous != object
        {
            return Err(format!("inconsistent duplicate listing for {}", object.key));
        }
    }
    Ok(result)
}

fn build_plan(rows: &[Row], objects: &Inventory, now: i64) -> Result<Plan, String> {
    let mut protected_hashes = HashSet::new();
    let mut owners: HashMap<String, &str> = HashMap::new();
    for row in rows {
        if row.in_use
            || row.is_track
            || !row.short_name.starts_with(TRACK_HANDLE_PREFIX)
            || !is_audio(&row.file_type)
        {
            protected_hashes.insert(row.hash.as_str());
        }
        if !is_audio(&row.file_type) {
            continue;
        }
        // Audiobook writers use ordinary SHA-256 keys; special layouts cannot prove ownership here.
        if row.hash.len() != 64 || !row.hash.bytes().all(|byte| byte.is_ascii_hexdigit()) {
            return Err(format!(
                "unrecognized audio hash {:?}; prune is disabled",
                row.hash
            ));
        }
        let key = media_key(&row.hash, &row.file_type, row.uploader_id)
            .and_then(|key| audio_object_key(&key.key, &row.file_type))
            .ok_or_else(|| format!("unmappable audio hash {}; prune is disabled", row.hash))?;
        owners.insert(key, &row.hash);
    }
    let mut groups: BTreeMap<&str, Vec<&ListedObject>> = BTreeMap::new();
    let mut unclaimed = Vec::new();
    for object in objects.values() {
        match object_base(&object.key).and_then(|base| owners.get(base)) {
            Some(hash) => groups.entry(hash).or_default().push(object),
            None => unclaimed.push(object.key.clone()),
        }
    }
    let mut selected = Vec::new();
    for (hash, group) in groups {
        if protected_hashes.contains(hash)
            || group.iter().any(|object| {
                object.etag.as_deref().is_none_or(str::is_empty)
                    || object
                        .modified_at
                        .is_none_or(|time| time > now - MIN_AGE_SECONDS)
            })
        {
            continue;
        }
        selected.extend(group.into_iter().cloned());
    }
    selected.sort_by(|a, b| a.key.cmp(&b.key));
    Ok(Plan {
        objects: selected,
        unclaimed,
    })
}

fn object_base(key: &str) -> Option<&str> {
    key.strip_prefix(&format!("{AUDIO_OBJECT_PREFIX}/"))?;
    if let Some((base, chunk)) = key.split_once(".d/") {
        let index = chunk.strip_suffix(".zst")?;
        return (index.len() >= 5 && index.bytes().all(|byte| byte.is_ascii_digit()))
            .then_some(base);
    }
    Some(key.strip_suffix(".zm").unwrap_or(key))
}

fn fingerprint(endpoint: &str, bucket: &str, plan: &Plan) -> Result<String, String> {
    let bytes = serde_json::to_vec(&("r2-audio-prune-v1", endpoint, bucket, &plan.objects))
        .map_err(|e| format!("serialize plan: {e}"))?;
    Ok(hex::encode(Sha256::digest(bytes)))
}

fn confirm_plan(expected: &str, actual: &str) -> Result<(), String> {
    if expected != actual {
        return Err("prune fingerprint changed; nothing deleted. Review a new dry run".into());
    }
    Ok(())
}

fn verify_deletion(before: &Inventory, after: &Inventory, plan: &Plan) -> Result<(), String> {
    let selected: HashSet<_> = plan
        .objects
        .iter()
        .map(|object| object.key.as_str())
        .collect();
    let remaining = selected
        .iter()
        .filter(|key| after.contains_key(**key))
        .count();
    let changed = before
        .iter()
        .filter(|(key, object)| {
            !selected.contains(key.as_str()) && after.get(*key) != Some(*object)
        })
        .count();
    let added = after
        .keys()
        .filter(|key| !before.contains_key(*key))
        .count();
    if remaining > 0 || changed > 0 || added > 0 {
        return Err(format!(
            "verification failed: {remaining} selected remain, {changed} unselected missing/changed, {added} unexpected additions"
        ));
    }
    Ok(())
}

async fn reconcile(
    rows: &[Row],
    objects: &Inventory,
    bucket: &R2Client,
    media_dir: &Path,
    upload: bool,
) -> Result<usize, String> {
    let mut checked = HashSet::new();
    let mut failures = 0;
    for row in rows.iter().filter(|row| row.is_track) {
        let key = media_key(&row.hash, &row.file_type, row.uploader_id);
        let Some((key, object)) = key
            .and_then(|key| audio_object_key(&key.key, &row.file_type).map(|object| (key, object)))
        else {
            eprintln!("Invalid track medium: {} ({})", row.hash, row.file_type);
            failures += 1;
            continue;
        };
        if !checked.insert(object.clone()) {
            continue;
        }
        let path = media_dir.join(key.key);
        let on_disk = tokio::fs::metadata(&path).await.ok().map(|meta| meta.len());
        let expected = on_disk.or((row.size > 0).then_some(row.size));
        let present = objects.get(&object).map(|object| object.size);
        if present.is_some() && expected.is_none_or(|size| present == Some(size)) {
            continue;
        }
        if upload && on_disk.is_some() {
            bucket
                .put_file_with_metadata(&object, &path, &row.file_type, MEDIA_CACHE_CONTROL)
                .await
                .map_err(|e| format!("upload {object}: {e}"))?;
            println!("Uploaded: {object}");
        } else {
            eprintln!("Missing/wrong-size live object: {object}");
            failures += 1;
        }
    }
    println!(
        "Live originals checked: {}; missing/wrong-size: {failures}",
        checked.len()
    );
    Ok(failures)
}

fn parse_args(argv: impl IntoIterator<Item = String>) -> Result<Args, String> {
    let mut args = Args::default();
    let mut argv = argv.into_iter();
    while let Some(arg) = argv.next() {
        match arg.as_str() {
            "--upload" => args.upload = true,
            "--prune" => {
                if args.prune.is_some() {
                    return Err("--prune can only be passed once".into());
                }
                let fingerprint = argv
                    .next()
                    .ok_or("--prune requires the dry-run SHA-256 fingerprint")?;
                if fingerprint.len() != 64
                    || !fingerprint.bytes().all(|byte| byte.is_ascii_hexdigit())
                {
                    return Err("--prune requires the dry-run SHA-256 fingerprint".into());
                }
                args.prune = Some(fingerprint.to_ascii_lowercase());
            }
            "--env" => {
                args.env_file = Some(PathBuf::from(argv.next().ok_or("--env needs a path")?))
            }
            "--media-dir" => {
                args.media_dir = Some(PathBuf::from(
                    argv.next().ok_or("--media-dir needs a path")?,
                ))
            }
            "--help" | "-h" => {
                println!("{HELP}");
                std::process::exit(0);
            }
            other => return Err(format!("unknown argument '{other}'\n\n{HELP}")),
        }
    }
    if args.upload && args.prune.is_some() {
        return Err("--upload and --prune cannot be combined".into());
    }
    Ok(args)
}

const HELP: &str = "\
Backfill and reconcile audiobook audio in R2.

Usage: backfill-audio [options]
  (no flags)           read-only reconciliation and exact deletion plan
  --upload             copy missing/wrong-size live originals from disk
  --prune <sha256>     apply the exact reviewed dry-run fingerprint
  --env <path>         read DATABASE_URL / MEDIA_PATH / R2_* from this file
  --media-dir <path>   override MEDIA_PATH

Prune is R2-only: database rows and local files are retained. Only audio/
objects owned by unreferenced abt- media are eligible. Every media reference
and every row sharing a hash is checked. Unknown ownership/layouts, objects
without metadata, and sets modified within 24 hours are retained.

Apply rechecks ownership and the fingerprint under a database write lock,
held through deletion and verification. Uploads can wait while pruning.
External bucket writers must not run concurrently. --upload and --prune
cannot be combined. R2_* selects the bucket independently of AUDIO_BACKEND.
Exits non-zero for missing/wrong-size live originals or failed verification.";

#[cfg(test)]
mod tests {
    use super::*;

    fn row(seed: char) -> Row {
        Row {
            hash: seed.to_string().repeat(64),
            file_type: "audio/mpeg".into(),
            uploader_id: 1,
            size: 64,
            short_name: "abt-1".into(),
            is_track: false,
            in_use: false,
        }
    }

    fn objects(row: &Row) -> Vec<ListedObject> {
        let base = audio_object_key(
            &media_key(&row.hash, &row.file_type, 1).unwrap().key,
            &row.file_type,
        )
        .unwrap();
        [
            base.clone(),
            format!("{base}.zm"),
            format!("{base}.d/00000.zst"),
        ]
        .into_iter()
        .map(|key| ListedObject {
            key,
            size: 64,
            etag: Some("etag".into()),
            modified_at: Some(0),
        })
        .collect()
    }

    #[test]
    fn all_references_and_shared_hashes_are_protected() {
        for variant in 0..5 {
            let candidate = row('a');
            let mut protector = candidate.clone();
            match variant {
                0 => protector.in_use = true,
                1 => protector.is_track = true,
                2 => protector.short_name = "library-audio".into(),
                3 => protector.file_type = "image/png".into(),
                _ => {
                    protector.in_use = true;
                    protector.file_type = "audio/ogg".into();
                }
            }
            let listing = inventory(objects(&candidate)).unwrap();
            let plan = build_plan(&[candidate, protector], &listing, MIN_AGE_SECONDS).unwrap();
            assert!(plan.objects.is_empty(), "variant {variant}");
        }
    }

    #[test]
    fn only_known_exact_objects_are_selected_once() {
        let candidate = row('a');
        let mut listed = objects(&candidate);
        let base = listed[0].key.clone();
        listed.push(listed[0].clone());
        for key in [
            format!("{base}.d-backup/00000.zst"),
            format!("{base}.d/00000.zst/extra"),
            format!("{base}.d/../00000.zst"),
            format!("{base}.d/0.zst"),
            "audio/unowned.mp3".into(),
        ] {
            listed.push(ListedObject {
                key,
                ..listed[0].clone()
            });
        }
        let plan = build_plan(
            &[candidate.clone(), candidate],
            &inventory(listed).unwrap(),
            MIN_AGE_SECONDS,
        )
        .unwrap();
        assert_eq!(plan.objects.len(), 3);
        assert_eq!(plan.unclaimed.len(), 5);
    }

    #[test]
    fn prefix_boundary_and_conflicting_listings_fail_closed() {
        let mut listed = objects(&row('a'));
        listed[0].key = listed[0].key.replacen("audio/", "audio-backup/", 1);
        assert!(inventory(listed).is_err());
        let mut listed = objects(&row('a'));
        let mut conflict = listed[0].clone();
        conflict.etag = Some("changed".into());
        listed.push(conflict);
        assert!(inventory(listed).is_err());
    }

    #[test]
    fn fresh_or_incomplete_metadata_protects_the_whole_hash() {
        for variant in 0..4 {
            let candidate = row('a');
            let mut listed = objects(&candidate);
            match variant {
                0 => listed[1].modified_at = Some(1),
                1 => listed[1].modified_at = None,
                2 => listed[1].etag = None,
                _ => listed[1].etag = Some(String::new()),
            }
            assert!(
                build_plan(&[candidate], &inventory(listed).unwrap(), MIN_AGE_SECONDS)
                    .unwrap()
                    .objects
                    .is_empty()
            );
        }
    }

    #[test]
    fn unrecognized_audio_keys_disable_pruning() {
        let mut invalid = row('a');
        invalid.hash = "../../audio".into();
        assert!(build_plan(&[invalid], &BTreeMap::new(), MIN_AGE_SECONDS).is_err());
    }

    #[test]
    fn fingerprint_is_stable_but_binds_exact_objects_and_target() {
        let rows = [row('a'), row('b')];
        let mut listed: Vec<_> = rows.iter().flat_map(objects).collect();
        let plan = build_plan(&rows, &inventory(listed.clone()).unwrap(), MIN_AGE_SECONDS).unwrap();
        let original = fingerprint("endpoint", "bucket", &plan).unwrap();
        listed.reverse();
        let reordered =
            build_plan(&rows, &inventory(listed.clone()).unwrap(), MIN_AGE_SECONDS).unwrap();
        assert_eq!(
            original,
            fingerprint("endpoint", "bucket", &reordered).unwrap()
        );
        assert_ne!(original, fingerprint("elsewhere", "bucket", &plan).unwrap());
        assert_ne!(
            original,
            fingerprint("endpoint", "other-bucket", &plan).unwrap()
        );
        for variant in 0..4 {
            let mut changed = listed.clone();
            match variant {
                0 => changed[0].etag = Some("different".into()),
                1 => changed[0].size += 1,
                2 => changed[0].modified_at = Some(-1),
                _ => {
                    changed.pop();
                }
            }
            let changed = build_plan(&rows, &inventory(changed).unwrap(), MIN_AGE_SECONDS).unwrap();
            assert!(
                confirm_plan(
                    &original,
                    &fingerprint("endpoint", "bucket", &changed).unwrap()
                )
                .is_err()
            );
        }
        let other_key = build_plan(
            &[row('c')],
            &inventory(objects(&row('c'))).unwrap(),
            MIN_AGE_SECONDS,
        )
        .unwrap();
        assert_ne!(
            original,
            fingerprint("endpoint", "bucket", &other_key).unwrap()
        );
    }

    #[test]
    fn new_reference_invalidates_approved_plan() {
        let mut candidate = row('a');
        let listing = inventory(objects(&candidate)).unwrap();
        let original = build_plan(&[candidate.clone()], &listing, MIN_AGE_SECONDS).unwrap();
        candidate.in_use = true;
        let current = build_plan(&[candidate], &listing, MIN_AGE_SECONDS).unwrap();
        assert!(
            confirm_plan(
                &fingerprint("e", "b", &original).unwrap(),
                &fingerprint("e", "b", &current).unwrap()
            )
            .is_err()
        );
    }

    #[test]
    fn verification_checks_selected_and_every_unselected_object() {
        let candidate = row('a');
        let mut listed = objects(&candidate);
        let kept = objects(&row('b'));
        listed.extend(kept.clone());
        let before = inventory(listed).unwrap();
        let plan = build_plan(&[candidate], &before, MIN_AGE_SECONDS).unwrap();
        let after = inventory(kept).unwrap();
        assert!(verify_deletion(&before, &after, &plan).is_ok());
        assert!(verify_deletion(&before, &before, &plan).is_err());
        let mut damaged = after.clone();
        damaged.pop_first();
        assert!(verify_deletion(&before, &damaged, &plan).is_err());
        let mut damaged = after.clone();
        damaged.values_mut().next().unwrap().etag = Some("changed".into());
        assert!(verify_deletion(&before, &damaged, &plan).is_err());
        let mut added = after;
        let extra = objects(&row('c')).remove(0);
        added.insert(extra.key.clone(), extra);
        assert!(verify_deletion(&before, &added, &plan).is_err());
    }

    #[tokio::test]
    async fn real_database_alias_protects_every_row_sharing_its_hash() {
        use sqlx::Connection;

        let mut conn = SqliteConnection::connect("sqlite::memory:").await.unwrap();
        sqlx::migrate::Migrator::new(PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("migrations"))
            .await
            .unwrap()
            .run(&mut conn)
            .await
            .unwrap();
        sqlx::query("INSERT INTO users (id, username, email, password_hash) VALUES (1, 'author', 'a@example.com', 'h')")
            .execute(&mut conn).await.unwrap();
        for (id, hash) in [(1, 'a'), (2, 'a'), (3, 'b')] {
            sqlx::query("INSERT INTO media (id, hash, short_name, file_name, file_type, url, size, uploader_id) VALUES (?, ?, ?, 'audio.mp3', 'audio/mpeg', 'local', 64, 1)")
                .bind(id).bind(hash.to_string().repeat(64)).bind(format!("abt-{id}"))
                .execute(&mut conn).await.unwrap();
        }
        sqlx::query("INSERT INTO media_aliases (media_id, alias) VALUES (1, 'keep')")
            .execute(&mut conn)
            .await
            .unwrap();
        let rows = read_rows(&mut conn).await.unwrap();
        assert_eq!(rows.len(), 3);
        assert_eq!(rows.iter().filter(|row| row.in_use).count(), 1);
        let listing = inventory(rows.iter().flat_map(objects).collect()).unwrap();
        let plan = build_plan(&rows, &listing, MIN_AGE_SECONDS).unwrap();
        assert_eq!(plan.objects.len(), 3);
        assert!(
            plan.objects
                .iter()
                .all(|object| object.key.contains(&"b".repeat(64)))
        );
        let count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM media")
            .fetch_one(&mut conn)
            .await
            .unwrap();
        assert_eq!(count, 3);
    }

    #[test]
    fn destructive_cli_requires_fingerprint_and_excludes_upload() {
        let parse = |args: &[&str]| parse_args(args.iter().map(|arg| (*arg).to_string()));
        assert!(parse(&[]).unwrap().prune.is_none());
        assert!(parse(&["--prune"]).is_err());
        assert!(parse(&["--prune", "yes"]).is_err());
        assert!(parse(&["--prune", "--upload"]).is_err());
        let hash = "a".repeat(64);
        assert!(parse(&["--prune", &hash]).is_ok());
        assert!(parse(&["--upload", "--prune", &hash]).is_err());
        assert!(parse(&["--prune", &hash, "--prune", &hash]).is_err());
    }
}

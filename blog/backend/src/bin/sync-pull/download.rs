
pub async fn sync_media(
    client: &reqwest::Client,
    api: &Api<'_>,
    key: &str,
    media_dir: &Path,
    entries: &[MediaEntry],
    prune: bool,
    c: &mut Counters,
) -> Result<(), String> {
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
                        println!("  c.missing on source: {}", entry.path);
                    }
                    Err(e) => return Err(e),
                }
                let done = c.downloaded + c.skipped;
                if done % 200 == 0 && done < count {
                    println!("  {done}/{count} …");
                }
            }
            println!("  {c.downloaded} c.downloaded, {c.skipped} already up to date");
            if args.prune {
                let (removed, freed) = prune_extras(&media_dir, &expected, false).await;
                println!(
                    "  pruned {removed} extra file(s), freed {}",
                    format_bytes(freed)
                );
            }
}

pub async fn sync_demo_dirs(
    client: &reqwest::Client,
    api: &Api<'_>,
    key: &str,
    demos_dir: &Path,
    project_demos: &[DemoDir],
    game_demos: &[DemoDir],
    prune: bool,
    c: &mut Counters,
) -> Result<(), String> {
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
                    match download_to_file(&client, &api(&section), &key, &target, Some(file.size), true, false)
                        .await
                    {
                        Ok(Fetched::Downloaded) => {
                            c.downloaded += 1;
                            c.transferred += file.size;
                        }
                        Ok(Fetched::Current) => c.skipped += 1,
                        Ok(Fetched::Missing) => {
                            c.missing += 1;
                            println!("  c.missing on source: {section}");
                        }
                        Err(e) => return Err(e),
                    }
                }
                expected_per_dir.push((base, local));
            }
        }
        println!("  {c.downloaded} c.downloaded (cumulative), {c.skipped} already up to date");
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

pub async fn sync_artifacts(
    client: &reqwest::Client,
    api: &Api<'_>,
    key: &str,
    demos_dir: &Path,
    entries: &[ArtifactEntry],
    prune: bool,
    c: &mut Counters,
) -> Result<(), String> {
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
                        println!("  c.missing on source: {}", entry.key);
                    }
                    Err(e) => return Err(e),
                }
            }
            println!("  {c.downloaded} c.downloaded (cumulative), {c.skipped} already up to date");
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

// Local environment plumbing: .env reading/updating, sync-key resolution,
// and path derivation from the database URL.
use std::path::{Path, PathBuf};

/// `(DATABASE_URL, MEDIA_PATH, PROJECT_DEMOS_PATH)` as read from a .env file.
pub type EnvValues = (Option<String>, Option<String>, Option<String>);

pub fn read_env_file(path: &Path) -> Result<EnvValues, String> {
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

/// Rewrites KEY=VALUE lines for the given pairs in place, preserving comments,
/// ordering and all other keys; appends missing keys under a local section.
pub fn update_env_file(path: &Path, updates: &[(&str, String)]) -> Result<(), String> {
    let content = std::fs::read_to_string(path).unwrap_or_default();
    let mut pending: Vec<(String, String)> = updates
        .iter()
        .map(|(k, v)| (k.to_string(), v.clone()))
        .collect();
    let mut out = String::with_capacity(content.len() + 128);
    for line in content.lines() {
        let trimmed = line.trim_start();
        if let Some((key, _)) = trimmed.split_once('=') {
            let key = key.trim();
            if let Some(pos) = pending.iter().position(|(k, _)| k == key) {
                let (_, value) = pending.swap_remove(pos);
                out.push_str(&format!("{key}={value}\n"));
                continue;
            }
        }
        out.push_str(line);
        out.push('\n');
    }
    if !out.ends_with('\n') && !out.is_empty() {
        out.push('\n');
    }
    if !pending.is_empty() {
        out.push_str("\n# ── local sync-pull values ──\n");
        for (key, value) in &pending {
            out.push_str(&format!("{key}={value}\n"));
        }
    }
    std::fs::write(path, out).map_err(|e| format!("write {}: {e}", path.display()))
}

pub fn resolve_key(key: &str) -> Result<String, String> {
    if let Some(path) = key.strip_prefix('@') {
        let content = std::fs::read_to_string(path)
            .map_err(|e| format!("cannot read key file {path}: {e}"))?;
        Ok(content.trim().to_string())
    } else {
        Ok(key.to_string())
    }
}

/// Mirrors the backend's DATABASE_URL parsing: `sqlite:<path>`.
pub fn database_path_from_url(url: &str) -> Result<PathBuf, String> {
    url.strip_prefix("sqlite:")
        .map(PathBuf::from)
        .ok_or_else(|| format!("unsupported DATABASE_URL '{url}' (only sqlite: is supported)"))
}

pub fn format_bytes(size: u64) -> String {
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

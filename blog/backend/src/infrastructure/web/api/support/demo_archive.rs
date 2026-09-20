// Demo-zip intake shared by the game and project handlers: path safety for
// archive entries and the blocking-pool extraction into the demo directory.
// The caller picks the target directory name (`game-{id}`, `{id}`) and the
// error constructors, so each aggregate keeps its own error type.
use std::{
    fs,
    io::{Cursor, Read, Write},
    path::{Component, Path, PathBuf},
};

use axum::body::Bytes;
use uuid::Uuid;
use zip::ZipArchive;

use crate::infrastructure::web::server::ProjectDemoConfig;

pub fn has_invalid_component(path: &Path) -> bool {
    path.components().any(|component| {
        matches!(
            component,
            Component::ParentDir | Component::RootDir | Component::Prefix(_)
        )
    })
}

pub fn normalized_zip_path(path: &Path) -> Option<PathBuf> {
    if has_invalid_component(path) {
        return None;
    }
    let mut out = PathBuf::new();
    for component in path.components() {
        if let Component::Normal(part) = component {
            out.push(part);
        }
    }
    if out.as_os_str().is_empty() {
        None
    } else {
        Some(out)
    }
}

pub fn strip_common_root(paths: &[PathBuf]) -> Option<String> {
    let mut first_root: Option<String> = None;
    for path in paths {
        if path.file_name().is_some_and(|name| name == "index.html")
            && path.parent() == Some(Path::new(""))
        {
            return None;
        }

        let mut components = path.components();
        let first = match components.next() {
            Some(Component::Normal(part)) => part.to_string_lossy().to_string(),
            _ => return None,
        };
        if components.next().is_none() {
            return None;
        }
        match &first_root {
            Some(root) if root != &first => return None,
            None => first_root = Some(first),
            _ => {}
        }
    }
    first_root
}

// A 100 MB archive can expand to 200 MB of sync file writes, so the blocking
// pool runs the extraction instead of stalling a tokio worker for seconds.
pub async fn extract_demo_zip<E>(
    config: &ProjectDemoConfig,
    dir_name: String,
    zip_bytes: Bytes,
    internal_error: impl Fn(String) -> E + Send + 'static,
    invalid_demo: impl Fn(String) -> E + Send + 'static,
) -> Result<(), E>
where
    E: From<std::io::Error> + Send + 'static,
{
    let config = config.clone();
    tokio::task::spawn_blocking(move || {
        extract_demo_zip_blocking(config, dir_name, zip_bytes, &invalid_demo)
    })
    .await
    .map_err(|e| internal_error(e.to_string()))?
}

fn extract_demo_zip_blocking<E>(
    config: ProjectDemoConfig,
    dir_name: String,
    zip_bytes: Bytes,
    invalid_demo: &impl Fn(String) -> E,
) -> Result<(), E>
where
    E: From<std::io::Error>,
{
    if zip_bytes.len() as u64 > config.max_archive_size {
        return Err(invalid_demo("Demo archive is too large.".to_string()));
    }

    let mut archive = ZipArchive::new(Cursor::new(zip_bytes))
        .map_err(|e| invalid_demo(e.to_string()))?;
    if archive.is_empty() {
        return Err(invalid_demo("Demo archive is empty.".to_string()));
    }
    if archive.len() > config.max_files {
        return Err(invalid_demo(
            "Demo archive contains too many files.".to_string(),
        ));
    }

    let mut paths = Vec::<PathBuf>::new();
    for i in 0..archive.len() {
        let file = archive
            .by_index(i)
            .map_err(|e| invalid_demo(e.to_string()))?;
        if file.is_dir() {
            continue;
        }
        #[cfg(unix)]
        if file
            .unix_mode()
            .is_some_and(|mode| mode & 0o170000 == 0o120000)
        {
            return Err(invalid_demo(
                "Demo archive cannot contain symlinks.".to_string(),
            ));
        }
        let enclosed = file
            .enclosed_name()
            .ok_or_else(|| invalid_demo("Demo archive contains an unsafe path.".to_string()))?;
        let normalized = normalized_zip_path(&enclosed)
            .ok_or_else(|| invalid_demo("Demo archive contains an unsafe path.".to_string()))?;
        paths.push(normalized);
    }

    if paths.is_empty() {
        return Err(invalid_demo(
            "Demo archive does not contain files.".to_string(),
        ));
    }

    let common_root = strip_common_root(&paths);
    let rel_paths = paths
        .iter()
        .map(|path| {
            common_root
                .as_ref()
                .and_then(|root| path.strip_prefix(root).ok())
                .map(PathBuf::from)
                .unwrap_or_else(|| path.clone())
        })
        .collect::<Vec<_>>();

    if !rel_paths.iter().any(|path| path == Path::new("index.html")) {
        return Err(invalid_demo(
            "Demo archive must contain index.html.".to_string(),
        ));
    }

    let root = &config.dir;
    fs::create_dir_all(root)?;
    let tmp_dir = root.join(format!(".tmp-{}-{}", dir_name, Uuid::new_v4()));
    fs::create_dir_all(&tmp_dir)?;

    let mut extracted_size = 0_u64;
    for i in 0..archive.len() {
        let mut file = archive
            .by_index(i)
            .map_err(|e| invalid_demo(e.to_string()))?;
        if file.is_dir() {
            continue;
        }
        let enclosed = file
            .enclosed_name()
            .ok_or_else(|| invalid_demo("Demo archive contains an unsafe path.".to_string()))?;
        let original = normalized_zip_path(&enclosed)
            .ok_or_else(|| invalid_demo("Demo archive contains an unsafe path.".to_string()))?;
        let rel = common_root
            .as_ref()
            .and_then(|root| original.strip_prefix(root).ok())
            .map(PathBuf::from)
            .unwrap_or(original);
        if rel.as_os_str().is_empty() || has_invalid_component(&rel) {
            fs::remove_dir_all(&tmp_dir).ok();
            return Err(invalid_demo(
                "Demo archive contains an unsafe path.".to_string(),
            ));
        }

        extracted_size = extracted_size.saturating_add(file.size());
        if extracted_size > config.max_extracted_size {
            fs::remove_dir_all(&tmp_dir).ok();
            return Err(invalid_demo(
                "Demo archive expands too large.".to_string(),
            ));
        }

        let out_path = tmp_dir.join(&rel);
        if let Some(parent) = out_path.parent() {
            fs::create_dir_all(parent)?;
        }
        let mut bytes = Vec::new();
        file.read_to_end(&mut bytes)
            .map_err(|e| invalid_demo(e.to_string()))?;
        let mut out = fs::File::create(out_path)?;
        out.write_all(&bytes)?;
    }

    let final_dir = root.join(&dir_name);
    if final_dir.exists() {
        fs::remove_dir_all(&final_dir)?;
    }
    fs::rename(&tmp_dir, &final_dir)?;

    Ok(())
}

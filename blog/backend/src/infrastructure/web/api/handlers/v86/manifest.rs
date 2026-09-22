// Manifest parsing and machine-shape resolution: the single source of truth
// for what a v86 system or game boots with.
use std::{
    collections::{HashMap, HashSet},
    path::{Component, Path},
};

use sha2::{Digest, Sha256};

use crate::domain::errors::project::ProjectError;

use super::constants::{
    MANIFEST_MAX_BYTES, SAVE_FILE_MAX_COUNT, SAVE_FILE_MAX_LEN, V86_VGA_MEMORY_SIZE,
};
use super::dto::V86SystemSpecs;

/// XP's VESA driver rejects 8 MB of VRAM ("cannot find enough video memory");
/// 16 MB is what it wants for 800x600+. Guest memory itself is per-system
/// (`v86_systems.memory_size_mb`), so only this stays platform-derived.
fn v86_vga_memory_size_for(platform_key: &str) -> u64 {
    match platform_key {
        "windowsxp" => 16 * 1024 * 1024,
        _ => V86_VGA_MEMORY_SIZE,
    }
}

pub(super) fn parse_system_specs(specs: Option<&str>) -> V86SystemSpecs {
    specs
        .and_then(|json| serde_json::from_str(json).ok())
        .unwrap_or_default()
}

/// The single source of truth for the machine shape a system boots with.
/// The descriptor, the sandbox payload, and every snapshot-freshness check
/// resolve through this so they can never disagree with a capture.
pub(super) fn resolve_system_machine(platform_key: &str, specs: &V86SystemSpecs) -> (u64, u64) {
    let vga = match specs.vga_memory_size_mb {
        Some(mb) => mb.max(1) as u64 * 1024 * 1024,
        None => v86_vga_memory_size_for(platform_key),
    };
    (vga, V86_VGA_MEMORY_SIZE)
}

/// A single launch variant resolved from the manifest. `name` is the display
/// label and the "source of truth" for how many variants exist; `exe`/`args`
/// are that variant's coalesced values (falling back to the root keys).
#[derive(Debug, Clone)]
pub(super) struct VariantSpec {
    pub(super) index: i32,
    pub(super) name: String,
    pub(super) exe: String,
    pub(super) args: String,
}

pub(super) fn validate_manifest(manifest: &str) -> Result<String, ProjectError> {
    if manifest.len() > MANIFEST_MAX_BYTES {
        return Err(ProjectError::InvalidDemo(
            "The v86 manifest cannot exceed 64 KiB.".to_string(),
        ));
    }
    if manifest.contains('\0') {
        return Err(ProjectError::InvalidDemo(
            "The v86 manifest cannot contain NUL characters.".to_string(),
        ));
    }
    Ok(hex::encode(Sha256::digest(manifest.as_bytes())))
}

fn normalize_manifest_path(value: &str) -> Result<String, ProjectError> {
    let mut normalized = value.trim().trim_matches('"').replace('\\', "/");
    let upper = normalized.to_ascii_uppercase();
    if upper.starts_with("D:/GAME/") {
        normalized = normalized[8..].to_string();
    } else if upper.starts_with("D:/") {
        normalized = normalized[3..].to_string();
    }
    while normalized.starts_with("./") {
        normalized = normalized[2..].to_string();
    }
    let path = Path::new(&normalized);
    if normalized.is_empty()
        || path.is_absolute()
        || path.components().any(|component| {
            matches!(
                component,
                Component::ParentDir | Component::RootDir | Component::Prefix(_)
            )
        })
    {
        return Err(ProjectError::InvalidDemo(
            "The Windows 9x manifest contains an unsafe executable path.".to_string(),
        ));
    }
    Ok(normalized)
}

/// Parses a manifest into a lower-cased key -> value map. Line comments and
/// bare `[section]` headers are ignored, matching INI-style manifests.
fn parse_manifest_fields(manifest: &str) -> HashMap<String, String> {
    let mut fields = HashMap::new();
    for line in manifest.lines() {
        let line = line.trim();
        if line.is_empty()
            || line.starts_with('#')
            || line.starts_with(';')
            || (line.starts_with('[') && line.ends_with(']'))
        {
            continue;
        }
        if let Some((key, value)) = line.split_once('=') {
            fields.insert(key.trim().to_ascii_lowercase(), value.trim().to_string());
        }
    }
    fields
}

/// Per-project mouse settings resolved from the manifest.
#[derive(Debug, Clone, Copy)]
pub struct MouseConfig {
    pub revert_mouse_y: bool,
    pub mouse_speed: f64,
}

impl Default for MouseConfig {
    fn default() -> Self {
        MouseConfig {
            revert_mouse_y: false,
            mouse_speed: 1.0,
        }
    }
}

/// Resolves `revert_mouse_y` and `mouse_speed` from the manifest. Missing keys
/// fall back to the defaults (no revert, 1.0 speed); present-but-invalid
/// values are rejected so a typo surfaces at upload instead of silently
/// degrading every visitor's mouse.
pub fn parse_mouse_config(manifest: &str) -> Result<MouseConfig, ProjectError> {
    let fields = parse_manifest_fields(manifest);
    let revert_mouse_y = match fields.get("revert_mouse_y").map(String::as_str) {
        None | Some("") => false,
        Some("1") | Some("true") | Some("yes") | Some("on") => true,
        Some("0") | Some("false") | Some("no") | Some("off") => false,
        Some(other) => {
            return Err(ProjectError::InvalidDemo(format!(
                "Invalid revert_mouse_y value '{other}': expected 0 or 1."
            )));
        }
    };
    let mouse_speed = match fields.get("mouse_speed").map(String::as_str) {
        None | Some("") => 1.0,
        Some(raw) => {
            let value: f64 = raw.trim().parse().map_err(|_| {
                ProjectError::InvalidDemo(format!(
                    "Invalid mouse_speed value '{raw}': expected a number."
                ))
            })?;
            if !value.is_finite() || value <= 0.0 {
                return Err(ProjectError::InvalidDemo(
                    "mouse_speed must be a positive number.".to_string(),
                ));
            }
            value
        }
    };
    Ok(MouseConfig {
        revert_mouse_y,
        mouse_speed,
    })
}

/// Lowest suffix index (`name1` -> 1, `exe3` -> 3) for a `{base}{digits}` key,
/// or `None` when the key carries no numeric suffix.
fn key_index(base: &str, key: &str) -> Option<i32> {
    let rest = key.strip_prefix(base)?;
    if rest.is_empty() || !rest.chars().all(|c| c.is_ascii_digit()) {
        return None;
    }
    rest.parse::<i32>().ok()
}

/// Resolves the manifest's launch variants. Names are the source of truth:
/// the variant count comes from the highest named index, names must be
/// contiguous from 1..K, and every variant must resolve an executable.
/// Projects with no `name` keys inherit a single (unnamed) variant so existing
/// manifests keep working unchanged.
pub(super) fn parse_variants(manifest: &str) -> Result<Vec<VariantSpec>, ProjectError> {
    let fields = parse_manifest_fields(manifest);

    // Names define the variant set.
    let mut name_indices = HashSet::new();
    for key in fields.keys() {
        if key == "name" || key == "name1" {
            name_indices.insert(1);
        } else if let Some(index) = key_index("name", key) {
            name_indices.insert(index.max(1));
        }
    }
    let max_name = name_indices.iter().copied().max();

    // Highest index referenced by ANY variant-scoped key (name/exe/args).
    let mut explicit_max = 0;
    for key in fields.keys() {
        for base in ["name", "exe", "args"] {
            if let Some(i) = key_index(base, key) {
                explicit_max = explicit_max.max(i);
            }
        }
    }

    let k: i32 = match max_name {
        Some(m) => m,
        None => {
            // No named variants.
            if explicit_max > 1 {
                return Err(ProjectError::InvalidDemo(
                    "Variant keys (nameN/exeN/argsN) require a name for variant 1.".to_string(),
                ));
            }
            1 // a single, unnamed (legacy) variant
        }
    };

    // Names must be contiguous 1..=K.
    for i in 1..=k {
        let named = if i == 1 {
            fields.contains_key("name") || fields.contains_key("name1")
        } else {
            fields.contains_key(&format!("name{i}"))
        };
        if !named {
            return Err(ProjectError::InvalidDemo(format!(
                "The v86 manifest must name each variant contiguously (missing name for variant {i})."
            )));
        }
    }
    // No key may reference an index beyond the named set.
    if explicit_max > k {
        return Err(ProjectError::InvalidDemo(format!(
            "Variant keys reference index {explicit_max} but only {k} named variants exist."
        )));
    }

    let mut variants = Vec::new();
    for i in 1..=k {
        let name = resolve_for(&fields, "name", i, false).unwrap_or_default();
        let exe = resolve_for(&fields, "exe", i, true)
            .ok_or_else(|| {
                ProjectError::InvalidDemo(format!(
                    "Variant {i} requires an executable (exe{i} or exe)."
                ))
            })?
            .trim()
            .to_string();
        if exe.is_empty() {
            return Err(ProjectError::InvalidDemo(format!(
                "Variant {i} requires an executable (exe{i} or exe)."
            )));
        }
        let exe = normalize_manifest_path(&exe)?;
        if !exe.to_ascii_lowercase().ends_with(".exe") {
            return Err(ProjectError::InvalidDemo(format!(
                "The Windows 9x manifest executable for variant {i} must be an .exe file."
            )));
        }
        let args = resolve_for(&fields, "args", i, true).unwrap_or_default();
        variants.push(VariantSpec {
            index: i,
            name,
            exe,
            args,
        });
    }
    Ok(variants)
}

fn resolve_for(
    fields: &HashMap<String, String>,
    base: &str,
    index: i32,
    fallback_root: bool,
) -> Option<String> {
    let base = base.to_string();
    if index > 1 {
        return fields
            .get(&format!("{base}{index}"))
            .or_else(|| fallback_root.then(|| fields.get(&base)).flatten())
            .cloned();
    }
    fields
        .get(&base)
        .or_else(|| fields.get(&format!("{base}1")))
        .cloned()
}

/// Validates a single save entry from the manifest: a relative path under the
/// D:\ game drive with `/` or `\` separators, e.g. `Save0001.dat` or
/// `A\save0001.dat`. No absolute paths, no `.`/`..` components, and none of the
/// INI-dividing chars.
fn validate_save_file(entry: &str) -> Result<(), &'static str> {
    if entry.is_empty() {
        return Err("empty");
    }
    if entry.len() > SAVE_FILE_MAX_LEN {
        return Err("too long");
    }
    for component in entry.split(['/', '\\']) {
        if component.is_empty() {
            return Err("empty path component");
        }
        if component == ".." || component == "." {
            return Err("unsafe path component");
        }
        for byte in component.bytes() {
            if !(b'!'..=b'~').contains(&byte) {
                return Err("unsupported character");
            }
            if matches!(
                byte,
                b',' | b';' | b'=' | b':' | b'"' | b'<' | b'>' | b'|' | b'?' | b'*'
            ) {
                return Err("unsupported character");
            }
        }
    }
    Ok(())
}

/// Resolves the manifest's `save_paths`/`save_path`/`saves` into the exact save
/// entries to collect (e.g. `Save0001.dat; A/save0001.dat; backup/what.bak`).
/// An entry with no separator matches that *basename* anywhere under the D:\
/// game drive root; an entry with a folder matches that exact relative path.
/// The in-guest launcher walks the whole game tree and collects whatever
/// matches, so the save layout never has to be known in advance.
pub(super) fn save_files_from_manifest(manifest: &str) -> Result<Vec<String>, ProjectError> {
    let fields = parse_manifest_fields(manifest);
    let raw = fields
        .get("save_paths")
        .or_else(|| fields.get("save_path"))
        .or_else(|| fields.get("saves"))
        .map(String::as_str)
        .unwrap_or("");
    let mut files = Vec::new();
    let mut seen = HashSet::new();
    for entry in raw.split([',', ';']).map(str::trim) {
        let entry = entry.trim_matches('"');
        if entry.is_empty() {
            continue;
        }
        validate_save_file(entry).map_err(|reason| {
            ProjectError::InvalidDemo(format!(
                "Invalid Windows 9x save entry '{entry}': {reason}."
            ))
        })?;
        let normalized = entry.replace('/', "\\");
        if seen.insert(normalized.to_ascii_lowercase()) {
            files.push(normalized);
            if files.len() >= SAVE_FILE_MAX_COUNT {
                break;
            }
        }
    }
    Ok(files)
}

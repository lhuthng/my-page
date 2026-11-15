//! Environment knobs for the author API.
//!
//! There is deliberately **no deployment-wide on/off switch** here: exposure is
//! a per-audiobook flag (`audiobooks.author_api_enabled`, default 0) that the
//! book's creator controls from the dashboard. This file only holds limits.

/// Ceiling on one batch-download archive, in bytes. Defaults to 2 GiB so a
/// 900-chapter book cannot fill the VM's disk with a temp zip.
pub fn max_archive_bytes() -> u64 {
    std::env::var("AUTHOR_API_MAX_ARCHIVE_BYTES")
        .ok()
        .and_then(|value| value.parse::<u64>().ok())
        .unwrap_or(2 * 1024 * 1024 * 1024)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_missing_or_bad_value_still_yields_a_usable_cap() {
        // The point is that neither an unset variable nor junk can produce 0,
        // which would reject every archive.
        assert!(max_archive_bytes() > 0);
    }
}

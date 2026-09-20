use super::{normalize_optional_utc_timestamp, normalize_utc_timestamp};

#[test]
fn treats_sqlite_current_timestamp_as_utc() {
    assert_eq!(
        normalize_utc_timestamp("2026-07-09 15:30:45"),
        "2026-07-09T15:30:45Z"
    );
}

#[test]
fn preserves_existing_instant_when_offset_is_present() {
    assert_eq!(
        normalize_utc_timestamp("2026-07-09T17:30:45+02:00"),
        "2026-07-09T15:30:45Z"
    );
}

#[test]
fn normalizes_optional_timestamps() {
    assert_eq!(
        normalize_optional_utc_timestamp(Some("2026-07-09 15:30:45".to_string())),
        Some("2026-07-09T15:30:45Z".to_string())
    );
    assert_eq!(normalize_optional_utc_timestamp(None), None);
}

use chrono::{DateTime, NaiveDateTime, SecondsFormat, Utc};

const SQLITE_TIMESTAMP_FORMATS: [&str; 4] = [
    "%Y-%m-%d %H:%M:%S",
    "%Y-%m-%d %H:%M:%S%.f",
    "%Y-%m-%dT%H:%M:%S",
    "%Y-%m-%dT%H:%M:%S%.f",
];

pub fn normalize_utc_timestamp(value: impl AsRef<str>) -> String {
    let value = value.as_ref();

    if let Ok(datetime) = DateTime::parse_from_rfc3339(value) {
        return datetime
            .with_timezone(&Utc)
            .to_rfc3339_opts(SecondsFormat::Secs, true);
    }

    for format in SQLITE_TIMESTAMP_FORMATS {
        if let Ok(datetime) = NaiveDateTime::parse_from_str(value, format) {
            return DateTime::<Utc>::from_naive_utc_and_offset(datetime, Utc)
                .to_rfc3339_opts(SecondsFormat::Secs, true);
        }
    }

    value.to_string()
}

pub fn normalize_optional_utc_timestamp(value: Option<String>) -> Option<String> {
    value.map(normalize_utc_timestamp)
}

#[cfg(test)]
mod tests {
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
}

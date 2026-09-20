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
mod tests;

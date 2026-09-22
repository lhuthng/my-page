// Tuple row shapes for the audiobook aggregate's queries.
pub(super) type SnapshotRow = (
    i64,
    String,
    String,
    String,
    Option<String>,
    String,
    Option<String>,
    i64,
    i64,
    Option<String>,
    Option<String>,
    String,
    Option<String>,
);

pub(super) type DetailsRow = (
    i64,
    String,
    String,
    String,
    Option<String>,
    String,
    Option<String>,
    i64,
    String,
    String,
    String,
    Option<String>,
);

/// A track joined with the media row it plays from.
pub(super) type TrackRow = (
    i64,
    String,
    i64,
    Option<i64>,
    Option<String>,
    Option<String>,
);

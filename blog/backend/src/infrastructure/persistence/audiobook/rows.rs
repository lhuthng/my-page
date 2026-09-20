// Tuple row shapes for the audiobook aggregate's queries.
type SnapshotRow = (
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

type DetailsRow = (
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
type TrackRow = (
    i64,
    String,
    i64,
    Option<i64>,
    Option<String>,
    Option<String>,
);

/// A cover upload already written to disk, ready to be registered as a media
/// row. Built before the transaction opens so no large write happens while the
/// database write lock is held.

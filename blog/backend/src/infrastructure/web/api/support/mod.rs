// Shared handler utilities, one concern per file. These modules are generic
// over the aggregate: they may not import any `handlers::<feature>` module,
// otherwise they become a back-channel between features.
pub mod cover;

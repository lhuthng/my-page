// clippy 1.99's double_must_use fires on functions the async_trait
// macro generates: they carry #[must_use] although their return type is
// already #[must_use]. The expansion is the dependency's code, not this
// crate's, so the lint is allowed here and -D warnings stays strict for
// everything the project actually writes. unknown_lints rides along so
// older clippy versions, which do not know the lint yet, stay silent.
#![allow(unknown_lints, clippy::double_must_use)]

pub mod application;
pub mod domain;
pub mod helper;
pub mod infrastructure;

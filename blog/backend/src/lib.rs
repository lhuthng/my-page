// async_trait's generated functions carry #[must_use] although their
// return type already is #[must_use] — clippy 1.99's double_must_use
// flags that. The expansion is the dependency's, so allow it here;
// unknown_lints keeps older clippy quiet about the allow itself.
#![allow(unknown_lints, clippy::double_must_use)]

pub mod application;
pub mod domain;
pub mod helper;
pub mod infrastructure;

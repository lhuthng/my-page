// GraphQL query resolvers, one file per domain. async-graphql merges the
// multiple `#[Object] impl QueryRoot` blocks into a single schema type.
pub mod comments;
pub mod dashboard;
pub mod detail;
pub mod featured;
pub mod media;
pub mod posts;
pub mod series;
pub mod slug;
pub mod stats;
pub mod taxonomy;
pub mod users;

pub use super::schema::QueryRoot;

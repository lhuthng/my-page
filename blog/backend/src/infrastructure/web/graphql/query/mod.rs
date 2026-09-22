// GraphQL query resolvers, one struct per domain. `QueryRoot` (in
// `super::schema`) merges them into a single schema type via
// `MergedObject`.
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

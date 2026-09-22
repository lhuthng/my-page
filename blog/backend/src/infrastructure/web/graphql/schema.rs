// The GraphQL schema type and its assembly. `QueryRoot` merges the
// per-domain query structs in `query/` via `MergedObject`.
use async_graphql::{EmptyMutation, EmptySubscription, MergedObject, Schema};
use sqlx::SqlitePool;

use super::query::{
    comments::CommentsQuery, dashboard::DashboardQuery, detail::DetailQuery,
    featured::FeaturedQuery, media::MediaQuery, posts::PostsQuery, series::SeriesQuery,
    slug::SlugQuery, stats::StatsQuery, taxonomy::TaxonomyQuery, users::UsersQuery,
};

#[derive(MergedObject, Default)]
pub struct QueryRoot(
    CommentsQuery,
    DashboardQuery,
    DetailQuery,
    FeaturedQuery,
    MediaQuery,
    PostsQuery,
    SeriesQuery,
    SlugQuery,
    StatsQuery,
    TaxonomyQuery,
    UsersQuery,
);

pub type BlogSchema = Schema<QueryRoot, EmptyMutation, EmptySubscription>;

pub fn build_schema(pool: SqlitePool) -> BlogSchema {
    Schema::build(QueryRoot::default(), EmptyMutation, EmptySubscription)
        .data(pool)
        .finish()
}

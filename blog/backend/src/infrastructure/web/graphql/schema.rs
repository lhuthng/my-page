// The GraphQL schema type and its assembly. `QueryRoot`'s resolver fields
// live in `query/`.
use async_graphql::{EmptyMutation, EmptySubscription, Schema};
use sqlx::SqlitePool;

pub struct QueryRoot;

pub type BlogSchema = Schema<QueryRoot, EmptyMutation, EmptySubscription>;

pub fn build_schema(pool: SqlitePool) -> BlogSchema {
    Schema::build(QueryRoot, EmptyMutation, EmptySubscription)
        .data(pool)
        .finish()
}

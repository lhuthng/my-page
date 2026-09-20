// Thin router composer: each feature owns its route table and auth layers
// in `handlers/<feature>`, this file only nests them and applies the global
// layers. See `layers.rs` for the CORS/compression/trace policy.
use std::sync::Arc;

use axum::{
    extract::{DefaultBodyLimit, Extension},
    middleware,
    routing::get,
    routing::get_service,
    Router,
};
use async_graphql_axum::{GraphQLRequest, GraphQLResponse};
use tower_http::services::ServeDir;

use crate::domain::entities::secret::Claims;
use crate::infrastructure::web::api::{handlers, layers, middlewares};
use crate::infrastructure::web::graphql::BlogSchema;
use crate::infrastructure::web::server::AppState;

async fn graphql_handler(
    schema: Extension<BlogSchema>,
    Extension(claims): Extension<Claims>,
    req: GraphQLRequest,
) -> GraphQLResponse {
    let mut inner = req.into_inner();
    inner = inner.data(claims);
    schema.execute(inner).await.into()
}

async fn graphql_playground() -> impl axum::response::IntoResponse {
    axum::response::Html(async_graphql::http::playground_source(
        async_graphql::http::GraphQLPlaygroundConfig::new("/graphql"),
    ))
}

// Liveness probe for the Docker healthcheck: cheap, no DB, no state.
async fn health() -> &'static str {
    "ok"
}

// This MUST return Router<()> instead of Router<AppState>
pub fn build_router(state: Arc<AppState>) -> Router<()> {
    let demos_dir_name = state
        .project_demo_config
        .dir
        .file_name()
        .and_then(|n| n.to_str())
        .expect("PROJECT_DEMOS_PATH must have a valid directory name");
    let project_demos_path = format!("/{}", demos_dir_name);

    let graphql_routes = Router::new()
        .route(
            "/graphql",
            get(graphql_playground).post(graphql_handler),
        )
        .layer(Extension(state.graphql_schema.clone()))
        .layer(middleware::from_fn(middlewares::auth::mod_check))
        .layer(middleware::from_fn_with_state(
            state.clone(),
            middlewares::auth::user_guard,
        ));

    Router::new()
        .nest("/media", handlers::media::routes(state.clone()))
        .nest("/auth", handlers::auth::routes(state.clone()))
        .nest("/users", handlers::user::routes(state.clone()))
        .nest("/posts", handlers::post::routes(state.clone()))
        .nest("/tags", handlers::post::tags_routes(state.clone()))
        .nest("/projects", handlers::project::routes(state.clone()))
        .nest("/games", handlers::game::routes(state.clone()))
        .nest("/v86", handlers::v86::routes(state.clone()))
        .nest("/sync", handlers::sync::routes(state.clone()))
        .route(
            &format!("{project_demos_path}/v86/{{*path}}"),
            get(|| async { axum::http::StatusCode::NOT_FOUND }),
        )
        .nest_service(
            &project_demos_path,
            get_service(ServeDir::new(&state.project_demo_config.dir)),
        )
        .nest("/series", handlers::series::routes(state.clone()))
        .nest("/audiobooks", handlers::audiobook::routes(state.clone()))
        .nest("/mail", handlers::mail::routes(state.clone()))
        .nest("/newsletter", handlers::newsletter::routes(state.clone()))
        .nest("/analytics", handlers::dashboard::analytics_routes(state.clone()))
        .nest("/dashboard", handlers::dashboard::routes(state.clone()))
        .route("/health", get(health))
        .merge(graphql_routes)
        // Routers merged with auth layers carry their own wrapped default
        // fallback, and the last one to be merged wins it. An unmatched path
        // (e.g. a stale v86 chunk URL) would otherwise surface as a misleading
        // `401 Invalid tokens` from that fallback's user_guard instead of a
        // plain 404. Pin an explicit public fallback so unmatched paths answer
        // 404, never auth errors.
        .fallback(|| async { axum::http::StatusCode::NOT_FOUND })
        .layer(layers::trace())
        .layer(layers::cors())
        .layer(layers::compression())
        .with_state(state)
        .layer(DefaultBodyLimit::max(100 * 1024 * 1024))
}

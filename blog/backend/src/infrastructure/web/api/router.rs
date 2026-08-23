use std::sync::Arc;

use axum::{
    Router,
    extract::DefaultBodyLimit,
    middleware,
    routing::{delete, get, get_service, patch, post, put},
};
use http::{HeaderValue, Method, header::CONTENT_TYPE};
use tower_http::compression::CompressionLayer;
use tower_http::trace::TraceLayer;
use tower_http::{
    cors::{AllowOrigin, CorsLayer},
    services::ServeDir,
};

use crate::domain::entities::secret::Claims;
use crate::infrastructure::web::graphql::BlogSchema;
use crate::infrastructure::web::{
    api::{handlers, middlewares},
    server::AppState,
};
use async_graphql_axum::{GraphQLRequest, GraphQLResponse};
use axum::extract::Extension;

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

// This MUST retuns Router<()> instead of Router<AppState>
pub fn build_router(state: Arc<AppState>) -> Router<()> {
    let cors_origins = std::env::var("ALLOWED_ORIGINS")
        .or_else(|_| std::env::var("ALLOWED_ORIGIN"))
        .unwrap_or_else(|_| {
            if cfg!(debug_assertions) {
                "http://localhost:3004,http://localhost:5000,http://localhost:3000,https://portfolio.huuthangle.site".to_string()
            } else {
                "https://portfolio.huuthangle.site".to_string()
            }
        });
    let cors_origins = cors_origins
        .split(',')
        .map(str::trim)
        .filter(|origin| !origin.is_empty())
        .map(|origin| origin.parse::<HeaderValue>().unwrap())
        .collect::<Vec<_>>();
    let cors = CorsLayer::new()
        .allow_origin(AllowOrigin::list(cors_origins))
        .allow_methods([Method::GET, Method::POST, Method::OPTIONS])
        .allow_headers([CONTENT_TYPE]);

    let auth_routes = Router::new()
        .route("/login", post(handlers::auth::login))
        .route("/register", post(handlers::auth::register))
        .route("/verify-email", get(handlers::auth::verify_email))
        .route(
            "/forgot-password",
            post(handlers::auth::request_password_reset),
        )
        .route("/reset-password", post(handlers::auth::reset_password))
        .route(
            "/resend-verification",
            post(handlers::auth::resend_verification),
        )
        .route("/refresh", post(handlers::auth::refresh_token));

    let user_routes = Router::new()
        // public route
        .merge(
            Router::new()
                .route("/{username}", get(handlers::user::get_user))
                .route("/", get(handlers::user::search)),
        )
        // optional-public route
        .merge(
            Router::new()
                .route("/{username}/posts", get(handlers::user::get_posts))
                .route(
                    "/{username}/comments",
                    get(handlers::user::get_latest_comments),
                )
                .layer(middleware::from_fn_with_state(
                    state.clone(),
                    middlewares::auth::optional_user_guard,
                )),
        )
        // protected route
        .merge(
            Router::new()
                .route("/me", get(handlers::user::me))
                .route("/me/details", patch(handlers::user::change_details))
                .route("/me/avatar", patch(handlers::user::change_avatar))
                .route("/me/check-mod", get(handlers::user::check_mod))
                .layer(middleware::from_fn_with_state(
                    state.clone(),
                    middlewares::auth::user_guard,
                ))
                .layer(DefaultBodyLimit::max(20 * 1024 * 1024)),
        );

    let media_routes = Router::new()
        // protected route
        .merge(
            Router::new()
                .route("/upload", post(handlers::media::upload))
                .route("/d/{short_name}", get(handlers::media::get_details))
                .route("/d/{short_name}", patch(handlers::media::change_details))
                .route("/d/{short_name}/aliases", get(handlers::media::get_aliases))
                .route("/d/{short_name}/aliases", post(handlers::media::add_alias))
                .route(
                    "/d/{short_name}/aliases/{alias}",
                    patch(handlers::media::change_alias),
                )
                .route(
                    "/d/{short_name}/aliases/{alias}",
                    delete(handlers::media::delete_alias),
                )
                .layer(middleware::from_fn(middlewares::auth::mod_check))
                .layer(middleware::from_fn_with_state(
                    state.clone(),
                    middlewares::auth::user_guard,
                ))
                .layer(DefaultBodyLimit::max(100 * 1024 * 1024)),
        )
        // public route
        .merge(
            Router::new()
                .route("/s/{short_name}", get(handlers::media::get_link))
                .route("/i/{short_name}", get(handlers::media::get_media))
                .route("/all", get(handlers::media::search)),
        )
        .fallback_service(get_service(ServeDir::new(&state.media_config.dir)));

    let post_routes = Router::new()
        // optional-auth routes (view post, new comment)
        .merge(
            Router::new()
                .route(
                    "/id/{post_id}/comments/new",
                    put(handlers::post::new_comment),
                )
                .route("/s/{post_slug}", get(handlers::post::get_post_by_slug))
                .layer(middleware::from_fn_with_state(
                    state.clone(),
                    middlewares::auth::optional_user_guard,
                ))
                .layer(DefaultBodyLimit::max(2 * 1024 * 1024)),
        )
        // mod-protected routes
        .merge(
            Router::new()
                .route("/new", post(handlers::post::new_post))
                .route("/id/{post_id}", post(handlers::post::publish))
                .route("/id/{post_id}", get(handlers::post::get_post_details))
                .route("/id/{post_id}", patch(handlers::post::update_post))
                .route("/id/{post_id}/cover", patch(handlers::post::change_cover))
                .route(
                    "/id/{post_id}/related",
                    patch(handlers::post::set_related_posts),
                )
                .layer(middleware::from_fn(middlewares::auth::mod_check))
                .layer(middleware::from_fn_with_state(
                    state.clone(),
                    middlewares::auth::user_guard,
                ))
                .layer(DefaultBodyLimit::max(100 * 1024 * 1024)),
        )
        // admin-protected routes
        .merge(
            Router::new()
                .route(
                    "/id/{post_id}/featured",
                    put(handlers::post::set_post_featured),
                )
                .layer(middleware::from_fn(middlewares::auth::admin_check))
                .layer(middleware::from_fn_with_state(
                    state.clone(),
                    middlewares::auth::user_guard,
                )),
        )
        // public routes
        .merge(
            Router::new()
                .route("/id/{post_id}/comments", get(handlers::post::get_comments))
                .route("/id/{post_id}/view", post(handlers::post::push_view))
                .route("/id/{post_id}/like", post(handlers::post::push_like))
                .route(
                    "/id/{post_id}/related",
                    get(handlers::post::get_related_posts),
                )
                .route("/featured", get(handlers::post::get_featured_posts))
                .route("/latest", get(handlers::post::get_latest_posts))
                .route("/check", get(handlers::post::check_post))
                .route("/categories", get(handlers::post::get_categories))
                .route("/", get(handlers::post::search)),
        );

    let project_routes = Router::new()
        // optional-auth routes
        .merge(
            Router::new()
                .route(
                    "/s/{project_slug}",
                    get(handlers::project::get_project_by_slug),
                )
                .layer(middleware::from_fn_with_state(
                    state.clone(),
                    middlewares::auth::optional_user_guard,
                ))
                .layer(DefaultBodyLimit::max(3 * 1024 * 1024)),
        )
        // mod-protected routes
        .merge(
            Router::new()
                .route("/new", post(handlers::project::new_project))
                .route("/all", get(handlers::project::get_all_projects))
                .route("/id/{project_id}", post(handlers::project::publish_project))
                .route(
                    "/id/{project_id}",
                    get(handlers::project::get_project_details),
                )
                .route("/id/{project_id}", patch(handlers::project::update_project))
                .route(
                    "/id/{project_id}",
                    delete(handlers::project::delete_project_draft),
                )
                .route(
                    "/id/{project_id}/cover",
                    patch(handlers::project::change_cover),
                )
                .layer(middleware::from_fn(middlewares::auth::mod_check))
                .layer(middleware::from_fn_with_state(
                    state.clone(),
                    middlewares::auth::user_guard,
                ))
                .layer(DefaultBodyLimit::max(100 * 1024 * 1024)),
        )
        // admin-protected routes
        .merge(
            Router::new()
                .route(
                    "/id/{project_id}/featured",
                    put(handlers::project::set_project_featured),
                )
                .layer(middleware::from_fn(middlewares::auth::admin_check))
                .layer(middleware::from_fn_with_state(
                    state.clone(),
                    middlewares::auth::user_guard,
                )),
        )
        // public routes
        .merge(
            Router::new()
                .route("/latest", get(handlers::project::get_latest_projects))
                .route("/featured", get(handlers::project::get_featured_projects))
                .route("/check", get(handlers::project::check_project)),
        );

    let game_routes = Router::new()
        // optional-auth routes
        .merge(
            Router::new()
                .route(
                    "/s/{game_slug}",
                    get(handlers::game::get_game_by_slug),
                )
                .route(
                    "/s/{game_slug}/v86/saves",
                    get(handlers::v86::get_game_save)
                        .put(handlers::v86::put_game_save)
                        .delete(handlers::v86::delete_game_save),
                )
                .layer(middleware::from_fn_with_state(
                    state.clone(),
                    middlewares::auth::optional_user_guard,
                ))
                .layer(DefaultBodyLimit::max(3 * 1024 * 1024)),
        )
        // mod-protected routes
        .merge(
            Router::new()
                .route("/new", post(handlers::game::new_game))
                .route(
                    "/id/{game_id}/jsdos/upload",
                    post(handlers::game::start_jsdos_upload),
                )
                .route(
                    "/id/{game_id}/jsdos/upload/{upload_id}/chunk/{chunk_index}",
                    put(handlers::game::append_jsdos_chunk),
                )
                .route(
                    "/id/{game_id}/jsdos/upload/{upload_id}/complete",
                    post(handlers::game::complete_jsdos_upload),
                )
                .route(
                    "/id/{game_id}/jsdos/upload/{upload_id}",
                    delete(handlers::game::abort_jsdos_upload),
                )
                .route("/all", get(handlers::game::get_all_games))
                .route("/id/{game_id}", post(handlers::game::publish_game))
                .route(
                    "/id/{game_id}",
                    get(handlers::game::get_game_details),
                )
                .route("/id/{game_id}", patch(handlers::game::update_game))
                .route(
                    "/id/{game_id}",
                    delete(handlers::game::delete_game_draft),
                )
                .route(
                    "/id/{game_id}/cover",
                    patch(handlers::game::change_cover),
                )
                .layer(middleware::from_fn(middlewares::auth::mod_check))
                .layer(middleware::from_fn_with_state(
                    state.clone(),
                    middlewares::auth::user_guard,
                ))
                .layer(DefaultBodyLimit::max(100 * 1024 * 1024)),
        )
        // admin-protected routes
        .merge(
            Router::new()
                .route(
                    "/id/{game_id}/featured",
                    put(handlers::game::set_game_featured),
                )
                .layer(middleware::from_fn(middlewares::auth::admin_check))
                .layer(middleware::from_fn_with_state(
                    state.clone(),
                    middlewares::auth::user_guard,
                )),
        )
        // public routes
        .merge(
            Router::new()
                .route(
                    "/s/{game_slug}/jsdos",
                    get(handlers::game::get_jsdos_bundle),
                )
                .route(
                    "/s/{game_slug}/v86/{sha256}/full.iso",
                    get(handlers::v86::get_game_iso),
                )
                .route(
                    "/s/{game_slug}/v86/{sha256}/{part}",
                    get(handlers::v86::get_game_chunk),
                )
                .route(
                    "/s/{game_slug}/v86/disk/{sha256}/{part}",
                    get(handlers::v86::get_game_disk_chunk),
                )
                .route("/latest", get(handlers::game::get_latest_games))
                .route("/featured", get(handlers::game::get_featured_games))
                .route("/check", get(handlers::game::check_game)),
        );

    let v86_routes = Router::new()
        .merge(
            Router::new()
                .route("/systems/active", get(handlers::v86::list_active_systems))
                .route("/launcher", get(handlers::v86::get_game_launcher))
                .route("/games/upload", post(handlers::v86::start_game_upload))
                .route(
                    "/games/upload/{upload_id}/disk/{part_index}",
                    put(handlers::v86::upload_game_disk_part),
                )
                .route(
                    "/games/upload/{upload_id}/iso/{variant_index}",
                    put(handlers::v86::upload_game_variant_iso),
                )
                .route(
                    "/games/upload/{upload_id}/complete",
                    post(handlers::v86::complete_game_upload),
                )
                .route(
                    "/games/upload/{upload_id}",
                    get(handlers::v86::get_game_upload_status),
                )
                .route(
                    "/games/upload/{upload_id}",
                    delete(handlers::v86::abort_game_upload),
                )
                .route(
                    "/snapshots/upload",
                    post(handlers::v86::start_snapshot_upload),
                )
                .route(
                    "/snapshots/upload/{upload_id}/chunk/{chunk_index}",
                    put(handlers::v86::append_snapshot_chunk),
                )
                .route(
                    "/snapshots/upload/{upload_id}/complete",
                    post(handlers::v86::complete_snapshot_upload),
                )
                .route(
                    "/snapshots/upload/{upload_id}",
                    delete(handlers::v86::abort_snapshot_upload),
                )
                .route(
                    "/games/id/{game_id}/snapshot",
                    get(handlers::v86::get_game_snapshot),
                )
                .route(
                    "/games/id/{game_id}/snapshot/{variant_index}",
                    delete(handlers::v86::delete_game_snapshot),
                )
                .route(
                    "/games/id/{game_id}/capture-runtime",
                    get(handlers::v86::get_game_capture_runtime),
                )
                .layer(middleware::from_fn(middlewares::auth::mod_check))
                .layer(middleware::from_fn_with_state(
                    state.clone(),
                    middlewares::auth::user_guard,
                ))
                .layer(DefaultBodyLimit::max(9 * 1024 * 1024)),
        )
        .merge(
            Router::new()
                .route("/systems", get(handlers::v86::list_systems))
                .route("/systems/status", get(handlers::v86::get_server_status))
                .route("/systems/upload", post(handlers::v86::start_system_upload))
                .route(
                    "/systems/upload/{upload_id}/part/{part_index}",
                    put(handlers::v86::upload_system_part),
                )
                .route(
                    "/systems/upload/{upload_id}/complete",
                    post(handlers::v86::complete_system_upload),
                )
                .route(
                    "/systems/upload/{upload_id}",
                    get(handlers::v86::get_system_upload_status),
                )
                .route(
                    "/systems/upload/{upload_id}",
                    delete(handlers::v86::abort_system_upload),
                )
                .route("/systems/{system_id}", patch(handlers::v86::update_system))
                .route("/systems/{system_id}", delete(handlers::v86::delete_system))
                .route(
                    "/systems/{system_id}/versions/{version_id}",
                    delete(handlers::v86::delete_system_version),
                )
                .layer(middleware::from_fn(middlewares::auth::admin_check))
                .layer(middleware::from_fn_with_state(
                    state.clone(),
                    middlewares::auth::user_guard,
                ))
                .layer(DefaultBodyLimit::max(9 * 1024 * 1024)),
        )
        .route("/systems/public", get(handlers::v86::list_public_systems))
        .route(
            "/assets/systems/{sha256}/{part}",
            get(handlers::v86::get_system_chunk),
        )
        .route(
            "/snapshots/{sha256}/{part}",
            get(handlers::v86::get_snapshot_blob),
        );

    let tag_routes = Router::new()
        .route("/", get(handlers::post::search_tags))
        .route("/{tag_slug}", get(handlers::post::get_posts_by_tag));

    let series_routes = Router::new()
        // optional
        .merge(Router::new().route("/public/all", get(handlers::series::get_all_series)))
        // user protected
        .merge(
            Router::new()
                .route("/all", get(handlers::series::get_series))
                .route("/new", post(handlers::series::new_series))
                .route(
                    "/id/{series_id}",
                    patch(handlers::series::add_post_to_series),
                )
                .route(
                    "/id/{series_id}",
                    delete(handlers::series::remove_post_from_series),
                )
                .route(
                    "/id/{series_id}/posts",
                    get(handlers::series::get_series_posts),
                )
                .layer(middleware::from_fn(middlewares::auth::mod_check))
                .layer(middleware::from_fn_with_state(
                    state.clone(),
                    middlewares::auth::user_guard,
                )),
        );

    let dashboard_routes = Router::new()
        .route("/overview", get(handlers::dashboard::get_overview))
        .route("/posts", get(handlers::dashboard::get_posts))
        .route("/projects", get(handlers::dashboard::get_projects))
        .route("/users", get(handlers::dashboard::get_users))
        .route(
            "/newsletter/subscribers",
            get(handlers::newsletter::list_subscribers),
        )
        .route(
            "/newsletter/campaigns",
            get(handlers::newsletter::list_campaigns),
        )
        .route("/newsletter/send", post(handlers::newsletter::send_campaign))
        .layer(middleware::from_fn(middlewares::auth::mod_check))
        .layer(middleware::from_fn_with_state(
            state.clone(),
            middlewares::auth::user_guard,
        ))
        .merge(
            Router::new()
                .route(
                    "/analytics/countries",
                    get(handlers::dashboard::get_visitor_countries),
                )
                .route("/tags/{tag_id}", patch(handlers::dashboard::update_tag))
                .route("/tags/{tag_id}", delete(handlers::dashboard::delete_tag))
                .route("/backup", get(handlers::backup::download_backup))
                .layer(middleware::from_fn(middlewares::auth::admin_check))
                .layer(middleware::from_fn_with_state(
                    state.clone(),
                    middlewares::auth::user_guard,
                )),
        );

    let analytics_routes = Router::new().route("/visit", post(handlers::dashboard::track_visit));

    #[allow(unused_mut)]
    let mut mail_routes = Router::new()
        .route("/contact-form", post(handlers::mail::receive_contact_form));

    #[cfg(debug_assertions)]
    {
        mail_routes = mail_routes.route("/preview", get(handlers::mail::preview_email_templates));
    }

    let mail_routes = Router::new().merge(mail_routes.layer(cors.clone()));

    let newsletter_routes = Router::new().merge(
        Router::new()
            .route("/subscribe", post(handlers::newsletter::subscribe))
            .route("/confirm", get(handlers::newsletter::confirm))
            .route(
                "/unsubscribe",
                get(handlers::newsletter::unsubscribe).post(handlers::newsletter::unsubscribe_by_email),
            )
            .layer(cors),
    );

    let graphql_routes = Router::new()
        .route(
            "/graphql",
            axum::routing::get(graphql_playground).post(graphql_handler),
        )
        .layer(Extension(state.graphql_schema.clone()))
        .layer(middleware::from_fn(middlewares::auth::mod_check))
        .layer(middleware::from_fn_with_state(
            state.clone(),
            middlewares::auth::user_guard,
        ));

    let demos_dir_name = state
        .project_demo_config
        .dir
        .file_name()
        .and_then(|n| n.to_str())
        .expect("PROJECT_DEMOS_PATH must have a valid directory name");
    let project_demos_path = format!("/{}", demos_dir_name);

    Router::new()
        .nest("/media", media_routes)
        .nest("/auth", auth_routes)
        .nest("/users", user_routes)
        .nest("/posts", post_routes)
        .nest("/projects", project_routes)
        .nest("/games", game_routes)
        .nest("/v86", v86_routes)
        .route(
            &format!("{project_demos_path}/v86/{{*path}}"),
            get(|| async { axum::http::StatusCode::NOT_FOUND }),
        )
        .nest_service(
            &project_demos_path,
            get_service(ServeDir::new(&state.project_demo_config.dir)),
        )
        .nest("/tags", tag_routes)
        .nest("/series", series_routes)
        .nest("/mail", mail_routes)
        .nest("/newsletter", newsletter_routes)
        .nest("/analytics", analytics_routes)
        .nest("/dashboard", dashboard_routes)
        .merge(graphql_routes)
        // Routers merged with auth layers carry their own wrapped default
        // fallback, and the last one to be merged wins it. An unmatched path
        // (e.g. a stale v86 chunk URL) would otherwise surface as a misleading
        // `401 Invalid tokens` from that fallback's user_guard instead of a
        // plain 404. Pin an explicit public fallback so unmatched paths answer
        // 404, never auth errors.
        .fallback(|| async { axum::http::StatusCode::NOT_FOUND })
        .layer(TraceLayer::new_for_http())
        .layer(CompressionLayer::new())
        .with_state(state)
        .layer(DefaultBodyLimit::max(100 * 1024 * 1024))
}

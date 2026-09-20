// Profile endpoints: me, public profiles, user posts and comments, mod
// check, and search.
use std::sync::Arc;

use axum::{
    Extension, Json,
    extract::{Path, Query, State},
    response::IntoResponse,
};

use crate::{
    application::{
        commands::user::{
            ChangeDetailsCommand, GetLatestCommentsCommand, GetPostsCommand, GetUserCommand,
            MeCommand, SearchUserCommand,
        },
        services::user::UserService,
    },
    domain::{
        entities::secret::Claims,
        errors::user::UserError,
    },
    helper::time::normalize_utc_timestamp,
    infrastructure::web::{
        api::handlers::user::dto::{
            ChangeDetailsBody, CheckModResponse, GetLatestCommentsQuery, GetLatestCommentsResponse,
            GetPostsQuery, GetPostsResponse, GetUserResponse, MeResponse, SearchUserQuery,
            SearchUserResponse,
        },
        server::AppState,
    },
};

#[axum::debug_handler]
pub async fn me(
    State(state): State<Arc<AppState>>,
    Extension(claims): Extension<Claims>,
) -> Result<Json<MeResponse>, UserError> {
    let user_id = claims
        .user_id
        .parse::<i64>()
        .map_err(|e| UserError::InternalError(e.to_string()))?;

    let cmd = MeCommand { user_id };

    match state.user_service.me(cmd).await {
        Ok(me) => Ok(Json(MeResponse {
            username: me.username,
            display_name: me.display_name,
            role: me.role,
            avatar_url: me.avatar_url,
        })),
        Err(e) => Err(e),
    }
}

#[axum::debug_handler]
pub async fn change_details(
    State(state): State<Arc<AppState>>,
    Extension(claims): Extension<Claims>,
    Json(body): Json<ChangeDetailsBody>,
) -> Result<impl IntoResponse, UserError> {
    let user_id = claims
        .user_id
        .parse::<i64>()
        .map_err(|e| UserError::InternalError(e.to_string()))?;
    state
        .user_service
        .change_details(ChangeDetailsCommand {
            user_id,
            display_name: body.display_name,
            bio: body.bio,
        })
        .await?;
    Ok(())
}

#[axum::debug_handler]
pub async fn get_user(
    State(state): State<Arc<AppState>>,
    Path(username): Path<String>,
) -> Result<Json<GetUserResponse>, UserError> {
    let cmd = GetUserCommand { username };
    match state.user_service.get_user(cmd).await {
        Ok(user) => Ok(Json(GetUserResponse {
            username: user.username,
            display_name: user.display_name,
            avatar_url: user.avatar_url,
            bio: user.bio,
            role: user.role,
        })),
        Err(e) => Err(e),
    }
}

#[axum::debug_handler]
pub async fn get_posts(
    State(state): State<Arc<AppState>>,
    Extension(opt_claims): Extension<Option<Claims>>,
    Path(username): Path<String>,
    Query(query): Query<GetPostsQuery>,
) -> Result<impl IntoResponse, UserError> {
    let limit = query.limit.unwrap_or(5);
    let offset = query.offset.unwrap_or(0);
    let user_id = match opt_claims {
        None => None,
        Some(claims) => Some(
            claims
                .user_id
                .parse::<i64>()
                .map_err(|e| UserError::InternalError(e.to_string()))?,
        ),
    };

    let posts = state
        .user_service
        .get_posts(GetPostsCommand {
            user_id,
            username,
            limit,
            offset,
        })
        .await?;

    let wrapped_posts = GetPostsResponse {
        posts: posts
            .into_iter()
            .map(|post| Post {
                id: post.id,
                title: post.title,
                slug: post.slug,
                tag_names: post.tag_names,
                tag_slugs: post.tag_slugs,
                excerpt: post.excerpt,
                author_name: post.author_name,
                author_slug: post.author_slug,
                status: post.status,
                url: post.url,
                cover_media_type: post.cover_media_type,
                reading_time_minutes: post.reading_time_minutes,
            })
            .collect(),
    };

    Ok(Json(wrapped_posts))
}

pub async fn get_latest_comments(
    State(state): State<Arc<AppState>>,
    Path(username): Path<String>,
    Query(query): Query<GetLatestCommentsQuery>,
) -> Result<impl IntoResponse, UserError> {
    let comments = state
        .user_service
        .get_latest_comments(GetLatestCommentsCommand {
            username,
            limit: query.limit.unwrap_or(5),
            offset: query.offset.unwrap_or(0),
        })
        .await?;

    Ok(Json(GetLatestCommentsResponse {
        comments: comments
            .into_iter()
            .map(|comment| LatestComment {
                id: comment.id,
                parent_id: comment.parent_id,
                content: comment.content,
                created_at: normalize_utc_timestamp(comment.created_at),
                post_title: comment.post_title,
                post_slug: comment.post_slug,
                avatar_url: comment.avatar_url,
                display_name: comment.display_name,
                username: comment.username,
            })
            .collect(),
    }))
}

#[axum::debug_handler]
pub async fn check_mod(
    Extension(claims): Extension<Claims>,
) -> Result<impl IntoResponse, UserError> {
    let is_authorized = UserRole::try_from(claims.role.clone())
        .is_ok_and(|role| UserRole::Moderator.include(&role));

    Ok(Json(CheckModResponse { is_authorized }))
}

#[axum::debug_handler]
pub async fn search(
    State(state): State<Arc<AppState>>,
    Query(query): Query<SearchUserQuery>,
) -> Result<impl IntoResponse, UserError> {
    let term = query.term;
    let size = query.size.unwrap_or(1);
    let offset = query.offset.unwrap_or(0);

    let user_snapshots = state
        .user_service
        .search(SearchUserCommand { term, size, offset })
        .await?;

    let users: Vec<SearchUserResult> = user_snapshots
        .into_iter()
        .map(
            |UserSummary {
                 username,
                 display_name,
                 role,
                 avatar_url,
             }| SearchUserResult {
                username,
                display_name,
                role,
                avatar_url,
            },
        )
        .collect();

    Ok(Json(SearchUserResponse { users }))
}

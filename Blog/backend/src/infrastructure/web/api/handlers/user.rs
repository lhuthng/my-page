use std::sync::Arc;

use axum::{
    Extension, Json,
    extract::{Path, Query, State},
    response::IntoResponse,
};
use serde::{Deserialize, Serialize};

use crate::{
    application::{
        commands::user::{ChangeDetailsCommand, GetPostsCommand, GetUserCommand, MeCommand},
        services::user::UserService,
    },
    domain::{
        entities::{secret::Claims, user::UserRole},
        errors::user::UserError,
    },
    infrastructure::web::server::AppState,
};

#[derive(Debug, Serialize)]
pub struct MeResponse {
    username: String,
    display_name: String,
    role: String,
}

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
        })),
        Err(e) => Err(e),
    }
}

#[derive(Debug, Deserialize)]
pub struct ChangeDetailsBody {
    pub display_name: Option<String>,
    pub bio: Option<String>,
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

#[derive(Debug, Serialize)]
pub struct GetUserResponse {
    username: String,
    display_name: String,
    bio: String,
    role: String,
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
            bio: user.bio,
            role: user.role,
        })),
        Err(e) => Err(e),
    }
}

#[derive(Deserialize)]
pub struct GetPostsQuery {
    pub limit: Option<i64>,
    pub offset: Option<i64>,
}

#[derive(Serialize)]
pub struct Post {
    pub id: i64,
    pub title: String,
    pub slug: String,
    pub tag_names: Vec<String>,
    pub tag_slugs: Vec<String>,
    pub excerpt: String,
    pub author_name: String,
    pub author_slug: String,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub url: Option<String>,
}

#[derive(Serialize)]
pub struct GetPostsResponse {
    pub posts: Vec<Post>,
}

#[axum::debug_handler]
pub async fn get_posts(
    State(state): State<Arc<AppState>>,
    Path(username): Path<String>,
    Query(query): Query<GetPostsQuery>,
) -> Result<impl IntoResponse, UserError> {
    let limit = query.limit.unwrap_or(5);
    let offset = query.offset.unwrap_or(0);

    let posts = state
        .user_service
        .get_posts(GetPostsCommand {
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
                url: post.url,
            })
            .collect(),
    };

    Ok(Json(wrapped_posts))
}

#[derive(Debug, Serialize)]
pub struct CheckModResponse {
    is_authorized: bool,
}

#[axum::debug_handler]
pub async fn check_mod(
    Extension(claims): Extension<Claims>,
) -> Result<impl IntoResponse, UserError> {
    let is_authorized = UserRole::try_from(claims.role.clone())
        .is_ok_and(|role| UserRole::Moderator.include(&role));

    Ok(Json(CheckModResponse { is_authorized }))
}

use std::sync::Arc;

use axum::{
    Extension, Json,
    body::Bytes,
    extract::{Multipart, Path, Query, State},
    response::IntoResponse,
};
use serde::{Deserialize, Serialize};

use crate::{
    application::{
        commands::{
            media::ChangeAvatarCommand,
            user::{
                ChangeDetailsCommand, GetPostsCommand, GetUserCommand, MeCommand, SearchUserCommand,
            },
        },
        services::{media::MediaService, user::UserService},
    },
    domain::{
        entities::{
            secret::Claims,
            user::{UserRole, UserSummary},
        },
        errors::{media::MediaError, user::UserError},
    },
    infrastructure::web::{
        api::handlers::common::{MediumData, extract_medium},
        server::AppState,
    },
};

#[derive(Debug, Serialize)]
pub struct MeResponse {
    username: String,
    display_name: String,
    role: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    avatar_url: Option<String>,
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
            avatar_url: me.avatar_url,
        })),
        Err(e) => Err(e),
    }
}

#[axum::debug_handler]
pub async fn change_avatar(
    State(state): State<Arc<AppState>>,
    Extension(claims): Extension<Claims>,
    mut multipart: Multipart,
) -> Result<impl IntoResponse, MediaError> {
    let user_id = claims
        .user_id
        .parse::<i64>()
        .map_err(|_| MediaError::InternalError("Cannot parse id.".to_string()))?;

    let mut opt_filename: Option<String> = None;
    let mut opt_content_type: Option<String> = None;
    let mut opt_bytes: Option<Bytes> = None;

    while let Some(field) = multipart
        .next_field()
        .await
        .map_err(|e| MediaError::InternalError(e.to_string()))?
    {
        let field_name = field.name().ok_or(MediaError::UploadFailed(
            "Empty field detected.".to_string(),
        ))?;

        if field_name == "file" {
            if opt_filename.is_some() {
                return Err(MediaError::UploadFailed(
                    "Only one media is allowed at a time.".to_string(),
                ));
            }

            let MediumData {
                filename,
                content_type,
                bytes,
            } = extract_medium(field).await?;

            opt_filename = Some(filename);
            opt_content_type = Some(content_type);
            opt_bytes = Some(bytes);
        }
    }

    let filename =
        opt_filename.ok_or_else(|| MediaError::UploadFailed("Missing file".to_string()))?;
    let content_type = opt_content_type
        .ok_or_else(|| MediaError::UploadFailed("Missing content type".to_string()))?;
    let bytes =
        opt_bytes.ok_or_else(|| MediaError::UploadFailed("Missing file bytes".to_string()))?;

    state
        .media_service
        .change_avatar(
            ChangeAvatarCommand {
                user_id,
                filename,
                content_type,
                bytes,
            },
            &state.media_config,
        )
        .await?;

    Ok(())
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
    #[serde(skip_serializing_if = "Option::is_none")]
    avatar_url: Option<String>,
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
            avatar_url: user.avatar_url,
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

#[derive(Debug, Serialize, Deserialize)]
pub struct SearchUserQuery {
    pub term: String,
    pub size: Option<i64>,
    pub offset: Option<i64>,
}

#[derive(Serialize)]
pub struct SearchUserResult {
    pub username: String,
    pub display_name: String,
    pub role: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub avatar_url: Option<String>,
}

#[derive(Serialize)]
pub struct SearchUserResponse {
    pub users: Vec<SearchUserResult>,
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

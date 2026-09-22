// Comments: authenticated and anonymous creation, per-post listing.
use std::sync::Arc;

use axum::{
    Extension, Json,
    extract::{Path, Query, State},
    response::IntoResponse,
};

use crate::{
    application::{
        commands::post::{
            GetCommentsCommand, PostNewAnynymouseCommentCommand, PostNewCommentCommand,
        },
        services::post::PostService,
    },
    domain::{entities::secret::Claims, errors::post::PostError},
    helper::time::normalize_utc_timestamp,
    infrastructure::web::{
        api::handlers::post::dto::{CommentsQuery, NewCommentBody},
        api::handlers::post::response::{Comment, CommentsResponse, NewCommentResponse},
        server::AppState,
    },
};

pub async fn new_comment(
    State(state): State<Arc<AppState>>,
    Extension(opt_claims): Extension<Option<Claims>>,
    Path(post_id_str): Path<String>,
    Json(body): Json<NewCommentBody>,
) -> Result<impl IntoResponse, PostError> {
    let post_id: i64 = post_id_str.parse().map_err(|_| PostError::PostNotFound)?;
    let comment_id = match opt_claims {
        Some(claims) => {
            let user_id = claims
                .user_id
                .parse::<i64>()
                .map_err(|e| PostError::InternalError(e.to_string()))?;

            state
                .post_service
                .post_new_comment(PostNewCommentCommand {
                    post_id,
                    user_id,
                    parent_id: body.parent_id,
                    content: body.content,
                    guest_identity: body.guest_identity,
                })
                .await?
        }
        None => {
            let guest_identity = body.guest_identity.ok_or(PostError::UploadFailed(
                "guest_identity is required for anonymous comments".into(),
            ))?;
            state
                .post_service
                .post_new_anonymous_comment(PostNewAnynymouseCommentCommand {
                    post_id,
                    parent_id: body.parent_id,
                    content: body.content,
                    guest_identity,
                })
                .await?
        }
    };

    Ok(Json(NewCommentResponse { comment_id }))
}

pub async fn get_comments(
    State(state): State<Arc<AppState>>,
    Path(post_id_str): Path<String>,
    Query(query): Query<CommentsQuery>,
) -> Result<impl IntoResponse, PostError> {
    let before = query.before;
    let limit = crate::helper::string::clamp_page_size(query.limit, 1, 100);
    let parent_id = query.parent_id;
    let post_id = post_id_str.parse::<i64>().unwrap();

    let page = state
        .post_service
        .get_comments(GetCommentsCommand {
            post_id,
            limit,
            before,
            parent_id,
        })
        .await?;

    let comments: Vec<Comment> = page
        .comments
        .into_iter()
        .map(|comment| Comment {
            id: comment.id,
            parent_id: comment.parent_id,
            direct_reply_count: comment.direct_reply_count,
            content: comment.content,
            created_at: normalize_utc_timestamp(comment.created_at),
            username: comment.username,
            display_name: comment.display_name,
            avatar_url: comment.avatar_url,
            user_role: comment.user_role,
        })
        .collect();

    let wrapped_comments = CommentsResponse {
        comments,
        has_more: page.has_more,
    };

    Ok(Json(wrapped_comments))
}

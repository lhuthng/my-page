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
    infrastructure::{
        mail::{comment_notification, send_comment_notification_email},
        web::{
            api::handlers::post::dto::{CommentsQuery, NewCommentBody},
            api::handlers::post::response::{Comment, CommentsResponse, NewCommentResponse},
            server::AppState,
        },
    },
};

/// Must stay in sync with `blog/frontend/src/lib/features/comments/guest-identities.js`.
const GUEST_IDENTITIES: &[&str] = &["cutiecube", "straymeowmet", "loafphantom"];

/// How many characters of a fresh comment are quoted in the author email.
const COMMENT_EXCERPT_CHARS: usize = 280;

/// The post's title/routing plus its author's contact details, as loaded for
/// the notification email. `author_*` fields stay optional because the joins
/// are LEFT: a post can outlive its author.
#[derive(Debug, sqlx::FromRow)]
struct AuthorEmailRow {
    post_title: String,
    slug: String,
    content_kind: String,
    author_id: Option<i64>,
    author_email: Option<String>,
    author_name: Option<String>,
}

pub async fn new_comment(
    State(state): State<Arc<AppState>>,
    Extension(opt_claims): Extension<Option<Claims>>,
    Path(post_id_str): Path<String>,
    Json(body): Json<NewCommentBody>,
) -> Result<impl IntoResponse, PostError> {
    let post_id: i64 = post_id_str.parse().map_err(|_| PostError::PostNotFound)?;

    // The picker in the comment UI offers exactly these avatars. The route is
    // public, so the backend cannot trust that: without this check any caller
    // could store an arbitrary string in `comments.guest_identity`.
    let guest_identity = match body.guest_identity.as_deref() {
        Some(code) if GUEST_IDENTITIES.contains(&code) => Some(code.to_string()),
        Some(_) => {
            return Err(PostError::UploadFailed("Unknown guest identity".into()));
        }
        None => None,
    };

    // Captured before the commands below consume the values, so the
    // notification task can quote who wrote the comment and what it says.
    let mut commenter_user_id = None;
    let email_guest_identity = guest_identity.clone();
    let email_content = body.content.clone();

    let comment_id = match opt_claims {
        Some(claims) => {
            let user_id = claims
                .user_id
                .parse::<i64>()
                .map_err(|e| PostError::InternalError(e.to_string()))?;
            commenter_user_id = Some(user_id);

            state
                .post_service
                .post_new_comment(PostNewCommentCommand {
                    post_id,
                    user_id,
                    parent_id: body.parent_id,
                    content: body.content,
                    guest_identity,
                })
                .await?
        }
        None => {
            let guest_identity = guest_identity.ok_or(PostError::UploadFailed(
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

    spawn_comment_author_email(
        &state,
        post_id,
        comment_id,
        commenter_user_id,
        email_guest_identity,
        email_content,
    );

    Ok(Json(NewCommentResponse { comment_id }))
}

/// Emails the post's author about a fresh comment. Fire-and-forget, in the
/// same spirit as the publish-time newsletter fan-out: sending is best-effort
/// and must never fail (or slow down) an already-succeeded comment.
fn spawn_comment_author_email(
    state: &Arc<AppState>,
    post_id: i64,
    comment_id: i64,
    commenter_user_id: Option<i64>,
    guest_identity: Option<String>,
    content: String,
) {
    // Mail disabled: comments keep working, there is just no email.
    if state.config.mail.is_none() {
        return;
    }

    let state = Arc::clone(state);
    tokio::spawn(async move {
        if let Err(e) = notify_comment_author(
            &state,
            post_id,
            comment_id,
            commenter_user_id,
            guest_identity,
            &content,
        )
        .await
        {
            tracing::error!(
                "Failed to send comment notification email for post {}: {}",
                post_id,
                e
            );
        }
    });
}

/// Best-effort author notification for one new comment.
async fn notify_comment_author(
    state: &Arc<AppState>,
    post_id: i64,
    comment_id: i64,
    commenter_user_id: Option<i64>,
    guest_identity: Option<String>,
    content: &str,
) -> Result<(), String> {
    let mail_config = state
        .config
        .mail
        .clone()
        .ok_or_else(|| "mail is not configured".to_string())?;
    let app_base_url = state.config.app_base_url.clone();

    let row: Option<AuthorEmailRow> = sqlx::query_as(
        r#"
        SELECT p.title AS post_title, p.slug, p.content_kind, p.user_id AS author_id, u.email AS author_email,
               COALESCE(um.display_name, u.username) AS author_name
        FROM posts p
        LEFT JOIN users u ON u.id = p.user_id
        LEFT JOIN user_meta um ON um.user_id = p.user_id
        WHERE p.id = ?
        "#,
    )
    .bind(post_id)
    .fetch_optional(&state.post_service.pool)
    .await
    .map_err(|e| e.to_string())?;

    let Some(AuthorEmailRow {
        post_title,
        slug,
        content_kind,
        author_id,
        author_email,
        author_name,
    }) = row
    else {
        // The post vanished between the insert and this read; nothing to notify.
        return Ok(());
    };
    let Some(author_id) = author_id else {
        return Ok(()); // Orphaned post (author deleted): nobody to notify.
    };
    let Some(author_email) = author_email else {
        return Ok(()); // Defensive: users.email is NOT NULL, but never panic.
    };
    // The author commenting on their own post must not email themselves.
    if commenter_user_id == Some(author_id) {
        return Ok(());
    }

    let commenter_name = match commenter_user_id {
        Some(user_id) => {
            let row: Option<(String,)> = sqlx::query_as(
                r#"
                SELECT COALESCE(um.display_name, u.username)
                FROM users u
                LEFT JOIN user_meta um ON um.user_id = u.id
                WHERE u.id = ?
                "#,
            )
            .bind(user_id)
            .fetch_optional(&state.post_service.pool)
            .await
            .map_err(|e| e.to_string())?;
            row.map(|(name,)| name)
                .unwrap_or_else(|| "Someone".to_string())
        }
        // The identity was already validated against the allowlist above.
        None => guest_identity.unwrap_or_else(|| "Anonymous".to_string()),
    };

    // Deep links must land on the route the kind is served from, matching the
    // frontend's commentHref: posts, projects, and games each have their own.
    let section = match content_kind.as_str() {
        "project" => "projects",
        "game" => "games",
        _ => "posts",
    };
    let comment_url = format!(
        "{}/{}/{}#comment-{}",
        app_base_url.trim_end_matches('/'),
        section,
        slug,
        comment_id
    );

    // Quote a short head of the comment; the full text is one click away.
    let excerpt: String = {
        let mut chars = content.chars();
        let head: String = chars.by_ref().take(COMMENT_EXCERPT_CHARS).collect();
        if chars.next().is_some() {
            format!("{head}…")
        } else {
            head
        }
    };

    let (text, html) = comment_notification(
        &app_base_url,
        author_name.as_deref().unwrap_or("there"),
        &post_title,
        &commenter_name,
        &excerpt,
        &comment_url,
    );

    send_comment_notification_email(
        &mail_config,
        &author_email,
        author_name.as_deref(),
        &format!("New comment on \"{post_title}\""),
        &text,
        &html,
    )
    .await
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

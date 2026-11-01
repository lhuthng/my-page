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
    domain::{
        entities::{
            notification::{CommentRecipient, NotificationKind},
            secret::Claims,
        },
        errors::post::PostError,
    },
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

    let created = match opt_claims {
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

    spawn_comment_emails(
        &state,
        post_id,
        created.comment_id,
        commenter_user_id,
        email_guest_identity,
        email_content,
        created.recipients,
    );

    Ok(Json(NewCommentResponse {
        comment_id: created.comment_id,
    }))
}

/// Emails everyone the resolver selected for this comment. Fire-and-forget, in
/// the same spirit as the publish-time newsletter fan-out: sending is
/// best-effort and must never fail (or slow down) an already-succeeded comment.
fn spawn_comment_emails(
    state: &Arc<AppState>,
    post_id: i64,
    comment_id: i64,
    commenter_user_id: Option<i64>,
    guest_identity: Option<String>,
    content: String,
    recipients: Vec<CommentRecipient>,
) {
    // Mail disabled: comments and in-app notifications keep working.
    if state.config.mail.is_none() || recipients.is_empty() {
        return;
    }

    let state = Arc::clone(state);
    tokio::spawn(async move {
        if let Err(e) = notify_comment_recipients(
            &state,
            post_id,
            comment_id,
            commenter_user_id,
            guest_identity,
            &content,
            &recipients,
        )
        .await
        {
            tracing::error!(
                "Failed to send comment notification emails for post {}: {}",
                post_id,
                e
            );
        }
    });
}

#[derive(Debug, sqlx::FromRow)]
struct CommentMailPostRow {
    post_title: String,
    slug: String,
    content_kind: String,
}

#[derive(Debug, sqlx::FromRow)]
struct CommentMailRecipientRow {
    username: String,
    email: String,
    email_verified_at: Option<String>,
    display_name: String,
    notify_reply_email: i64,
    notify_mention_email: i64,
    notify_post_comment_email: i64,
}

impl CommentMailRecipientRow {
    fn wants_email(&self, kind: NotificationKind) -> bool {
        match kind {
            NotificationKind::Reply => self.notify_reply_email != 0,
            NotificationKind::Mention => self.notify_mention_email != 0,
            NotificationKind::PostComment => self.notify_post_comment_email != 0,
        }
    }
}

/// Best-effort email fan-out for one new comment. The recipient list was
/// resolved in the comment transaction, so email and in-app agree by
/// construction. Each recipient gets at most one email for this comment.
async fn notify_comment_recipients(
    state: &Arc<AppState>,
    post_id: i64,
    comment_id: i64,
    commenter_user_id: Option<i64>,
    guest_identity: Option<String>,
    content: &str,
    recipients: &[CommentRecipient],
) -> Result<(), String> {
    let mail_config = state
        .config
        .mail
        .clone()
        .ok_or_else(|| "mail is not configured".to_string())?;
    let app_base_url = state.config.app_base_url.clone();

    let post: Option<CommentMailPostRow> = sqlx::query_as(
        r#"
        SELECT p.title AS post_title, p.slug, p.content_kind
        FROM posts p
        WHERE p.id = ?
        "#,
    )
    .bind(post_id)
    .fetch_optional(&state.post_service.pool)
    .await
    .map_err(|e| e.to_string())?;

    let Some(post) = post else {
        // The post vanished between the insert and this read; nothing to send.
        return Ok(());
    };

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
    let section = match post.content_kind.as_str() {
        "project" => "projects",
        "game" => "games",
        _ => "posts",
    };
    let comment_url = format!(
        "{}/{}/{}#comment-{}",
        app_base_url.trim_end_matches('/'),
        section,
        post.slug,
        comment_id
    );

    for recipient in recipients {
        let user: Option<CommentMailRecipientRow> = sqlx::query_as(
            r#"
            SELECT u.username,
                   u.email,
                   u.email_verified_at,
                   COALESCE(um.display_name, u.username) AS display_name,
                   u.notify_reply_email,
                   u.notify_mention_email,
                   u.notify_post_comment_email
            FROM users u
            LEFT JOIN user_meta um ON um.user_id = u.id
            WHERE u.id = ?
            "#,
        )
        .bind(recipient.user_id)
        .fetch_optional(&state.post_service.pool)
        .await
        .map_err(|e| e.to_string())?;

        let Some(user) = user else {
            continue; // The account disappeared after the comment landed.
        };
        // Unverified addresses bounce; in-app still records the event.
        if user.email_verified_at.is_none() || !user.wants_email(recipient.kind) {
            continue;
        }

        let manage_url = format!(
            "{}/profiles/{}?section=notifications",
            app_base_url.trim_end_matches('/'),
            user.username
        );

        // The template renders the markdown and caps the quote, so pass the
        // raw comment body.
        let (text, html) = comment_notification(
            &app_base_url,
            recipient.kind,
            &user.display_name,
            &post.post_title,
            &commenter_name,
            content,
            &comment_url,
            &manage_url,
        );

        let subject = match recipient.kind {
            NotificationKind::Reply => format!("New reply on \"{}\"", post.post_title),
            NotificationKind::Mention => {
                format!("You were mentioned on \"{}\"", post.post_title)
            }
            NotificationKind::PostComment => format!("New comment on \"{}\"", post.post_title),
        };

        if let Err(e) = send_comment_notification_email(
            &mail_config,
            &user.email,
            Some(&user.display_name),
            &subject,
            &text,
            &html,
        )
        .await
        {
            // One bad address must not starve the rest of the recipient list.
            tracing::error!(
                "Failed to send {} notification for comment {} to user {}: {}",
                recipient.kind.as_str(),
                comment_id,
                recipient.user_id,
                e
            );
        }
    }

    Ok(())
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

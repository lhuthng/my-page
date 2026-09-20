// Post write methods: create/update/publish, related posts, featured flag,
// and cover metadata.
use std::collections::HashMap;

use sqlx::Row;

use crate::application::{
    commands::post::{
        NewPostCommand, PublishCommand, SetFeaturedPostCommand, SetRelatedPostsCommand,
        UpdatePostCommand, UpdatePostCoverCommand,
    },
    services::post::PostService,
};
use crate::domain::entities::post::PostStats;
use crate::domain::errors::post::PostError;

use super::links::{link_post_media, link_post_tags, resolve_tag_ids, MAX_TAGS_PER_POST};
use super::PostServiceImpl;

macro_rules! set_opt {
    ($fields:expr, $( ($str: expr, $opt:expr) ),* ) => {
        $(
            $opt.is_some().then(|| $fields.push(format!("{} = ?", $str)));
        )*
    };
}

macro_rules! bind_opt {
    ($query:expr, $( $opt:expr ),* ) => {
        $(
            if let Some(val) = $opt {
                $query = $query.bind(val);
            }
        )*
    };
}

#[async_trait::async_trait]
impl PostService for PostServiceImpl {
    async fn new_post(&self, cmd: NewPostCommand) -> Result<i64, PostError> {
        let title = crate::helper::string::validate_text(&cmd.title, "Title", 200)
            .map_err(PostError::Validation)?;
        let slug =
            crate::helper::string::validate_slug(&cmd.slug).map_err(PostError::Validation)?;
        let excerpt = crate::helper::string::validate_text(&cmd.excerpt, "Excerpt", 400)
            .map_err(PostError::Validation)?;
        let content = crate::helper::string::validate_body(&cmd.content, "Content")
            .map_err(PostError::Validation)?;

        let mut tx = self.pool.begin().await?;
        let reading_time_minutes =
            crate::helper::reading_time::estimate_reading_time_minutes(&content);
        let post_id: i64 = sqlx::query_scalar(
            r#"
            INSERT INTO posts (user_id, title, slug, excerpt, content, status, content_kind, reading_time_minutes)
            VALUES (?, ?, ?, ?, ?, ?, ?, ?)
            RETURNING id
            "#,
        )
        .bind(cmd.user_id)
        .bind(&title)
        .bind(&slug)
        .bind(&excerpt)
        .bind(&content)
        .bind("draft".to_string())
        .bind(&cmd.content_kind)
        .bind(reading_time_minutes)
        .fetch_one(&mut *tx)
        .await?;

        sqlx::query("INSERT INTO post_stats (post_id) VALUES (?)")
            .bind(post_id)
            .execute(&mut *tx)
            .await?;

        link_post_media(&mut tx, post_id, &cmd.media_usage).await?;

        let tag_ids = resolve_tag_ids(&mut tx, &cmd.tags).await?;
        link_post_tags(&mut tx, post_id, &tag_ids).await?;

        tx.commit().await?;
        Ok(post_id)
    }
    async fn update_post(&self, cmd: UpdatePostCommand) -> Result<String, PostError> {
        use crate::{application::commands::post::UpdatePostCommand as C, helper::string::*};
        let cmd = C {
            title: cmd
                .title
                .map(|v| validate_text(&v, "Title", 200).map_err(PostError::Validation))
                .transpose()?,
            slug: cmd
                .slug
                .map(|v| validate_slug(&v).map_err(PostError::Validation))
                .transpose()?,
            excerpt: cmd
                .excerpt
                .map(|v| validate_text(&v, "Excerpt", 400).map_err(PostError::Validation))
                .transpose()?,
            content: cmd
                .content
                .map(|v| validate_body(&v, "Content").map_err(PostError::Validation))
                .transpose()?,
            ..cmd
        };

        let mut tx = self.pool.begin().await?;

        // Authorise before writing anything. This has to be an explicit check
        // rather than an `AND user_id = ?` predicate on the UPDATE, because a
        // patch that only changes tags or media never reaches the UPDATE at all
        // and would otherwise skip the check entirely.
        let current = sqlx::query_as::<_, (i64, Option<String>)>(
            "SELECT user_id, updated_at FROM posts WHERE id = ?",
        )
        .bind(cmd.post_id)
        .fetch_optional(&mut *tx)
        .await?;
        let (owner_id, current_updated_at) = current.ok_or(PostError::PostNotFound)?;
        if let Some(required) = cmd.required_author_id
            && required != owner_id
        {
            return Err(PostError::Forbidden);
        }

        // Optimistic lock: refuse to overwrite a row that moved under us.
        let current_updated_at = current_updated_at.unwrap_or_default();
        if let Some(expected) = &cmd.expected_updated_at
            && crate::helper::time::normalize_utc_timestamp(expected)
                != crate::helper::time::normalize_utc_timestamp(&current_updated_at)
        {
            return Err(PostError::Conflict(current_updated_at));
        }

        let mut set_fields: Vec<String> = vec![];

        // Reading time follows the body, and the body is now a single column —
        // so it is recomputed on every save rather than at publish time.
        let reading_time_opt: Option<i64> = cmd
            .content
            .clone()
            .map(|text| crate::helper::reading_time::estimate_reading_time_minutes(&text));

        set_opt!(
            set_fields,
            ("title", cmd.title),
            ("slug", cmd.slug),
            ("excerpt", cmd.excerpt),
            ("content", cmd.content),
            ("reading_time_minutes", reading_time_opt)
        );

        // Always bump updated_at, even for a tags-or-media-only patch: it is the
        // optimistic-lock token, so every accepted write has to move it. It
        // carries no placeholder, so appending it last keeps the positional
        // binds below aligned with the fields set_opt! pushed.
        set_fields.push("updated_at = CURRENT_TIMESTAMP".to_string());

        let set_stn = set_fields.join(", ");
        let sql = format!(
            r#"
            UPDATE posts
            SET {}
            WHERE id = ?
            RETURNING updated_at
            "#,
            set_stn
        );
        let mut query = sqlx::query_scalar::<_, String>(&sql);

        bind_opt!(
            query,
            cmd.title,
            cmd.slug,
            cmd.excerpt,
            cmd.content,
            reading_time_opt
        );

        query = query.bind(cmd.post_id);

        let new_updated_at: String = query.fetch_one(&mut *tx).await?;
        if let Some(media_usage) = &cmd.media_usage {
            sqlx::query("DELETE FROM post_media_usages WHERE post_id = ?")
                .bind(cmd.post_id)
                .execute(&mut *tx)
                .await?;

            link_post_media(&mut tx, cmd.post_id, media_usage).await?;
        }
        if let Some(tags) = cmd.tags {
            let tag_ids = resolve_tag_ids(&mut tx, &tags).await?;
            link_post_tags(&mut tx, cmd.post_id, &tag_ids).await?;
        }
        tx.commit().await?;
        Ok(crate::helper::time::normalize_utc_timestamp(new_updated_at))
    }
    async fn publish(&self, cmd: PublishCommand) -> Result<(), PostError> {
        // Publishing is now a pure visibility flip. The body is a single column
        // that the last save already wrote, so there is nothing to copy across
        // and no reading time to recompute — only the state changes.
        // `published_at` is stamped the first time and preserved afterwards.
        let result = sqlx::query(
            r#"
            UPDATE posts
            SET
                published_at = CASE
                    WHEN status = 'draft' THEN CURRENT_TIMESTAMP
                    ELSE published_at
                END,
                updated_at = CURRENT_TIMESTAMP,
                status = 'published'
            WHERE id = ? AND user_id = ? AND deleted_at IS NULL
            "#,
        )
        .bind(cmd.post_id)
        .bind(cmd.user_id)
        .execute(&self.pool)
        .await
        .map_err(|e| PostError::InternalError(e.to_string()))?;

        // Zero rows means either the id is unknown or it belongs to someone
        // else — the same outcome the old ownership SELECT produced.
        if result.rows_affected() == 0 {
            return Err(PostError::PostNotFound);
        }

        Ok(())
    }
    // async fn unpublish(&self, cmd: UnpublishCommand) -> Result<(), PostError> {

    // }
    async fn set_related_posts(&self, cmd: SetRelatedPostsCommand) -> Result<(), PostError> {
        let mut tx = self.pool.begin().await?;

        let owner_id: Option<i64> = sqlx::query_scalar("SELECT user_id FROM posts WHERE id = ?")
            .bind(cmd.post_id)
            .fetch_optional(&mut *tx)
            .await?;
        if owner_id.ok_or(PostError::PostNotFound)? != cmd.user_id {
            return Err(PostError::Forbidden);
        }

        sqlx::query("DELETE FROM related_posts WHERE post_id = ?")
            .bind(cmd.post_id)
            .execute(&mut *tx)
            .await?;

        for (order, slug) in cmd.related_post_slugs.iter().enumerate() {
            sqlx::query(
                "INSERT OR IGNORE INTO related_posts (post_id, related_post_id, display_order) \
                 SELECT ?, id, ? FROM posts WHERE slug = ? LIMIT 1",
            )
            .bind(cmd.post_id)
            .bind(order as i64)
            .bind(slug)
            .execute(&mut *tx)
            .await?;
        }

        tx.commit().await?;
        Ok(())
    }

    async fn set_post_featured(&self, cmd: SetFeaturedPostCommand) -> Result<(), PostError> {
        let is_featured_val = if cmd.is_featured { 1 } else { 0 };
        sqlx::query(
            r#"
            UPDATE posts
            SET is_featured = ?
            WHERE id = ?
            "#,
        )
        .bind(is_featured_val)
        .bind(cmd.post_id)
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    async fn update_post_cover(&self, cmd: UpdatePostCoverCommand) -> Result<(), PostError> {
        let mut set_fields: Vec<String> = vec![];
        if cmd.og_image_seconds.is_some() {
            set_fields.push("og_image_seconds = ?".to_string());
        }
        if !set_fields.is_empty() {
            let sql = format!(
                "UPDATE posts SET {} WHERE id = ? AND user_id = ?",
                set_fields.join(", ")
            );
            let mut query = sqlx::query(&sql);
            if let Some(seconds) = cmd.og_image_seconds {
                query = query.bind(seconds);
            }
            query = query.bind(cmd.post_id).bind(cmd.user_id);
            query.execute(&self.pool).await?;
        }
        Ok(())
    }
}

// Post link tables: tag resolution/linking and the media @-mention scan.
use std::collections::HashMap;

use once_cell::sync::Lazy;
use regex::Regex;

use crate::domain::errors::post::PostError;

pub(super) static MENTION_RE: Lazy<Regex> = Lazy::new(|| Regex::new(r"@([A-Za-z0-9_-]+)").unwrap());

/// Internal content link in a body: a site-relative or absolute URL whose path
/// is `/<kind>/<slug>`. The optional absolute prefix lets the same scan catch
/// both `[x](/posts/y)` and `[x](https://host/posts/y)`.
pub(super) static CONTENT_LINK_RE: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r#"(?:https?://[^\s)"'<>]+)?/(posts|projects|games|series)/([A-Za-z0-9_-]+)"#)
        .unwrap()
});

/// `:::app <type> <slug>` embed of another content row. `lottie` and
/// `glb-demo` are left out on purpose: they name media files, which the media
/// scan already registers.
pub(super) static APP_EMBED_RE: Lazy<Regex> =
    Lazy::new(|| Regex::new(r":::app\s+(project|html|v86|jsdos|game)\s+([A-Za-z0-9_-]+)").unwrap());

/// Usernames mentioned in a comment, deduplicated in first-seen order.
pub(crate) fn mentioned_usernames(content: &str) -> Vec<String> {
    let mut seen = std::collections::HashSet::new();
    MENTION_RE
        .captures_iter(content)
        .filter_map(|cap| cap.get(1).map(|m| m.as_str().to_string()))
        .filter(|name| seen.insert(name.clone()))
        .collect()
}

/// Upper bound on how many tags one post may carry. The tag SQL is built by
/// joining one placeholder per tag, so this also bounds the generated statement.
pub(super) const MAX_TAGS_PER_POST: usize = 30;

/// Validate, deduplicate, insert-if-missing, and resolve `tags` to tag ids.
///
/// Both `new_post` and `update_post` previously built their placeholder lists
/// from the *input* tag count while binding from the *resolved* rows. Any tag
/// that did not resolve 1:1 (a tag that did not exist yet, or a duplicate in the
/// input) produced a placeholder/bind mismatch and a sqlx error. Deriving the
/// placeholders from the resolved ids keeps the two counts in step by
/// construction.
pub(super) async fn resolve_tag_ids(
    tx: &mut sqlx::SqliteConnection,
    tags: &[String],
) -> Result<Vec<i64>, PostError> {
    if tags.is_empty() {
        return Ok(Vec::new());
    }
    if tags.len() > MAX_TAGS_PER_POST {
        return Err(PostError::Validation(format!(
            "A post may have at most {} tags.",
            MAX_TAGS_PER_POST
        )));
    }

    // Validate up front so a bad tag is a 400 rather than a trigger-raised 500.
    let mut unique = Vec::<String>::new();
    for tag in tags {
        let slug = crate::helper::string::validate_slug(tag).map_err(PostError::Validation)?;
        if !unique.contains(&slug) {
            unique.push(slug);
        }
    }

    let values = unique
        .iter()
        .map(|_| "(?, ?)")
        .collect::<Vec<_>>()
        .join(", ");
    let sql = format!("INSERT OR IGNORE INTO tags (slug, name) VALUES {}", values);
    let mut query = sqlx::query(&sql);
    for tag in &unique {
        query = query.bind(tag).bind(tag);
    }
    query.execute(&mut *tx).await?;

    let placeholder = unique.iter().map(|_| "?").collect::<Vec<_>>().join(", ");
    let sql = format!("SELECT id FROM tags WHERE slug IN ({})", placeholder);
    let mut query = sqlx::query_scalar::<_, i64>(&sql);
    for tag in &unique {
        query = query.bind(tag);
    }
    Ok(query.fetch_all(&mut *tx).await?)
}

/// Replace a post's tag links with `tag_ids`.
pub(super) async fn link_post_tags(
    tx: &mut sqlx::SqliteConnection,
    post_id: i64,
    tag_ids: &[i64],
) -> Result<(), PostError> {
    sqlx::query("DELETE FROM post_tags WHERE post_id = ?")
        .bind(post_id)
        .execute(&mut *tx)
        .await?;

    if tag_ids.is_empty() {
        return Ok(());
    }

    let values = tag_ids
        .iter()
        .map(|_| "(?, ?)")
        .collect::<Vec<_>>()
        .join(", ");
    let sql = format!("INSERT INTO post_tags (post_id, tag_id) VALUES {}", values);
    let mut query = sqlx::query(&sql);
    for tag_id in tag_ids {
        query = query.bind(post_id).bind(tag_id);
    }
    query.execute(&mut *tx).await?;
    Ok(())
}

/// Link a post to the media rows named by `media_usage` (short_name -> code).
///
/// Placeholders are derived from the rows that actually resolved, not from the
/// requested short names, so a short name with no matching media row is skipped
/// instead of desynchronising the bind list.
pub(super) async fn link_post_media(
    tx: &mut sqlx::SqliteConnection,
    post_id: i64,
    media_usage: &HashMap<String, i64>,
) -> Result<(), PostError> {
    if media_usage.is_empty() {
        return Ok(());
    }

    let placeholder = media_usage
        .iter()
        .map(|_| "?")
        .collect::<Vec<_>>()
        .join(", ");
    let sql = format!(
        "SELECT id, short_name FROM media WHERE short_name IN ({})",
        placeholder
    );
    let mut query = sqlx::query_as::<_, (i64, String)>(&sql);
    for short_name in media_usage.keys() {
        query = query.bind(short_name);
    }
    let media: Vec<(i64, String)> = query.fetch_all(&mut *tx).await?;

    let resolved: Vec<(i64, i64)> = media
        .into_iter()
        .filter_map(|(medium_id, short_name)| {
            media_usage.get(&short_name).map(|code| (medium_id, *code))
        })
        .collect();

    if resolved.is_empty() {
        return Ok(());
    }

    let values = resolved
        .iter()
        .map(|_| "(?, ?, ?)")
        .collect::<Vec<_>>()
        .join(", ");
    let sql = format!(
        "INSERT INTO post_media_usages (post_id, medium_id, code) VALUES {}",
        values
    );
    let mut query = sqlx::query(&sql);
    for (medium_id, code) in &resolved {
        query = query.bind(post_id).bind(medium_id).bind(code);
    }
    query.execute(&mut *tx).await?;
    Ok(())
}

/// Rebuild the `content_links` rows for one post body.
///
/// Media entries come from the resolved `post_media_usages` join rather than
/// from the `@[img:N]` tokens, so a token always maps to the id the renderer
/// uses. Content links are resolved by slug at save time and store a snapshot
/// of the target title and slug, which is what makes a reference survive a
/// later rename or delete.
pub(super) async fn sync_content_links(
    tx: &mut sqlx::SqliteConnection,
    post_id: i64,
    content: &str,
) -> Result<(), PostError> {
    sqlx::query("DELETE FROM content_links WHERE source_post_id = ? AND field = 'body'")
        .bind(post_id)
        .execute(&mut *tx)
        .await?;

    let media = sqlx::query_as::<_, (i64, String)>(
        r#"
        SELECT u.medium_id, m.short_name
        FROM post_media_usages u
        JOIN media m ON m.id = u.medium_id
        WHERE u.post_id = ?
        "#,
    )
    .bind(post_id)
    .fetch_all(&mut *tx)
    .await?;

    for (medium_id, short_name) in media {
        insert_content_link(
            tx,
            post_id,
            &format!("media:{medium_id}"),
            "media",
            medium_id,
            None,
            Some(&short_name),
        )
        .await?;
    }

    for cap in CONTENT_LINK_RE.captures_iter(content) {
        let path_kind = match &cap[1] {
            "posts" => "post",
            "projects" => "project",
            "games" => "game",
            _ => "series",
        };
        let slug = &cap[2];

        if path_kind == "series" {
            let series =
                sqlx::query_as::<_, (i64, String)>("SELECT id, title FROM series WHERE slug = ?")
                    .bind(slug)
                    .fetch_optional(&mut *tx)
                    .await?;
            if let Some((series_id, title)) = series {
                insert_content_link(
                    tx,
                    post_id,
                    &format!("series:{series_id}"),
                    "series",
                    series_id,
                    Some(&title),
                    Some(slug),
                )
                .await?;
            }
            continue;
        }

        link_post_target(tx, post_id, path_kind, slug).await?;
    }

    for cap in APP_EMBED_RE.captures_iter(content) {
        let target_kind = if &cap[1] == "game" { "game" } else { "project" };
        link_post_target(tx, post_id, target_kind, &cap[2]).await?;
    }

    Ok(())
}

/// Resolve a slug to the content row of `target_kind` and record the link. The
/// same `content:{id}` key is used for markdown links and app embeds, so a body
/// that both links and embeds one project stores a single row.
///
/// A slug is globally unique on `posts`, but the kind in the URL still has to
/// match the row: `/posts/x` must not link a project named `x`.
async fn link_post_target(
    tx: &mut sqlx::SqliteConnection,
    post_id: i64,
    target_kind: &str,
    slug: &str,
) -> Result<(), PostError> {
    let target = sqlx::query_as::<_, (i64, String)>(
        "SELECT id, title FROM posts WHERE slug = ? AND content_kind = ?",
    )
    .bind(slug)
    .bind(target_kind)
    .fetch_optional(&mut *tx)
    .await?;
    if let Some((target_id, title)) = target {
        insert_content_link(
            tx,
            post_id,
            &format!("content:{target_id}"),
            "content",
            target_id,
            Some(&title),
            Some(slug),
        )
        .await?;
    }
    Ok(())
}

#[allow(clippy::too_many_arguments)]
async fn insert_content_link(
    tx: &mut sqlx::SqliteConnection,
    source_post_id: i64,
    ref_key: &str,
    target_kind: &str,
    target_id: i64,
    target_title: Option<&str>,
    target_slug: Option<&str>,
) -> Result<(), PostError> {
    sqlx::query(
        r#"
        INSERT OR REPLACE INTO content_links
        (source_post_id, field, ref_key, target_kind, target_id, target_title, target_slug)
        VALUES (?, 'body', ?, ?, ?, ?, ?)
        "#,
    )
    .bind(source_post_id)
    .bind(ref_key)
    .bind(target_kind)
    .bind(target_id)
    .bind(target_title)
    .bind(target_slug)
    .execute(&mut *tx)
    .await?;
    Ok(())
}

/// Remember the slug an entity moved away from so a stale URL can be
/// redirected. The newest holder of a slug wins; a slug that is live again is
/// cleared, because redirect lookups check the live row first anyway.
pub(super) async fn record_slug_change(
    tx: &mut sqlx::SqliteConnection,
    entity_kind: &str,
    entity_id: i64,
    old_slug: &str,
    new_slug: &str,
) -> Result<(), PostError> {
    if old_slug == new_slug {
        return Ok(());
    }

    sqlx::query("DELETE FROM slug_history WHERE entity_kind = ? AND slug = ?")
        .bind(entity_kind)
        .bind(new_slug)
        .execute(&mut *tx)
        .await?;

    sqlx::query(
        r#"
        INSERT INTO slug_history (entity_kind, entity_id, slug)
        VALUES (?, ?, ?)
        ON CONFLICT(entity_kind, slug) DO UPDATE SET
            entity_id = excluded.entity_id,
            created_at = CURRENT_TIMESTAMP
        "#,
    )
    .bind(entity_kind)
    .bind(entity_id)
    .bind(old_slug)
    .execute(&mut *tx)
    .await?;

    Ok(())
}

// Post link tables: tag resolution/linking and the media @-mention scan.
use std::collections::HashMap;

use once_cell::sync::Lazy;
use regex::Regex;

use crate::domain::errors::post::PostError;

pub(super) static MENTION_RE: Lazy<Regex> = Lazy::new(|| Regex::new(r"@([A-Za-z0-9_-]+)").unwrap());

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

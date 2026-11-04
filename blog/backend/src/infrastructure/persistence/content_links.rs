// Link health for a published body. `content_links` stores the target slug as
// it was written, so comparing it with the live row is enough to tell a reader
// that a reference moved or that its target is gone.
use serde::Serialize;
use sqlx::SqlitePool;

#[derive(Debug, Clone, Serialize)]
pub struct LinkState {
    pub ref_key: String,
    /// `content` or `series`.
    pub target_kind: String,
    /// `post`, `project`, `game` or `series`.
    pub entity_kind: String,
    pub target_slug: Option<String>,
    pub current_slug: Option<String>,
    /// `ok`, `moved` or `missing`.
    pub status: String,
}

pub async fn health_for(
    pool: &SqlitePool,
    slug: &str,
) -> Result<Option<Vec<LinkState>>, sqlx::Error> {
    let source_id: Option<i64> = sqlx::query_scalar(
        "SELECT id FROM posts WHERE slug = ? AND status = 'published' AND deleted_at IS NULL",
    )
    .bind(slug)
    .fetch_optional(pool)
    .await?;
    let Some(source_id) = source_id else {
        return Ok(None);
    };

    let rows = sqlx::query_as::<_, (String, String, Option<i64>, Option<String>)>(
        r#"
        SELECT ref_key, target_kind, target_id, target_slug
        FROM content_links
        WHERE source_post_id = ? AND field = 'body'
          AND target_kind IN ('content', 'series')
        ORDER BY ref_key
        "#,
    )
    .bind(source_id)
    .fetch_all(pool)
    .await?;

    let mut links = Vec::with_capacity(rows.len());
    for (ref_key, target_kind, target_id, target_slug) in rows {
        let Some(target_id) = target_id else {
            continue;
        };

        let (entity_kind, current_slug) = if target_kind == "series" {
            let current = sqlx::query_scalar::<_, String>("SELECT slug FROM series WHERE id = ?")
                .bind(target_id)
                .fetch_optional(pool)
                .await?;
            ("series".to_string(), current)
        } else {
            let live = sqlx::query_as::<_, (String, String)>(
                r#"
                SELECT slug, content_kind FROM posts
                WHERE id = ? AND status = 'published' AND deleted_at IS NULL
                "#,
            )
            .bind(target_id)
            .fetch_optional(pool)
            .await?;
            match live {
                Some((current, kind)) => (kind, Some(current)),
                None => {
                    // A deleted post keeps its `content_kind`, which is what
                    // lets the frontend name the missing target.
                    let kind = sqlx::query_scalar::<_, String>(
                        "SELECT content_kind FROM posts WHERE id = ?",
                    )
                    .bind(target_id)
                    .fetch_optional(pool)
                    .await?;
                    (kind.unwrap_or_else(|| "post".to_string()), None)
                }
            }
        };

        let status = match (&current_slug, &target_slug) {
            (Some(current), Some(original)) if current != original => "moved",
            (Some(_), _) => "ok",
            _ => "missing",
        };

        links.push(LinkState {
            ref_key,
            target_kind,
            entity_kind,
            target_slug,
            current_slug,
            status: status.to_string(),
        });
    }

    Ok(Some(links))
}

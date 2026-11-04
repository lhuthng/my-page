// Slug redirect lookups. The live row always wins; `slug_history` is only
// consulted after a live lookup fails, which is how a re-used slug keeps
// serving the new entity instead of redirecting to the old one.
use serde::Serialize;
use sqlx::SqlitePool;

#[derive(Debug, Clone, Serialize)]
pub struct RedirectTarget {
    pub kind: String,
    pub slug: String,
}

pub async fn resolve(
    pool: &SqlitePool,
    kind: &str,
    slug: &str,
) -> Result<Option<RedirectTarget>, sqlx::Error> {
    match kind {
        "series" => resolve_series(pool, slug).await,
        "post" | "project" | "game" => resolve_post_kind(pool, kind, slug).await,
        _ => Ok(None),
    }
}

async fn resolve_series(
    pool: &SqlitePool,
    slug: &str,
) -> Result<Option<RedirectTarget>, sqlx::Error> {
    let live: Option<String> = sqlx::query_scalar("SELECT slug FROM series WHERE slug = ?")
        .bind(slug)
        .fetch_optional(pool)
        .await?;
    if let Some(slug) = live {
        return Ok(Some(RedirectTarget {
            kind: "series".to_string(),
            slug,
        }));
    }

    let moved: Option<String> = sqlx::query_scalar(
        r#"
        SELECT s.slug
        FROM slug_history h
        JOIN series s ON s.id = h.entity_id
        WHERE h.entity_kind = 'series' AND h.slug = ? AND s.slug <> h.slug
        "#,
    )
    .bind(slug)
    .fetch_optional(pool)
    .await?;

    Ok(moved.map(|slug| RedirectTarget {
        kind: "series".to_string(),
        slug,
    }))
}

async fn resolve_post_kind(
    pool: &SqlitePool,
    kind: &str,
    slug: &str,
) -> Result<Option<RedirectTarget>, sqlx::Error> {
    let live: Option<String> = sqlx::query_scalar(
        r#"
        SELECT slug FROM posts
        WHERE slug = ? AND content_kind = ?
          AND status = 'published' AND deleted_at IS NULL
        "#,
    )
    .bind(slug)
    .bind(kind)
    .fetch_optional(pool)
    .await?;
    if let Some(slug) = live {
        return Ok(Some(RedirectTarget {
            kind: kind.to_string(),
            slug,
        }));
    }

    let moved: Option<String> = sqlx::query_scalar(
        r#"
        SELECT p.slug
        FROM slug_history h
        JOIN posts p ON p.id = h.entity_id
        WHERE h.entity_kind = ? AND h.slug = ?
          AND p.content_kind = ?
          AND p.status = 'published' AND p.deleted_at IS NULL
          AND p.slug <> h.slug
        "#,
    )
    .bind(kind)
    .bind(slug)
    .bind(kind)
    .fetch_optional(pool)
    .await?;

    Ok(moved.map(|slug| RedirectTarget {
        kind: kind.to_string(),
        slug,
    }))
}

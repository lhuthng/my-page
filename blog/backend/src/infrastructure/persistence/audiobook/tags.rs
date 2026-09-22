// Audiobook tag replacement, loading, and listing.

use sqlx::{Sqlite, Transaction};

use crate::application::commands::audiobook::{
    CheckAudiobookSlugCommand, ListAudiobookTagsCommand,
};
use crate::domain::entities::audiobook::AudiobookTag;
use crate::domain::errors::audiobook::AudiobookError;

use super::AudiobookServiceImpl;
use super::validation::{MAX_TAG_NAME_CHARS, MAX_TAGS_PER_AUDIOBOOK};

impl AudiobookServiceImpl {
    pub(super) async fn replace_tags(
        tx: &mut Transaction<'_, Sqlite>,
        audiobook_id: i64,
        names: &[String],
    ) -> Result<(), AudiobookError> {
        sqlx::query("DELETE FROM audiobook_tag_links WHERE audiobook_id = ?")
            .bind(audiobook_id)
            .execute(&mut **tx)
            .await?;

        let mut seen: Vec<String> = Vec::new();
        for raw in names {
            let name = crate::helper::string::validate_text(raw, "Tag", MAX_TAG_NAME_CHARS)
                .map_err(AudiobookError::Validation)?;
            let slug = crate::helper::string::slugify(&name);
            if slug.is_empty() {
                return Err(AudiobookError::Validation(format!(
                    "Tag '{}' must contain at least one letter or number.",
                    name
                )));
            }
            // Duplicate tags in one payload would violate the link primary key.
            if seen.contains(&slug) {
                continue;
            }
            if seen.len() >= MAX_TAGS_PER_AUDIOBOOK {
                return Err(AudiobookError::Validation(format!(
                    "An audiobook may have at most {MAX_TAGS_PER_AUDIOBOOK} tags."
                )));
            }
            seen.push(slug.clone());

            // Reuse an existing tag row when the slug matches, otherwise create
            // it; `ON CONFLICT DO NOTHING` keeps a concurrent insert harmless.
            sqlx::query(
                "INSERT INTO audiobook_tags (name, slug) VALUES (?, ?)
                 ON CONFLICT(slug) DO NOTHING",
            )
            .bind(&name)
            .bind(&slug)
            .execute(&mut **tx)
            .await?;

            let tag_id: i64 = sqlx::query_scalar("SELECT id FROM audiobook_tags WHERE slug = ?")
                .bind(&slug)
                .fetch_one(&mut **tx)
                .await?;

            sqlx::query(
                "INSERT INTO audiobook_tag_links (audiobook_id, tag_id) VALUES (?, ?)
                 ON CONFLICT(audiobook_id, tag_id) DO NOTHING",
            )
            .bind(audiobook_id)
            .bind(tag_id)
            .execute(&mut **tx)
            .await?;
        }

        Ok(())
    }

    pub(super) async fn load_tags(
        tx: &mut Transaction<'_, Sqlite>,
        audiobook_id: i64,
    ) -> Result<Vec<AudiobookTag>, AudiobookError> {
        let rows: Vec<(i64, String, String, Option<String>)> = sqlx::query_as(
            r#"
            SELECT t.id, t.name, t.slug, t.description
            FROM audiobook_tag_links l
            JOIN audiobook_tags t ON t.id = l.tag_id
            WHERE l.audiobook_id = ?
            ORDER BY t.name COLLATE NOCASE ASC
            "#,
        )
        .bind(audiobook_id)
        .fetch_all(&mut **tx)
        .await?;

        Ok(rows
            .into_iter()
            .map(|(id, name, slug, description)| AudiobookTag {
                id,
                name,
                slug,
                description,
                // Per-audiobook reads do not need the global usage count; the
                // tag manager fills it in.
                audiobook_count: 0,
            })
            .collect())
    }

    pub(super) async fn list_audiobook_tags(
        &self,
        cmd: ListAudiobookTagsCommand,
    ) -> Result<Vec<AudiobookTag>, AudiobookError> {
        let rows: Vec<(i64, String, String, Option<String>, i64)> = if cmd.is_admin {
            sqlx::query_as(
                r#"
                SELECT t.id, t.name, t.slug, t.description,
                       (SELECT COUNT(*) FROM audiobook_tag_links l
                        WHERE l.tag_id = t.id)
                FROM audiobook_tags t
                ORDER BY t.name COLLATE NOCASE ASC
                LIMIT ? OFFSET ?
                "#,
            )
            .bind(cmd.limit)
            .bind(cmd.offset)
            .fetch_all(&self.pool)
            .await?
        } else {
            sqlx::query_as(
                r#"
                SELECT t.id, t.name, t.slug, t.description,
                       (SELECT COUNT(*) FROM audiobook_tag_links l
                        JOIN audiobooks a ON a.id = l.audiobook_id
                        WHERE l.tag_id = t.id AND a.user_id = ?)
                FROM audiobook_tags t
                WHERE EXISTS (
                    SELECT 1 FROM audiobook_tag_links l
                    JOIN audiobooks a ON a.id = l.audiobook_id
                    WHERE l.tag_id = t.id AND a.user_id = ?)
                ORDER BY t.name COLLATE NOCASE ASC
                LIMIT ? OFFSET ?
                "#,
            )
            .bind(cmd.user_id)
            .bind(cmd.user_id)
            .bind(cmd.limit)
            .bind(cmd.offset)
            .fetch_all(&self.pool)
            .await?
        };

        Ok(rows
            .into_iter()
            .map(
                |(id, name, slug, description, audiobook_count)| AudiobookTag {
                    id,
                    name,
                    slug,
                    description,
                    audiobook_count,
                },
            )
            .collect())
    }

    pub(super) async fn check_audiobook_slug(
        &self,
        cmd: CheckAudiobookSlugCommand,
    ) -> Result<bool, AudiobookError> {
        let slug =
            crate::helper::string::validate_slug(&cmd.slug).map_err(AudiobookError::Validation)?;

        let taken: bool =
            sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM audiobooks WHERE slug = ?)")
                .bind(&slug)
                .fetch_one(&self.pool)
                .await?;

        // `true` means the slug is free.
        Ok(!taken)
    }
}

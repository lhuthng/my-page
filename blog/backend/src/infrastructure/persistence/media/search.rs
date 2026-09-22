// Media search over short names, file names, and aliases.

use crate::application::commands::media::SearchMediaCommand;
use crate::domain::entities::media::LinkResult;
use crate::domain::errors::media::MediaError;

use super::MediaServiceImpl;
use super::rows::MediaSearchRow;

impl MediaServiceImpl {
    pub(super) async fn search(
        &self,
        cmd: SearchMediaCommand,
    ) -> Result<Vec<LinkResult>, MediaError> {
        let rows = sqlx::query_as::<_, MediaSearchRow>(
            r#"
            SELECT DISTINCT
                m.short_name,
                'media/i/' || m.short_name AS url,
                m.file_type,
                m.hash,
                m.uploader_id,
                CASE
                    WHEN LOWER(m.short_name) = LOWER(?1) THEN 3
                    WHEN LOWER(m.short_name) LIKE LOWER(?1) || '%' THEN 2
                    WHEN LOWER(m.short_name) LIKE '%' || LOWER(?1) || '%' THEN 1
                    ELSE 0
                END AS score
            FROM media AS m
            LEFT JOIN media_aliases AS ma ON ma.media_id = m.id
            WHERE
                LOWER(m.short_name) LIKE '%' || LOWER(?1) || '%'
                OR LOWER(COALESCE(ma.alias, '')) LIKE '%' || LOWER(?1) || '%'
            ORDER BY score DESC, m.created_at DESC
            LIMIT ?2 OFFSET ?3;
            "#,
        )
        .bind(&cmd.term)
        .bind(cmd.size)
        .bind(cmd.skip)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| MediaError::InternalError(e.to_string()))?;

        Ok(rows
            .into_iter()
            .map(|r| LinkResult {
                short_name: Some(r.short_name),
                url: r.url,
                file_type: r.file_type,
                hash: r.hash,
                uploader_id: r.uploader_id,
            })
            .collect())
    }
}

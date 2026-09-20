// Ownership and role guards shared by the game, project, and v86 handlers.
// The aggregate's pool and table are parameters; error construction is
// injected so each caller keeps its own error type.
use crate::domain::{
    entities::secret::Claims,
    errors::project::ProjectError,
};

pub fn is_admin_or_mod(role: &str) -> bool {
    role == "admin" || role == "moderator"
}

/// Require that `user_id` owns the row in `table` (joined to its post for the
/// author). Both original copies returned `ProjectError`, which callers
/// convert through their own `From` impl.
pub async fn require_owner(
    pool: &sqlx::SqlitePool,
    table: &str,
    id: i64,
    user_id: i64,
) -> Result<(), ProjectError> {
    let owner: Option<i64> = sqlx::query_scalar(&format!(
        "SELECT posts.user_id FROM {table} JOIN posts ON posts.id = {table}.post_id WHERE {table}.id = ?"
    ))
    .bind(id)
    .fetch_optional(pool)
    .await?;
    match owner {
        Some(id) if id == user_id => Ok(()),
        Some(_) => Err(ProjectError::Forbidden),
        None => Err(ProjectError::ProjectNotFound),
    }
}

/// Resolve the post id of the aggregate row, requiring that `claims` owns it
/// or is admin/moderator. Returns the post id for the soft-delete paths.
pub async fn require_can_delete<E>(
    pool: &sqlx::SqlitePool,
    table: &str,
    id: i64,
    claims: &Claims,
    internal_error: impl Fn(String) -> E,
    not_found_error: impl Fn() -> E,
    forbidden_error: impl Fn() -> E,
) -> Result<i64, E> {
    let user_id = claims
        .user_id
        .parse::<i64>()
        .map_err(|_| internal_error("Cannot parse id".to_string()))?;
    let owner: Option<i64> = sqlx::query_scalar(&format!(
        "SELECT posts.user_id FROM {table} JOIN posts ON posts.id = {table}.post_id WHERE {table}.id = ?"
    ))
    .bind(id)
    .fetch_optional(pool)
    .await?;
    let owner = owner.ok_or_else(not_found_error)?;
    if owner == user_id || is_admin_or_mod(&claims.role) {
        // need post_id for soft-delete; fetch it
        let post_id: Option<i64> =
            sqlx::query_scalar(&format!("SELECT post_id FROM {table} WHERE id=?"))
                .bind(id)
                .fetch_optional(pool)
                .await?;
        post_id.ok_or_else(not_found_error)
    } else {
        Err(forbidden_error())
    }
}

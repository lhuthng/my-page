use sqlx::{SqlitePool, prelude::FromRow};

use crate::{
    application::{commands::user::MeCommand, services::user::UserService},
    domain::{entities::user::Me, errors::user::UserError},
};

#[derive(FromRow, Debug)]
struct MeRow {
    display_name: String,
}

pub struct UserServiceImpl {
    pub pool: SqlitePool,
}

impl UserServiceImpl {
    pub fn new(pool: SqlitePool) -> Self {
        Self { pool }
    }
}

#[async_trait::async_trait]
impl UserService for UserServiceImpl {
    async fn me(&self, cmd: MeCommand) -> Result<Me, UserError> {
        let me_row = sqlx::query_as::<_, MeRow>(
            r#"
			SELECT display_name
			FROM user_meta
			WHERE user_id = ?
			"#,
        )
        .bind(&cmd.user_id)
        .fetch_one(&self.pool)
        .await
        .map_err(|_| UserError::NotFound)?;

        Ok(Me {
            display_name: me_row.display_name,
        })
    }
}

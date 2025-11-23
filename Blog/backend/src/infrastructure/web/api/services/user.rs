use sqlx::{SqlitePool, prelude::FromRow};

use crate::{
    application::{
        commands::user::{GetUserCommand, MeCommand},
        services::user::UserService,
    },
    domain::{
        entities::user::{Me, User},
        errors::user::UserError,
    },
};

pub struct UserServiceImpl {
    pub pool: SqlitePool,
}

impl UserServiceImpl {
    pub fn new(pool: SqlitePool) -> Self {
        Self { pool }
    }
}

#[derive(FromRow, Debug)]
struct MeRow {
    username: String,
    display_name: String,
    role: String,
}

#[derive(FromRow, Debug)]
struct UserRow {
    username: String,
    display_name: String,
    bio: String,
    role: String,
}

#[async_trait::async_trait]
impl UserService for UserServiceImpl {
    async fn me(&self, cmd: MeCommand) -> Result<Me, UserError> {
        let me_row = sqlx::query_as::<_, MeRow>(
            r#"
			SELECT username, display_name, role
			FROM user_meta JOIN users ON users.id = user_id
			WHERE user_id = ?
			"#,
        )
        .bind(&cmd.user_id)
        .fetch_one(&self.pool)
        .await
        .map_err(|_| UserError::NotFound)?;

        Ok(Me {
            username: me_row.username,
            display_name: me_row.display_name,
            role: me_row.role,
        })
    }
    async fn get_user(&self, cmd: GetUserCommand) -> Result<User, UserError> {
        let user_row = sqlx::query_as::<_, UserRow>(
            r#"
            SELECT username, display_name, bio, role
            FROM users
            JOIN user_meta ON id = user_id
            WHERE username = ?
            "#,
        )
        .bind(&cmd.username)
        .fetch_one(&self.pool)
        .await
        .map_err(|_| UserError::NotFound)?;

        Ok(User {
            username: user_row.username,
            display_name: user_row.display_name,
            bio: user_row.bio,
            role: user_row.role,
        })
    }
}

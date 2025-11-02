use crate::domain::errors::auth::AuthError;

use super::value_objects::{Email, HashedPassword};

#[derive(Debug, Clone)]
pub enum UserRole {
    Admin,
    Moderator,
    User,
}

impl TryFrom<String> for UserRole {
    type Error = AuthError;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        match value.as_str() {
            "user" => Ok(UserRole::User),
            "admin" => Ok(UserRole::Admin),
            "moderator" => Ok(UserRole::Moderator),
            _ => Err(AuthError::InternalError(format!("Unknown role: {}", value))),
        }
    }
}

#[derive(Debug, Clone)]
pub struct User {
    pub id: i64,
    pub username: String,
    pub email: Email,
    pub hashed_password: HashedPassword,
    pub role: UserRole,
    pub display_name: String,
    pub bio: String,
}

use crate::domain::errors::{auth::AuthError, user::UserError};

// use super::value_objects::{Email, HashedPassword};

#[derive(Debug, Clone)]
pub enum UserRole {
    Admin,
    Moderator,
    User,
}

impl UserRole {
    fn rank(&self) -> i8 {
        match self {
            UserRole::Admin => 0,
            UserRole::Moderator => 1,
            UserRole::User => 2,
        }
    }
    pub fn include(&self, other: &Self) -> bool {
        return self.rank() >= other.rank();
    }
}

impl TryFrom<String> for UserRole {
    type Error = UserError;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        match value.as_str() {
            "user" => Ok(UserRole::User),
            "admin" => Ok(UserRole::Admin),
            "moderator" => Ok(UserRole::Moderator),
            _ => Err(UserError::InternalError(format!("Unknown role: {}", value))),
        }
    }
}

#[derive(Debug, Clone)]
pub struct Me {
    pub username: String,
    pub display_name: String,
    pub role: String,
}

#[derive(Debug, Clone)]
pub struct User {
    pub username: String,
    pub display_name: String,
    pub bio: String,
    pub role: String,
}

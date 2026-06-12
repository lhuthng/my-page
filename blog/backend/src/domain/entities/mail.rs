use serde::Deserialize;
use validator::Validate;

#[derive(Debug, Clone, Deserialize, Validate)]
pub struct ContactFormCredential {
    #[validate(length(
        min = 3,
        max = 48,
        message = "Name must have at least 3 characters and at most 48 characters"
    ))]
    pub name: String,

    #[validate(email(message = "Invalid email address"))]
    pub email: String,

    #[validate(length(
        min = 5,
        max = 1500,
        message = "Message must have at least 5 characters and at most 1500 characters"
    ))]
    pub content: String,
}

use serde::Deserialize;
use validator::Validate;

#[derive(Debug, Deserialize, Validate)]
pub struct ContactFormCredential {
    #[validate(length(
        min = 3,
        max = 16,
        message = "Name must have at least 3 characters and at most 16 characters"
    ))]
    pub name: String,

    #[validate(email(message = "Invalid email address"))]
    pub email: String,

    #[validate(length(
        min = 5,
        max = 512,
        message = "Name must have at least 5 characters and at most 512 characters"
    ))]
    pub content: String,
}

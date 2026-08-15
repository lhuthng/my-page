use serde::Serialize;
use validator::Validate;

#[derive(Debug, Clone, serde::Deserialize, Validate)]
pub struct SubscribeRequest {
    #[validate(email(message = "Invalid email address"))]
    pub email: String,
}

#[derive(Debug, Clone)]
pub struct ConfirmSubscriptionMailPayload {
    pub email: String,
    pub token: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct SubscriberSnapshot {
    pub id: i64,
    pub email: String,
    pub status: String,
    pub created_at: String,
    pub confirmed_at: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct CampaignSnapshot {
    pub id: i64,
    pub post_id: Option<i64>,
    pub subject: String,
    pub recipient_count: i64,
    pub success_count: i64,
    pub failure_count: i64,
    pub started_at: String,
    pub completed_at: Option<String>,
}

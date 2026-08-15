#[derive(serde::Deserialize)]
pub struct SubscribeCommand {
    pub email: String,
}

#[derive(serde::Deserialize)]
pub struct ConfirmSubscriptionCommand {
    pub token: String,
}

#[derive(serde::Deserialize)]
pub struct UnsubscribeCommand {
    pub token: String,
}

#[derive(serde::Deserialize)]
pub struct UnsubscribeByEmailCommand {
    pub email: String,
}

pub struct ListSubscribersCommand {
    pub is_admin: bool,
}

#[derive(serde::Deserialize)]
pub struct SendCampaignCommand {
    pub post_id: Option<i64>,
    pub subject: String,
    pub body_html: String,
    pub body_text: String,
    #[serde(skip)]
    pub sent_by_user_id: i64,
}

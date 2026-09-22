mod brevo;
mod messages;
pub(crate) mod smtp;
mod templates;

pub use messages::contact::send_contact_emails;
pub use messages::newsletter::{send_campaign_email, send_subscription_confirm_email};
pub use messages::password_reset::send_password_reset_email;
pub use messages::verification::send_verification_email;
#[cfg(debug_assertions)]
pub use templates::{CampaignPostData, preview_page};
pub use templates::{campaign_post_body, campaign_post_text};

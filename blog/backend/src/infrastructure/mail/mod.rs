use lettre::{
    Message, SmtpTransport, Transport,
    message::{
        Mailbox, MultiPart, SinglePart,
        header::{HeaderName, HeaderValue},
    },
    transport::smtp::authentication::Credentials,
};
use reqwest::Client;
use serde::Serialize;
use tokio::task;

use crate::{
    domain::entities::{
        auth::{PasswordResetMailPayload, VerificationMailPayload},
        mail::ContactFormCredential,
        newsletter::ConfirmSubscriptionMailPayload,
    },
    infrastructure::web::server::{MailConfig, MailTransportConfig},
};

mod brevo;
mod messages;
pub(crate) mod smtp;
mod templates;

pub use messages::contact::send_contact_emails;
pub use messages::newsletter::{send_campaign_email, send_subscription_confirm_email};
pub use messages::password_reset::send_password_reset_email;
pub use messages::verification::send_verification_email;

// Transport health check: proves the configured transport is reachable and
// authenticated WITHOUT sending any message.
use crate::infrastructure::web::server::{MailConfig, MailTransportConfig};

use super::smtp::build_smtp_mailer;

/// Probes the configured transport without sending anything: SMTP gets an
/// EHLO handshake, Brevo an authenticated account lookup. Returns a short
/// human-readable description of what was verified.
pub async fn check_mail_transport(mail_config: &MailConfig) -> Result<String, String> {
    match &mail_config.transport {
        MailTransportConfig::Smtp { host, port, .. } => {
            let mailer = build_smtp_mailer(mail_config)?;
            let host = host.clone();
            let port = *port;

            tokio::task::spawn_blocking(move || {
                mailer
                    .test_connection()
                    .map_err(|err| format!("{host}:{port} refused the handshake: {err}"))?;
                Ok(format!(
                    "{host}:{port} is ready to accept mail (EHLO handshake ok)"
                ))
            })
            .await
            .map_err(|err| format!("Handshake task failed: {err}"))?
        }
        MailTransportConfig::BrevoApi { api_key } => {
            let client = reqwest::Client::builder()
                .timeout(std::time::Duration::from_secs(10))
                .build()
                .map_err(|err| format!("Failed to build Brevo client: {err}"))?;

            // The account endpoint is the cheapest authenticated call Brevo
            // offers; a bad key shows up as 401 before anything is ever sent.
            let response = client
                .get("https://api.brevo.com/v3/account")
                .header("api-key", api_key)
                .header("accept", "application/json")
                .send()
                .await
                .map_err(|err| format!("Brevo API unreachable: {err}"))?;

            if response.status().is_success() {
                Ok("Brevo API accepted the key (account lookup ok)".to_string())
            } else {
                let status = response.status();
                let body = response
                    .text()
                    .await
                    .unwrap_or_else(|_| "Unable to read Brevo error response".to_string());
                Err(format!("Brevo API rejected the key ({status}): {body}"))
            }
        }
    }
}

//! Verifies the configured mail transport without sending any message:
//! SMTP gets an EHLO handshake, the Brevo API an authenticated account
//! lookup. Exits 0 when the transport is ready to deliver, 1 when it is
//! not, so deploys and shell checks can gate on it.
//!
//! On the VM the backend container carries the mail env (compose
//! `env_file`), so run it there with:
//!   docker compose exec -T backend check-mail
use std::process::ExitCode;

use backend::infrastructure::mail::check_mail_transport;
use backend::infrastructure::web::server::MailConfig;

#[tokio::main]
async fn main() -> ExitCode {
    // Mirrors the server binary so a local `cargo run --bin check-mail`
    // reads the same backend/.env.
    dotenvy::dotenv().ok();

    let Some(mail_config) = MailConfig::from_env() else {
        eprintln!(
            "Mail is not configured (set BREVO_API_KEY, or SMTP_HOST/SMTP_PORT/SMTP_USERNAME/\
             SMTP_PASSWORD/SMTP_FROM); there is no transport to check."
        );
        return ExitCode::FAILURE;
    };

    match check_mail_transport(&mail_config).await {
        Ok(detail) => {
            println!("Mail transport ready, nothing was sent: {detail}");
            ExitCode::SUCCESS
        }
        Err(err) => {
            eprintln!("Mail transport check FAILED, nothing was sent: {err}");
            ExitCode::FAILURE
        }
    }
}

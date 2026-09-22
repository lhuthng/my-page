// Mail transport configuration: Brevo API or SMTP, selected from the env.
use std::env;

#[derive(Clone)]
pub struct MailConfig {
    pub transport: MailTransportConfig,
    pub from: String,
    pub to: String,
}

#[derive(Clone)]
pub enum MailTransportConfig {
    BrevoApi {
        api_key: String,
    },
    Smtp {
        host: String,
        port: u16,
        username: String,
        password: String,
    },
}

impl MailConfig {
    pub fn from_env() -> Option<MailConfig> {
        let brevo_api_key = env::var("BREVO_API_KEY").ok();
        let host = env::var("SMTP_HOST").ok();
        let port = env::var("SMTP_PORT").ok();
        let username = env::var("SMTP_USERNAME").ok();
        let password = env::var("SMTP_PASSWORD").ok();
        let from = env::var("SMTP_FROM").ok();
        let to = env::var("SMTP_TO").ok();

        let any_set = [
            &brevo_api_key,
            &host,
            &port,
            &username,
            &password,
            &from,
            &to,
        ]
        .into_iter()
        .any(|value| value.is_some());

        if !any_set {
            return None;
        }

        let from = from.expect("SMTP_FROM must be set when mail is enabled");
        let to = to.expect("SMTP_TO must be set when mail is enabled");

        if let Some(api_key) = brevo_api_key {
            return Some(MailConfig {
                transport: MailTransportConfig::BrevoApi { api_key },
                from,
                to,
            });
        }

        Some(MailConfig {
            transport: MailTransportConfig::Smtp {
                host: host.expect("SMTP_HOST must be set when SMTP is enabled"),
                port: port
                    .expect("SMTP_PORT must be set when SMTP is enabled")
                    .parse::<u16>()
                    .expect("SMTP_PORT must be a valid port number"),
                username: username.expect("SMTP_USERNAME must be set when SMTP is enabled"),
                password: password.expect("SMTP_PASSWORD must be set when SMTP is enabled"),
            },
            from,
            to,
        })
    }
}

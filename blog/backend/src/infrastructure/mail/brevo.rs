// Brevo API transport: payload shapes and the HTTP call.
use reqwest::Client;
use serde::Serialize;

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub(super) struct BrevoAddress<'a> {
    pub(super) email: &'a str,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(super) name: Option<&'a str>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub(super) struct BrevoEmailPayload<'a> {
    pub(super) sender: BrevoAddress<'a>,
    pub(super) to: Vec<BrevoAddress<'a>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(super) reply_to: Option<BrevoAddress<'a>>,
    pub(super) subject: &'a str,
    pub(super) text_content: &'a str,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(super) html_content: Option<&'a str>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(super) headers: Option<std::collections::HashMap<&'a str, &'a str>>,
}

pub(super) async fn send_brevo_email(
    api_key: &str,
    payload: &BrevoEmailPayload<'_>,
) -> Result<(), String> {
    let client = Client::builder()
        .timeout(std::time::Duration::from_secs(10))
        .build()
        .map_err(|err| format!("Failed to build Brevo client: {err}"))?;

    let response = client
        .post("https://api.brevo.com/v3/smtp/email")
        .header("api-key", api_key)
        .header("accept", "application/json")
        .json(payload)
        .send()
        .await
        .map_err(|err| format!("Brevo API request failed: {err}"))?;

    if response.status().is_success() {
        Ok(())
    } else {
        let status = response.status();
        let body = response
            .text()
            .await
            .unwrap_or_else(|_| "Unable to read Brevo error response".to_string());
        Err(format!("Brevo API returned {status}: {body}"))
    }
}

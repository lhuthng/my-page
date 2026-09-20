// Brevo API transport: payload shapes and the HTTP call.
use serde::Serialize;

pub(super) struct BrevoAddress<'a> {
    email: &'a str,
    #[serde(skip_serializing_if = "Option::is_none")]
    name: Option<&'a str>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub(super) struct BrevoEmailPayload<'a> {
    sender: BrevoAddress<'a>,
    to: Vec<BrevoAddress<'a>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    reply_to: Option<BrevoAddress<'a>>,
    subject: &'a str,
    text_content: &'a str,
    #[serde(skip_serializing_if = "Option::is_none")]
    html_content: Option<&'a str>,
    #[serde(skip_serializing_if = "Option::is_none")]
    headers: Option<std::collections::HashMap<&'a str, &'a str>>,
}


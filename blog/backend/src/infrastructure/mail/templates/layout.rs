/// Branded HTML email templates.
///
/// Email clients strip `<style>` blocks and don't support flexbox/grid, so
/// every email is rendered as a table-based layout with inline styles only.
/// The design is a light theme (white card on light gray) with the site's
/// navy/lavender accents, matching the huuthangle.site brand.
/// Shared chrome for every email: colors, buttons, and the page shell.
use super::{
    BACKGROUND, CARD, DIVIDER, FONT_STACK, MUTED, NAVY, PRIMARY, TEXT, WHITE, escape_html,
};

#[cfg(debug_assertions)]
pub(super) fn logo_data_uri() -> String {
    use base64::{Engine, engine::general_purpose};
    const LOGO: &[u8] = include_bytes!("../logo.png");
    format!(
        "data:image/png;base64,{}",
        general_purpose::STANDARD.encode(LOGO)
    )
}

/// Table-based call-to-action button.
pub(super) fn render_button(label: &str, href: &str) -> String {
    format!(
        r#"<table role="presentation" cellpadding="0" cellspacing="0" border="0"><tr><td style="border-radius:8px;background-color:{PRIMARY};"><a href="{href}" style="display:inline-block;padding:12px 28px;font-family:{FONT_STACK};font-size:16px;font-weight:600;color:{WHITE};text-decoration:none;border-radius:8px;">{label}</a></td></tr></table>"#,
        href = escape_html(href),
        label = escape_html(label),
    )
}

/// Wraps `inner_html` in the full branded email document (preheader, logo
/// header, content card, footer).
pub fn render_shell(
    app_base_url: &str,
    preheader: &str,
    inner_html: &str,
    footer_html: &str,
) -> String {
    let logo_url = escape_html(&format!("{}/logo.png", app_base_url.trim_end_matches('/')));
    let preheader = escape_html(preheader);
    let spacer = "&nbsp;&nbsp;&nbsp;&nbsp;&nbsp;&nbsp;&nbsp;&nbsp;&nbsp;&nbsp;&nbsp;&nbsp;&nbsp;&nbsp;&nbsp;&nbsp;&nbsp;&nbsp;&nbsp;&nbsp;&nbsp;&nbsp;&nbsp;&nbsp;&nbsp;&nbsp;&nbsp;&nbsp;&nbsp;&nbsp;";

    format!(
        r#"<!DOCTYPE html>
<html lang="en" xmlns="http://www.w3.org/1999/xhtml">
<head>
<meta charset="utf-8">
<meta name="viewport" content="width=device-width, initial-scale=1">
<meta name="color-scheme" content="light">
<meta name="supported-color-schemes" content="light">
<meta name="x-apple-disable-message-reformatting">
<title>huuthangle.site</title>
</head>
<body style="margin:0;padding:0;background-color:{BACKGROUND};word-spacing:normal;font-family:{FONT_STACK};">
<div role="presentation" style="display:none;font-size:1px;color:{BACKGROUND};line-height:1px;max-height:0;max-width:0;opacity:0;overflow:hidden;mso-hide:all;">{preheader}{spacer}</div>
<table role="presentation" width="100%" cellpadding="0" cellspacing="0" border="0" style="background-color:{BACKGROUND};">
<tr>
<td align="center" style="padding:28px 12px;">
<div style="max-width:600px;margin:0 auto;border-radius:14px;overflow:hidden;box-shadow:0 4px 12px rgba(0,0,0,0.15);">
<table role="presentation" width="100%" cellpadding="0" cellspacing="0" border="0">
<tr>
<td style="background-color:{NAVY};padding:20px 28px;">
<table role="presentation" cellpadding="0" cellspacing="0" border="0">
<tr>
<td style="vertical-align:middle;"><img src="{logo_url}" width="auto" height="36" alt="huuthangle.site" style="display:block;border:0;border-radius:8px;"></td>
<td style="vertical-align:middle;padding-left:12px;"><span style="color:{WHITE};font-family:{FONT_STACK};font-size:18px;font-weight:700;letter-spacing:0.2px;">HuuThangLe.site</span></td>
</tr>
</table>
</td>
</tr>
<tr>
<td style="background-color:{CARD};padding:32px 28px;color:{TEXT};font-family:{FONT_STACK};font-size:16px;line-height:1.6;">
{inner_html}
</td>
</tr>
<tr>
<td style="background-color:{CARD};border-top:1px solid {DIVIDER};padding:20px 28px;color:{MUTED};font-family:{FONT_STACK};font-size:12px;line-height:1.6;text-align:center;">
{footer_html}
</td>
</tr>
</table>
</div>
</td>
</tr>
</table>
</body>
</html>"#,
        logo_url = logo_url,
        preheader = preheader,
        spacer = spacer,
        inner_html = inner_html,
        footer_html = footer_html,
    )
}

/// Standard footer used by account/newsletter emails that do not carry an
/// unsubscribe link (confirmation, verification, password reset).
pub(super) fn site_footer(app_base_url: &str, note: &str) -> String {
    let site = escape_html(&format!("{}/", app_base_url.trim_end_matches('/')));
    format!(
        r#"<a href="{site}" style="color:{PRIMARY};text-decoration:none;">huuthangle.site</a> · {note}"#,
        site = site,
    )
}

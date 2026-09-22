use super::layout::{render_button, render_shell, site_footer};
/// Branded HTML email templates.
///
/// Email clients strip `<style>` blocks and don't support flexbox/grid, so
/// every email is rendered as a table-based layout with inline styles only.
/// The design is a light theme (white card on light gray) with the site's
/// navy/lavender accents, matching the huuthangle.site brand.
/// Verification and password-reset email bodies.
use super::{MUTED, TEXT, escape_html};

pub fn verification(app_base_url: &str, username: &str, verify_link: &str) -> (String, String) {
    let text = format!(
        "Hey {username}! (＾▽＾)\n\nLet's get you verified - open the link below within 30 minutes:\n\n{verify_link}\n\nIf you didn't create this account, you can ignore this email. (・_・;)\n",
    );

    let inner = format!(
        r#"<h1 style="margin:0 0 16px;font-size:24px;font-weight:700;color:{TEXT};line-height:1.3;">Verify your account (ง •̀_•́)ง</h1>
<p style="margin:0 0 20px;">Hey {username}, welcome aboard! One tiny click below within <strong>30 minutes</strong> and your account is good to go.</p>
{button}
<p style="margin:24px 0 0;font-size:14px;color:{MUTED};">If you didn't create this account, you can ignore this email. (´・ω・`)</p>"#,
        username = escape_html(username),
        button = render_button("Verify email", verify_link),
    );

    let footer = site_footer(
        app_base_url,
        "You received this email because this address was used to create an account.",
    );
    let html = render_shell(
        app_base_url,
        "Verify your HuuThangLe account",
        &inner,
        &footer,
    );
    (text, html)
}

/// Password reset email.
pub fn password_reset(app_base_url: &str, username: &str, reset_link: &str) -> (String, String) {
    let text = format!(
        "Hey {username}! (・∀・)\n\nForgot your password? It happens to the best of us. Use the link below within 30 minutes to set a new one:\n\n{reset_link}\n\nIf you didn't request this, just ignore this email - your password stays exactly as it is. (￣▽￣)\n",
    );

    let inner = format!(
        r#"<h1 style="margin:0 0 16px;font-size:24px;font-weight:700;color:{TEXT};line-height:1.3;">Let's fix that password (｀・ω・´)</h1>
<p style="margin:0 0 20px;">Hey {username}, we got a request to reset the password for your huuthangle.site account. Click the button below within <strong>30 minutes</strong> to pick a shiny new one.</p>
{button}
<p style="margin:24px 0 0;font-size:14px;color:{MUTED};">If you didn't request this, ignore this email - your password stays the same. (´-ω-`)</p>"#,
        username = escape_html(username),
        button = render_button("Reset password", reset_link),
    );

    let footer = site_footer(
        app_base_url,
        "You received this email because a password reset was requested for this address.",
    );
    let html = render_shell(
        app_base_url,
        "Reset your HuuThangLe password",
        &inner,
        &footer,
    );
    (text, html)
}

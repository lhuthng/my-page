/// Branded HTML email templates.
///
/// Email clients strip `<style>` blocks and don't support flexbox/grid, so
/// every email is rendered as a table-based layout with inline styles only.
/// The design is a light theme (white card on light gray) with the site's
/// navy/lavender accents, matching the huuthangle.site brand.
pub const FONT_STACK: &str = "-apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, Helvetica, Arial, sans-serif";

const BACKGROUND: &str = "#eef0f6";
const CARD: &str = "#ffffff";
const NAVY: &str = "#495c83";
const PRIMARY: &str = "#7a86b6";
const TEXT: &str = "#333a4d";
const MUTED: &str = "#6b7280";
const DIVIDER: &str = "#e5e7eb";
const WHITE: &str = "#ffffff";

/// Escapes a plain-text value for safe inclusion in an HTML attribute or body.
pub fn escape_html(value: &str) -> String {
    value
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&#39;")
}

/// Dev-only: embeds `logo.png` (a copy of `frontend/static/logo.png`) as a
/// base64 data URI so the preview page shows the logo even when the frontend
/// origin isn't reachable. Real emails keep the hosted `{app_base_url}/logo.png`.
#[cfg(debug_assertions)]
fn logo_data_uri() -> String {
    use base64::{Engine, engine::general_purpose};
    const LOGO: &[u8] = include_bytes!("logo.png");
    format!("data:image/png;base64,{}", general_purpose::STANDARD.encode(LOGO))
}

/// Table-based call-to-action button.
fn render_button(label: &str, href: &str) -> String {
    format!(
        r#"<table role="presentation" cellpadding="0" cellspacing="0" border="0"><tr><td style="border-radius:8px;background-color:{PRIMARY};"><a href="{href}" style="display:inline-block;padding:12px 28px;font-family:{FONT_STACK};font-size:16px;font-weight:600;color:{WHITE};text-decoration:none;border-radius:8px;">{label}</a></td></tr></table>"#,
        href = escape_html(href),
        label = escape_html(label),
    )
}

/// Wraps `inner_html` in the full branded email document (preheader, logo
/// header, content card, footer).
pub fn render_shell(app_base_url: &str, preheader: &str, inner_html: &str, footer_html: &str) -> String {
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
fn site_footer(app_base_url: &str, note: &str) -> String {
    let site = escape_html(&format!("{}/", app_base_url.trim_end_matches('/')));
    format!(
        r#"<a href="{site}" style="color:{PRIMARY};text-decoration:none;">huuthangle.site</a> · {note}"#,
        site = site,
    )
}

/// Subscription confirmation email (newsletter).
pub fn subscription_confirm(app_base_url: &str, confirm_link: &str) -> (String, String) {
    let text = format!(
        "Hey there! (◕‿◕)\n\nYou're one step away from newsletter greatness - open the link below within 30 minutes to confirm your subscription to the HuuThangLe site:\n\n{}\n\nIf this wasn't you, just ignore this email. No harm, no foul! (￣▽￣)ノ\n",
        confirm_link
    );

    let inner = format!(
        r#"<h1 style="margin:0 0 16px;font-size:24px;font-weight:700;color:{TEXT};line-height:1.3;">You're one click away! (＾▽＾)</h1>
<p style="margin:0 0 20px;">Hey there! Thanks for subscribing to the HuuThangLe newsletter. Confirm your email address by clicking the button below within <strong>30 minutes</strong> and you're officially in the club.</p>
{button}
<p style="margin:24px 0 0;font-size:14px;color:{MUTED};">If you didn't sign up for this, you can safely ignore this email - no spam, no hard feelings! (´-ω-`)</p>"#,
        button = render_button("Confirm subscription ✨", confirm_link),
    );

    let footer = site_footer(app_base_url, "You received this email because this address was used to subscribe.");
    let html = render_shell(app_base_url, "Confirm your subscription to the HuuThangLe newsletter.", &inner, &footer);
    (text, html)
}

/// Newsletter campaign email. `body_html` is trusted content (from the site
/// owner or the post-publish flow) and is inserted verbatim.
pub fn campaign(
    app_base_url: &str,
    body_html: &str,
    body_text: &str,
    unsubscribe_link: &str,
) -> (String, String) {
    let text = format!(
        "{}\n\n---\nSent with ♡ by HuuThangLe.site\nUnsubscribe: {}\n",
        body_text, unsubscribe_link
    );

    let site = escape_html(&format!("{}/", app_base_url.trim_end_matches('/')));
    let footer = format!(
        r#"Sent with ♡ by <a href="{site}" style="color:{PRIMARY};text-decoration:none;">huuthangle.site</a> · <a href="{unsub}" style="color:{PRIMARY};text-decoration:none;">Unsubscribe</a>"#,
        unsub = escape_html(unsubscribe_link),
    );

    let html = render_shell(app_base_url, "Latest from HuuThangLe.site", body_html, &footer);
    (text, html)
}

/// Structured post data used to render a post-publish campaign email.
#[cfg(debug_assertions)]
pub struct CampaignPostData {
    pub title: String,
    pub excerpt: String,
    pub cover_url: Option<String>,
    pub post_url: String,
}

/// Text body for a post campaign (title, excerpt, link).
pub fn campaign_post_text(title: &str, excerpt: &str, post_url: &str) -> String {
    format!("{title}\n\n{excerpt}\n\nRead it here: {post_url}")
}

/// Inner HTML for a post campaign: thumbnail, title, excerpt and a CTA.
/// `body_html` for post campaigns is built from this, then wrapped by
/// [`campaign`] (which adds the shell + unsubscribe footer).
pub fn campaign_post_body(title: &str, excerpt: &str, cover_url: Option<&str>, post_url: &str) -> String {
    let cover = cover_url
        .map(|url| {
            format!(
                r#"<div style="width:100%;max-width:544px;height:0;padding-bottom:52.36%;position:relative;background:#eef1f6;margin:0 0 20px;"><img src="{url}" alt="" style="position:absolute;top:0;left: 50%;width: fit-content;height:100%;object-fit:contain;border-radius:10px;display:block;border:0;transform:translateX(-50%);"></div>"#,
                url = escape_html(url),
            )
        })
        .unwrap_or_default();

    format!(
        r#"{cover}
<h2 style="margin:0 0 12px;font-size:22px;font-weight:700;color:{TEXT};line-height:1.35;">{title}</h2>
<p style="margin:0 0 20px;">{excerpt}</p>
{button}
<p style="margin:20px 0 0;font-size:13px;color:{MUTED};">Enjoyed this? There's plenty more where that came from. (≧▽≦)</p>"#,
        cover = cover,
        title = escape_html(title),
        excerpt = escape_html(excerpt),
        button = render_button("Read the full post", post_url),
    )
}

/// Account verification email.
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

    let footer = site_footer(app_base_url, "You received this email because this address was used to create an account.");
    let html = render_shell(app_base_url, "Verify your HuuThangLe account", &inner, &footer);
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

    let footer = site_footer(app_base_url, "You received this email because a password reset was requested for this address.");
    let html = render_shell(app_base_url, "Reset your HuuThangLe password", &inner, &footer);
    (text, html)
}

/// Contact form confirmation email sent back to the visitor.
pub fn contact_confirmation(app_base_url: &str, name: &str) -> (String, String) {
    let text = format!(
        "Hey {name}! (◕‿◕)\n\nThanks for reaching out! I got your message and will get back to you soon.\n\n- Huu Thang (•̀ᴗ•́)و",
    );

    let inner = format!(
        r#"<h1 style="margin:0 0 16px;font-size:24px;font-weight:700;color:{TEXT};line-height:1.3;">Thanks for reaching out! (≧◡≦)</h1>
<p style="margin:0 0 20px;">Hey {name}, your message landed safely in my inbox. I'll get back to you as soon as I can!</p>
<p style="margin:0;color:{MUTED};">- Huu Thang (•̀ᴗ•́)و</p>"#,
        name = escape_html(name),
    );

    let footer = site_footer(app_base_url, "This is a confirmation that your message was received.");
    let html = render_shell(app_base_url, "Thanks for reaching out!", &inner, &footer);
    (text, html)
}

/// Renders every email type side by side so templates can be previewed in a
/// browser (used by the dev-only `/api/mail/preview` route). `campaign_post`
/// (a real post, picked by the handler) is used for the campaign sample when
/// present; otherwise a hardcoded placeholder is rendered.
#[cfg(debug_assertions)]
pub fn preview_page(app_base_url: &str, campaign_post: Option<&CampaignPostData>) -> String {
    let unsubscribe = format!("{app_base_url}/newsletter/unsubscribe?token=sample");

    let campaign_html = match campaign_post {
        Some(post) => {
            let (_, html) = campaign(
                app_base_url,
                &campaign_post_body(
                    &post.title,
                    &post.excerpt,
                    post.cover_url.as_deref(),
                    &post.post_url,
                ),
                &campaign_post_text(&post.title, &post.excerpt, &post.post_url),
                &unsubscribe,
            );
            html
        }
        None => {
            let sample_body = format!(
                r#"<h2 style="margin:0 0 12px;font-size:20px;font-weight:700;color:#333a4d;">A sample blog post</h2><p style="margin:0 0 16px;">This is the excerpt that gets emailed to subscribers when a new post is published. It can contain <strong>bold</strong>, <em>italics</em>, and links.</p><p style="margin:0;"><a href="{sample_url}" style="color:#7a86b6;font-weight:600;">Read the full post</a></p>"#,
                sample_url = escape_html(&format!("{}/posts/sample", app_base_url.trim_end_matches('/'))),
            );
            let (_, html) = campaign(
                app_base_url,
                &sample_body,
                "A sample blog post\n\nThis is the excerpt that gets emailed to subscribers when a new post is published.",
                &unsubscribe,
            );
            html
        }
    };

    let samples = [
        (
            "Subscription confirm",
            subscription_confirm(
                app_base_url,
                &format!("{app_base_url}/newsletter/confirm?token=sample"),
            ),
        ),
        ("Campaign", (String::new(), campaign_html)),
        (
            "Account verification",
            verification(
                app_base_url,
                "thang",
                &format!("{app_base_url}/verify-email?token=sample"),
            ),
        ),
        (
            "Password reset",
            password_reset(
                app_base_url,
                "thang",
                &format!("{app_base_url}/reset-password?token=sample"),
            ),
        ),
        (
            "Contact confirmation",
            contact_confirmation(app_base_url, "Visitor"),
        ),
    ];

    let mut body = String::new();
    for (label, (_, html)) in samples {
        body.push_str(&format!(
            r#"<h1 style="margin:32px 0 8px;font-family:system-ui,sans-serif;font-size:20px;color:#222;">{label}</h1>"#
        ));
        body.push_str(&html);
    }

    // The hosted logo may not be reachable from the backend-only preview, so
    // inline the embedded copy instead of `{app_base_url}/logo.png`.
    let remote_logo = format!("{}/logo.png", app_base_url.trim_end_matches('/'));
    let body = body.replace(&remote_logo, &logo_data_uri());

    format!(
        r#"<!DOCTYPE html>
<html lang="en">
<head>
<meta charset="utf-8">
<meta name="viewport" content="width=device-width, initial-scale=1">
<title>Email template previews</title>
<style>
    body {{ margin:0; padding:32px; background:#d8dbe4; font-family:system-ui,sans-serif; }}
    body.dark {{ background:#0f1219; }}
    #theme-toggle {{ position:fixed; top:16px; right:16px; z-index:1000; padding:10px 14px; border-radius:9999px; border:1px solid #c3c9d6; background:#fff; color:#333a4d; font:600 14px system-ui,sans-serif; cursor:pointer; }}
    body.dark #theme-toggle {{ background:#232a3a; color:#e6e8f0; border-color:#3a4260; }}
    h1 {{ margin:32px 0 8px; font-family:system-ui,sans-serif; font-size:20px; color:#222; }}
    body.dark h1 {{ color:#cfd6e4; }}
    body.dark [style*="background-color:#eef0f6"] {{ background-color:#161a26 !important; }}
    body.dark [style*="background:#eef1f6"] {{ background:#1a2030 !important; }}
    body.dark [style*="background-color:#ffffff"] {{ background-color:#232a3a !important; }}
    body.dark [style*="color:#333a4d"] {{ color:#e6e8f0 !important; }}
    body.dark [style*="color:#6b7280"] {{ color:#a7b0c2 !important; }}
    body.dark [style*="border-top:1px solid #e5e7eb"] {{ border-top-color:#343c52 !important; }}
</style>
</head>
<body style="margin:0;padding:32px;background:#d8dbe4;font-family:system-ui,sans-serif;">{body}
<script>
    const btn = document.createElement('button');
    btn.id = 'theme-toggle';
    const apply = () => {{
        btn.textContent = document.body.classList.contains('dark') ? '☀️' : '🌙';
    }};
    btn.onclick = () => {{
        document.body.classList.toggle('dark');
        apply();
        try {{ localStorage.setItem('mail-preview-theme', document.body.classList.contains('dark') ? 'dark' : 'light'); }} catch {{}}
    }};
    document.body.prepend(btn);
    try {{
        if (localStorage.getItem('mail-preview-theme') === 'dark') document.body.classList.add('dark');
    }} catch {{}}
    apply();
</script>
</body>
</html>"#
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    const BASE: &str = "https://blog.huuthangle.site";

    #[test]
    fn escape_html_escapes_special_chars() {
        assert_eq!(
            escape_html(r#"<a href="x">&'</a>"#),
            "&lt;a href=&quot;x&quot;&gt;&amp;&#39;&lt;/a&gt;"
        );
    }

    #[test]
    fn shell_contains_brand_header_and_footer() {
        let html = render_shell(BASE, "preheader", "<p>body</p>", "footer");
        assert!(html.contains("/logo.png"));
        assert!(html.contains("huuthangle.site"));
        assert!(html.contains("<p>body</p>"));
        assert!(html.contains("footer"));
        assert!(html.contains("preheader"));
    }

    #[test]
    fn subscription_confirm_contains_link_and_button() {
        let link = format!("{BASE}/newsletter/confirm?token=abc");
        let (text, html) = subscription_confirm(BASE, &link);
        assert!(text.contains("confirm your subscription"));
        assert!(html.contains("token=abc"));
        assert!(html.contains("Confirm subscription"));
        assert!(html.contains("newsletter/confirm"));
    }

    #[test]
    fn campaign_wraps_content_and_unsubscribe() {
        let (text, html) = campaign(
            BASE,
            "<p>Hello world</p>",
            "Hello world",
            &format!("{BASE}/newsletter/unsubscribe?token=xyz"),
        );
        assert!(text.contains("Unsubscribe"));
        assert!(html.contains("<p>Hello world</p>"));
        assert!(html.contains("newsletter/unsubscribe?token=xyz"));
        assert!(html.contains("Unsubscribe"));
    }

    #[test]
    fn campaign_post_renders_title_excerpt_cover_and_cta() {
        let text = campaign_post_text("My Title", "My excerpt here", "https://x.test/posts/my");
        assert!(text.contains("My Title"));
        assert!(text.contains("My excerpt here"));
        assert!(text.contains("https://x.test/posts/my"));

        let html =
            campaign_post_body("My Title", "My excerpt here", Some("https://x.test/c.jpg"), "https://x.test/posts/my");
        assert!(html.contains("My Title"));
        assert!(html.contains("My excerpt here"));
        assert!(html.contains("https://x.test/c.jpg"));
        assert!(html.contains("object-fit:contain"));
        assert!(html.contains("padding-bottom:52.36%"));
        assert!(html.contains(r#"object-fit:contain;border-radius:10px"#));
        assert!(html.contains("Read the full post"));

        let no_cover =
            campaign_post_body("T", "E", None, "https://x.test/posts/my");
        assert!(!no_cover.contains("<img"));
        assert!(no_cover.contains("Read the full post"));
    }

    #[test]
    fn verification_and_reset_render_buttons() {
        let (_, html) = verification(BASE, "thang", &format!("{BASE}/verify-email?token=v"));
        assert!(html.contains("Verify email"));
        assert!(html.contains("token=v"));

        let (_, html) = password_reset(BASE, "thang", &format!("{BASE}/reset-password?token=r"));
        assert!(html.contains("Reset password"));
        assert!(html.contains("token=r"));
    }

    #[test]
    fn contact_confirmation_renders() {
        let (text, html) = contact_confirmation(BASE, "Thang & Co");
        assert!(text.contains("Thanks for reaching out"));
        assert!(html.contains("Thang &amp; Co"));
        assert!(html.contains("Thanks for reaching out"));
    }
}
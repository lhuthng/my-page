use super::layout::{render_button, render_shell, site_footer};
#[cfg(debug_assertions)]
use super::layout::logo_data_uri;
/// Branded HTML email templates.
///
/// Email clients strip `<style>` blocks and don't support flexbox/grid, so
/// every email is rendered as a table-based layout with inline styles only.
/// The design is a light theme (white card on light gray) with the site's
/// navy/lavender accents, matching the huuthangle.site brand.
/// Subscription and campaign email bodies.
use super::{MUTED, PRIMARY, TEXT, escape_html};

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

    let footer = site_footer(
        app_base_url,
        "You received this email because this address was used to subscribe.",
    );
    let html = render_shell(
        app_base_url,
        "Confirm your subscription to the HuuThangLe newsletter.",
        &inner,
        &footer,
    );
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

    let html = render_shell(
        app_base_url,
        "Latest from HuuThangLe.site",
        body_html,
        &footer,
    );
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
pub fn campaign_post_body(
    title: &str,
    excerpt: &str,
    cover_url: Option<&str>,
    post_url: &str,
) -> String {
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

/// Renders every email type side by side so templates can be previewed in a
/// browser (used by the dev-only `/api/mail/preview` route). `campaign_post`
/// (a real post, picked by the handler) is used for the campaign sample when
/// present; otherwise a hardcoded placeholder is rendered.
#[cfg(debug_assertions)]
pub fn preview_page(app_base_url: &str, campaign_post: Option<&CampaignPostData>) -> String {
    use super::auth::{password_reset, verification};
    use super::contact::contact_confirmation;

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
                sample_url = escape_html(&format!(
                    "{}/posts/sample",
                    app_base_url.trim_end_matches('/')
                )),
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

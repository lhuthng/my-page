/// Branded HTML email templates.
///
/// Email clients strip `<style>` blocks and don't support flexbox/grid, so
/// every email is rendered as a table-based layout with inline styles only.
/// The design is a light theme (white card on light gray) with the site's
/// navy/lavender accents, matching the huuthangle.site brand.
/// Subscription and campaign email bodies.
use super::escape_html;
use super::layout::{
    CARD, DIVIDER, MUTED, NAVY, PRIMARY, TEXT, WHITE, logo_data_uri, render_button, render_shell,
    site_footer,
};

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

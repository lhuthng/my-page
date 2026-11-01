/// Comment notification email for replies, mentions, and new comments on a
/// post the recipient wrote.
use crate::domain::entities::notification::NotificationKind;

use super::layout::{render_button, render_shell, site_footer};
use super::markdown::render_comment;
use super::{PRIMARY, TEXT, escape_html};

/// How many characters of a fresh comment are quoted in the email.
const COMMENT_EXCERPT_CHARS: usize = 280;

fn copy_for(kind: NotificationKind) -> (&'static str, &'static str, &'static str, &'static str) {
    match kind {
        NotificationKind::Reply => (
            "New reply (ﾉ´ヮ`)ﾉ*: ･ﾟ",
            "replied to your comment on",
            "replied to your comment",
            "You commented on this post.",
        ),
        NotificationKind::Mention => (
            "You were mentioned (・∀・)",
            "mentioned you in a comment on",
            "mentioned you",
            "You were mentioned in a comment.",
        ),
        NotificationKind::PostComment => (
            "New comment (ﾉ´ヮ`)ﾉ*: ･ﾟ",
            "just left a comment on",
            "commented on your post",
            "You wrote this post.",
        ),
    }
}

#[allow(clippy::too_many_arguments)] // each input is a distinct string; a struct would only rename them
pub fn comment_notification(
    app_base_url: &str,
    kind: NotificationKind,
    recipient_name: &str,
    post_title: &str,
    commenter_name: &str,
    comment_markdown: &str,
    comment_url: &str,
    manage_url: &str,
) -> (String, String) {
    // Comments are stored as raw markdown, so render them the same way the
    // site does instead of quoting `**bold**` and `![gif](...)` literally.
    let rendered = render_comment(comment_markdown, COMMENT_EXCERPT_CHARS);
    let excerpt_text = rendered.text.trim();
    let (heading, lead, preheader_tail, footer_note) = copy_for(kind);

    let text = format!(
        "Hey {recipient_name}! (ﾉ´ヮ`)ﾉ*: ･ﾟ\n\n{commenter_name} {lead} \"{post_title}\":\n\n\"{excerpt_text}\"\n\nRead it here:\n{comment_url}\n\nManage email notifications:\n{manage_url}\n",
    );

    // Email clients do not support auto margins, so the CTA sits in a
    // full-width table cell and is aligned to the right edge.
    let button = format!(
        r#"<table role="presentation" width="100%" cellpadding="0" cellspacing="0" border="0"><tr><td align="right">{}</td></tr></table>"#,
        render_button("Read the comment", comment_url)
    );

    let inner = format!(
        r#"<h1 style="margin:0 0 16px;font-size:24px;font-weight:700;color:{TEXT};line-height:1.3;">{heading}</h1>
<p style="margin:0 0 20px;">Hey {recipient_name}, <strong>{commenter_name}</strong> {lead} <strong>"{post_title}"</strong>:</p>
<div style="margin:0 0 24px;padding:12px 16px;border-left:3px solid {PRIMARY};color:{TEXT};font-size:15px;line-height:1.6;">{excerpt_html}</div>
{button}"#,
        heading = heading,
        recipient_name = escape_html(recipient_name),
        commenter_name = escape_html(commenter_name),
        lead = lead,
        post_title = escape_html(post_title),
        excerpt_html = rendered.html,
        button = button,
    );

    // The manage link is session-based on purpose: unlike the newsletter's
    // token, it survives a JWT_SECRET rotation.
    let footer = format!(
        r#"{}<br><a href="{}" style="color:{PRIMARY};text-decoration:none;">Manage notifications</a>"#,
        site_footer(app_base_url, footer_note),
        escape_html(manage_url),
    );
    let html = render_shell(
        app_base_url,
        &format!("{commenter_name} {preheader_tail}"),
        &inner,
        &footer,
    );
    (text, html)
}

#[cfg(test)]
mod tests {
    use super::comment_notification;
    use crate::domain::entities::notification::NotificationKind;

    #[test]
    fn escapes_values_in_html_but_keeps_text_readable() {
        let (text, html) = comment_notification(
            "https://huuthangle.site",
            NotificationKind::PostComment,
            "Thắng <admin>",
            "My \"post\" & <things>",
            "<script>alert(1)</script>",
            "line one\nline two",
            "https://huuthangle.site/projects/deep-dive#comment-42",
            "https://huuthangle.site/profiles/me?section=notifications",
        );

        // The plain-text part stays human-readable.
        assert!(text.contains("<script>alert(1)</script>"));
        assert!(text.contains("line one\nline two"));
        // The HTML part escapes hostile values and keeps the deep link.
        assert!(html.contains("&lt;script&gt;"));
        assert!(html.contains("My &quot;post&quot; &amp; &lt;things&gt;"));
        assert!(html.contains("line one\nline two"));
        assert!(!html.contains("<script>"));
        assert!(html.contains(r#"href="https://huuthangle.site/projects/deep-dive#comment-42""#));
    }

    #[test]
    fn renders_markdown_inside_the_quote() {
        let (text, html) = comment_notification(
            "https://huuthangle.site",
            NotificationKind::PostComment,
            "Thang",
            "Ownership",
            "cutiecube",
            "**bold** and *italic* and `code`\n\n```rust\nfn main() {}\n```",
            "https://huuthangle.site/posts/ownership#comment-7",
            "https://huuthangle.site/profiles/thang?section=notifications",
        );

        assert!(html.contains("<strong>bold</strong>"));
        assert!(html.contains("<em>italic</em>"));
        assert!(html.contains("<code style="));
        assert!(html.contains("<pre style="));
        assert!(!html.contains("**bold**"));

        // The plain-text part drops the syntax but keeps the words.
        assert!(text.contains("bold and italic and code"));
        assert!(!text.contains("**"));
        assert!(!text.contains("```"));
    }

    #[test]
    fn renders_gifs_as_images_and_links_spoofed_urls_as_text() {
        let (text, html) = comment_notification(
            "https://huuthangle.site",
            NotificationKind::PostComment,
            "Thang",
            "GIFs",
            "loafphantom",
            "![dance](https://media.giphy.com/media/abc/giphy.gif)\n\n![x](javascript:alert(1))",
            "https://huuthangle.site/posts/gifs#comment-8",
            "https://huuthangle.site/profiles/thang?section=notifications",
        );

        assert!(html.contains(r#"<img src="https://media.giphy.com/media/abc/giphy.gif""#));
        assert!(html.contains("max-width:100%"));
        assert!(html.contains("[image: x]"));
        assert!(!html.contains("javascript:"));
        assert!(!html.contains("alert(1)\""));
        assert!(text.contains("[image: dance] https://media.giphy.com/media/abc/giphy.gif"));
    }

    #[test]
    fn truncation_does_not_leave_open_markup() {
        let long = format!("{} **tail**", "word ".repeat(200));
        let (text, html) = comment_notification(
            "https://huuthangle.site",
            NotificationKind::PostComment,
            "Thang",
            "Long",
            "cutiecube",
            &long,
            "https://huuthangle.site/posts/long#comment-9",
            "https://huuthangle.site/profiles/thang?section=notifications",
        );

        assert!(text.contains('…'));
        assert!(html.contains('…'));
        assert!(!html.contains("**"));
        // Every tag emitted by the renderer is balanced.
        let opens = html.matches("<p ").count() + html.matches("<div ").count();
        let closes = html.matches("</p>").count() + html.matches("</div>").count();
        assert_eq!(opens, closes);

        // The manage link is present in both alternatives.
        assert!(html.contains("Manage notifications"));
        assert!(text.contains("Manage email notifications"));
    }

    #[test]
    fn reply_and_mention_copy_match_the_reason() {
        let (reply_text, reply_html) = comment_notification(
            "https://huuthangle.site",
            NotificationKind::Reply,
            "Thang",
            "Ownership",
            "cutiecube",
            "thanks!",
            "https://huuthangle.site/posts/ownership#comment-10",
            "https://huuthangle.site/profiles/thang?section=notifications",
        );
        assert!(reply_text.contains("replied to your comment on"));
        assert!(reply_html.contains("New reply"));
        assert!(reply_html.contains("You commented on this post."));

        let (mention_text, mention_html) = comment_notification(
            "https://huuthangle.site",
            NotificationKind::Mention,
            "Thang",
            "Ownership",
            "loafphantom",
            "ping @thang",
            "https://huuthangle.site/posts/ownership#comment-11",
            "https://huuthangle.site/profiles/thang?section=notifications",
        );
        assert!(mention_text.contains("mentioned you in a comment on"));
        assert!(mention_html.contains("You were mentioned"));
        assert!(mention_html.contains("You were mentioned in a comment."));
    }
}

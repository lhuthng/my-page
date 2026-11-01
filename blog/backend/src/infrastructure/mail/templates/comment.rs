/// New-comment notification email sent to a post's author.
use super::layout::{render_button, render_shell, site_footer};
use super::markdown::render_comment;
use super::{PRIMARY, TEXT, escape_html};

/// How many characters of a fresh comment are quoted in the author email.
const COMMENT_EXCERPT_CHARS: usize = 280;

pub fn comment_notification(
    app_base_url: &str,
    author_name: &str,
    post_title: &str,
    commenter_name: &str,
    comment_markdown: &str,
    comment_url: &str,
) -> (String, String) {
    // Comments are stored as raw markdown, so render them the same way the
    // site does instead of quoting `**bold**` and `![gif](...)` literally.
    let rendered = render_comment(comment_markdown, COMMENT_EXCERPT_CHARS);
    let excerpt_text = rendered.text.trim();

    let text = format!(
        "Hey {author_name}! (ﾉ´ヮ`)ﾉ*: ･ﾟ\n\n{commenter_name} left a comment on \"{post_title}\":\n\n\"{excerpt_text}\"\n\nRead it here:\n{comment_url}\n",
    );

    let inner = format!(
        r#"<h1 style="margin:0 0 16px;font-size:24px;font-weight:700;color:{TEXT};line-height:1.3;">New comment (ﾉ´ヮ`)ﾉ*: ･ﾟ</h1>
<p style="margin:0 0 20px;">Hey {author_name}, <strong>{commenter_name}</strong> just left a comment on <strong>"{post_title}"</strong>:</p>
<div style="margin:0 0 24px;padding:12px 16px;border-left:3px solid {PRIMARY};color:{TEXT};font-size:15px;line-height:1.6;">{excerpt_html}</div>
{button}"#,
        author_name = escape_html(author_name),
        commenter_name = escape_html(commenter_name),
        post_title = escape_html(post_title),
        excerpt_html = rendered.html,
        button = render_button("Read the comment", comment_url),
    );

    let footer = site_footer(app_base_url, "You wrote this post.");
    let html = render_shell(
        app_base_url,
        &format!("{commenter_name} commented on your post"),
        &inner,
        &footer,
    );
    (text, html)
}

#[cfg(test)]
mod tests {
    use super::comment_notification;

    #[test]
    fn escapes_values_in_html_but_keeps_text_readable() {
        let (text, html) = comment_notification(
            "https://huuthangle.site",
            "Thắng <admin>",
            "My \"post\" & <things>",
            "<script>alert(1)</script>",
            "line one\nline two",
            "https://huuthangle.site/projects/deep-dive#comment-42",
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
            "Thang",
            "Ownership",
            "cutiecube",
            "**bold** and *italic* and `code`\n\n```rust\nfn main() {}\n```",
            "https://huuthangle.site/posts/ownership#comment-7",
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
            "Thang",
            "GIFs",
            "loafphantom",
            "![dance](https://media.giphy.com/media/abc/giphy.gif)\n\n![x](javascript:alert(1))",
            "https://huuthangle.site/posts/gifs#comment-8",
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
            "Thang",
            "Long",
            "cutiecube",
            &long,
            "https://huuthangle.site/posts/long#comment-9",
        );

        assert!(text.contains('…'));
        assert!(html.contains('…'));
        assert!(!html.contains("**"));
        // Every tag emitted by the renderer is balanced.
        let opens = html.matches("<p ").count() + html.matches("<div ").count();
        let closes = html.matches("</p>").count() + html.matches("</div>").count();
        assert_eq!(opens, closes);
    }
}

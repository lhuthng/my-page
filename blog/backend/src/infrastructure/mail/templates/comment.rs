/// New-comment notification email sent to a post's author.
use super::layout::{render_button, render_shell, site_footer};
use super::{MUTED, PRIMARY, TEXT, escape_html};

pub fn comment_notification(
    app_base_url: &str,
    author_name: &str,
    post_title: &str,
    commenter_name: &str,
    comment_excerpt: &str,
    comment_url: &str,
) -> (String, String) {
    let text = format!(
        "Hey {author_name}! (ﾉ´ヮ`)ﾉ*: ･ﾟ\n\n{commenter_name} left a comment on \"{post_title}\":\n\n\"{comment_excerpt}\"\n\nRead it here:\n{comment_url}\n",
    );

    // Email clients ignore white-space styling, so hard newlines in the
    // comment become explicit <br> after escaping.
    let excerpt_html = escape_html(comment_excerpt).replace('\n', "<br>");
    let inner = format!(
        r#"<h1 style="margin:0 0 16px;font-size:24px;font-weight:700;color:{TEXT};line-height:1.3;">New comment (ﾉ´ヮ`)ﾉ*: ･ﾟ</h1>
<p style="margin:0 0 20px;">Hey {author_name}, <strong>{commenter_name}</strong> just left a comment on <strong>"{post_title}"</strong>:</p>
<p style="margin:0 0 24px;padding:12px 16px;border-left:3px solid {PRIMARY};color:{TEXT};font-size:15px;line-height:1.6;">{excerpt_html}</p>
{button}
<p style="margin:24px 0 0;font-size:14px;color:{MUTED};">You're receiving this because you wrote this post.</p>"#,
        author_name = escape_html(author_name),
        commenter_name = escape_html(commenter_name),
        post_title = escape_html(post_title),
        excerpt_html = excerpt_html,
        button = render_button("Read the comment", comment_url),
    );

    let footer = site_footer(
        app_base_url,
        "You received this email because you wrote this post.",
    );
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
        assert!(html.contains("line one<br>line two"));
        assert!(html.contains(r#"href="https://huuthangle.site/projects/deep-dive#comment-42""#));
    }
}

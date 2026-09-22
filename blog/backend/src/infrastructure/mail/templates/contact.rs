use super::layout::{render_shell, site_footer};
/// Contact form confirmation email sent back to the visitor.
use super::{MUTED, TEXT, escape_html};

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

    let footer = site_footer(
        app_base_url,
        "This is a confirmation that your message was received.",
    );
    let html = render_shell(app_base_url, "Thanks for reaching out!", &inner, &footer);
    (text, html)
}

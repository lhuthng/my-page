/// Renders a comment's markdown into email-safe HTML and plain text.
///
/// The frontend renders comments with markdown-it's default preset:
/// CommonMark plus tables and strikethrough, `html: false`, and `linkify:
/// false`. This renderer mirrors that subset so the email quote shows the
/// same markup the reader sees on the site.
///
/// Email clients strip `<style>` blocks, so every tag is emitted with inline
/// styles. Raw HTML is escaped, and only http(s) URLs become links or images,
/// so a comment cannot carry `javascript:` URLs or tracking markup into an
/// inbox.
use pulldown_cmark::{Event, HeadingLevel, Options, Parser, Tag, TagEnd};

use super::{DIVIDER, MUTED, PRIMARY, TEXT, escape_html};

const INLINE_CODE_STYLE: &str = "font-family:ui-monospace,SFMono-Regular,Menlo,monospace;font-size:13px;background:#f3f4f8;border-radius:4px;padding:1px 4px;word-break:break-word;";
const CODE_BLOCK_STYLE: &str = "margin:0 0 12px;padding:12px;background:#f3f4f8;border-radius:8px;font-family:ui-monospace,SFMono-Regular,Menlo,monospace;font-size:13px;line-height:1.5;color:#333a4d;white-space:pre-wrap;word-break:break-word;";
const LINK_STYLE: &str = "color:#7a86b6;text-decoration:underline;word-break:break-word;";
const IMAGE_STYLE: &str = "max-width:100%;height:auto;border-radius:8px;display:block;";
const CELL_STYLE: &str =
    "border:1px solid #e5e7eb;padding:6px 8px;text-align:left;vertical-align:top;";

pub(super) struct RenderedComment {
    pub html: String,
    pub text: String,
}

/// Renders `markdown` for the notification email. Visible characters are
/// capped at `max_chars`; the cut happens while walking the parsed document,
/// so the output never ends with half a tag or half a `**`.
pub(super) fn render_comment(markdown: &str, max_chars: usize) -> RenderedComment {
    let mut renderer = CommentRenderer {
        html: String::new(),
        text: String::new(),
        remaining: max_chars,
        truncated: false,
        open: Vec::new(),
        image: None,
        image_count: 0,
        link: None,
        in_table_head: false,
    };

    let options = Options::ENABLE_TABLES | Options::ENABLE_STRIKETHROUGH;
    for event in Parser::new_ext(markdown, options) {
        if !renderer.handle(event) {
            break;
        }
    }

    renderer.finish()
}

struct ImageState {
    url: String,
    alt: String,
}

struct LinkState {
    text_start: usize,
    url: String,
}

struct CommentRenderer {
    html: String,
    text: String,
    remaining: usize,
    truncated: bool,
    /// Closing tags for elements still open, newest last. Used to unwind
    /// cleanly when the excerpt is cut mid-element.
    open: Vec<&'static str>,
    image: Option<ImageState>,
    image_count: usize,
    link: Option<LinkState>,
    in_table_head: bool,
}

impl CommentRenderer {
    /// Returns false once the excerpt is full, so the parser stops.
    fn handle(&mut self, event: Event<'_>) -> bool {
        match event {
            Event::Start(tag) => {
                self.start(tag);
                true
            }
            Event::End(tag) => {
                self.end(tag);
                true
            }
            Event::Text(value) => {
                if let Some(image) = self.image.as_mut() {
                    image.alt.push_str(&value);
                    return true;
                }
                match self.take(&value) {
                    Some(part) => {
                        self.push_text(&part);
                        !self.truncated
                    }
                    None => false,
                }
            }
            Event::Code(value) => match self.take(&value) {
                Some(part) => {
                    self.html.push_str(&format!(
                        "<code style=\"{INLINE_CODE_STYLE}\">{}</code>",
                        escape_html(&part)
                    ));
                    self.text.push_str(&part);
                    !self.truncated
                }
                None => false,
            },
            Event::Html(value) | Event::InlineHtml(value) => match self.take(&value) {
                Some(part) => {
                    self.push_text(&part);
                    !self.truncated
                }
                None => false,
            },
            Event::SoftBreak => {
                self.html.push('\n');
                self.text.push('\n');
                true
            }
            Event::HardBreak => {
                self.html.push_str("<br>");
                self.text.push('\n');
                true
            }
            Event::Rule => {
                self.html.push_str(&format!(
                    "<hr style=\"margin:12px 0;border:0;border-top:2px dashed {DIVIDER};\">"
                ));
                self.text_break();
                self.text.push_str("---");
                self.text_break();
                true
            }
            _ => true,
        }
    }

    fn start(&mut self, tag: Tag<'_>) {
        match tag {
            Tag::Paragraph => {
                self.push_open("<p style=\"margin:0 0 12px;line-height:1.6;\">", "</p>");
                self.text_break();
            }
            Tag::Heading { level, .. } => {
                let (name, size) = heading_style(level);
                self.push_open(
                    &format!(
                        "<{name} style=\"margin:16px 0 8px;font-size:{size}px;font-weight:700;color:{TEXT};line-height:1.3;\">"
                    ),
                    heading_close(level),
                );
                self.text_break();
            }
            Tag::BlockQuote(_) => {
                self.push_open(
                    &format!(
                        "<blockquote style=\"margin:0 0 12px;padding:4px 12px;border-left:3px solid {PRIMARY};color:{MUTED};\">"
                    ),
                    "</blockquote>",
                );
                self.text_break();
            }
            Tag::CodeBlock(_) => {
                self.push_open(
                    &format!("<pre style=\"{CODE_BLOCK_STYLE}\"><code>"),
                    "</code></pre>",
                );
                self.text_break();
            }
            Tag::List(Some(_)) => {
                self.push_open(
                    "<ol style=\"margin:0 0 12px;padding-left:20px;line-height:1.6;\">",
                    "</ol>",
                );
                self.text_break();
            }
            Tag::List(None) => {
                self.push_open(
                    "<ul style=\"margin:0 0 12px;padding-left:20px;line-height:1.6;\">",
                    "</ul>",
                );
                self.text_break();
            }
            Tag::Item => {
                self.push_open("<li style=\"margin:0 0 4px;\">", "</li>");
                self.text_break();
                self.text.push_str("- ");
            }
            Tag::Table(_) => {
                self.push_open(
                    "<table role=\"presentation\" width=\"100%\" cellpadding=\"0\" cellspacing=\"0\" style=\"border-collapse:collapse;margin:0 0 12px;font-size:14px;\">",
                    "</table>",
                );
                self.text_break();
            }
            Tag::TableHead => {
                self.in_table_head = true;
                self.push_open("<thead><tr>", "</tr></thead>");
            }
            Tag::TableRow => {
                self.push_open("<tr>", "</tr>");
            }
            Tag::TableCell => {
                let name = if self.in_table_head { "th" } else { "td" };
                self.push_open(
                    &format!("<{name} style=\"{CELL_STYLE}\">"),
                    cell_close(name),
                );
                self.text_break();
            }
            Tag::Emphasis => self.push_open("<em>", "</em>"),
            Tag::Strong => self.push_open("<strong>", "</strong>"),
            Tag::Strikethrough => self.push_open("<s>", "</s>"),
            Tag::Link { dest_url, .. } => {
                if is_safe_url(&dest_url) {
                    self.push_open(
                        &format!(
                            "<a href=\"{}\" style=\"{LINK_STYLE}\">",
                            escape_html(&dest_url)
                        ),
                        "</a>",
                    );
                    self.link = Some(LinkState {
                        text_start: self.text.len(),
                        url: dest_url.to_string(),
                    });
                }
            }
            Tag::Image { dest_url, .. } => {
                self.image = Some(ImageState {
                    url: dest_url.to_string(),
                    alt: String::new(),
                });
            }
            _ => {}
        }
    }

    fn end(&mut self, tag: TagEnd) {
        match tag {
            TagEnd::Paragraph => {
                self.close();
                self.text_break();
            }
            TagEnd::Heading(_) => {
                self.close();
                self.text_break();
            }
            TagEnd::BlockQuote(_) => {
                self.close();
                self.text_break();
            }
            TagEnd::CodeBlock => {
                self.close();
                self.text_break();
            }
            TagEnd::List(_) => {
                self.close();
                self.text_break();
            }
            TagEnd::Item => {
                self.close();
                self.text_break();
            }
            TagEnd::Table => {
                self.close();
                self.text_break();
            }
            TagEnd::TableHead => {
                self.in_table_head = false;
                self.close();
            }
            TagEnd::TableRow => self.close(),
            TagEnd::TableCell => {
                self.close();
                self.text_break();
            }
            TagEnd::Emphasis | TagEnd::Strong | TagEnd::Strikethrough => self.close(),
            TagEnd::Link => {
                if let Some(link) = self.link.take() {
                    self.close();
                    // Email clients need the destination spelled out in the
                    // plain-text part; skip it when the label already is the URL.
                    let label = self
                        .text
                        .get(link.text_start..)
                        .unwrap_or("")
                        .trim()
                        .to_string();
                    if !label.is_empty() && label != link.url {
                        self.text.push_str(&format!(" <{}>", link.url));
                    }
                }
            }
            TagEnd::Image => self.finish_image(),
            _ => {}
        }
    }

    fn finish_image(&mut self) {
        let Some(image) = self.image.take() else {
            return;
        };
        let alt = image.alt.trim();
        let alt_label = if alt.is_empty() { "image" } else { alt };
        let safe = is_safe_url(&image.url);
        let embed = safe && self.image_count == 0;
        self.image_count += 1;

        if embed {
            self.html.push_str(&format!(
                "<img src=\"{}\" alt=\"{}\" style=\"{IMAGE_STYLE}\"><br>",
                escape_html(&image.url),
                escape_html(alt_label),
            ));
        }
        if safe {
            // Always link the image, because many clients block remote
            // images (or show only the first GIF frame). Skip the extra link
            // when the image already sits inside one, to avoid nested <a>.
            if self.link.is_none() {
                let label = if embed {
                    "Open image".to_string()
                } else {
                    format!("Open image: {alt_label}")
                };
                self.html.push_str(&format!(
                    "<a href=\"{}\" style=\"{LINK_STYLE}font-size:13px;\">{}</a>",
                    escape_html(&image.url),
                    escape_html(&label),
                ));
            }
            self.text
                .push_str(&format!("[image: {alt_label}] {}", image.url));
        } else {
            self.html.push_str(&format!(
                "<span style=\"font-size:13px;color:{MUTED};\">[image: {}]</span>",
                escape_html(alt_label),
            ));
            self.text.push_str(&format!("[image: {alt_label}]"));
        }
        self.text_break();
    }

    fn finish(mut self) -> RenderedComment {
        if self.truncated {
            self.html.push('…');
            self.text.push('…');
        }
        while let Some(close) = self.open.pop() {
            self.html.push_str(close);
        }
        RenderedComment {
            html: self.html,
            text: self.text,
        }
    }

    /// Takes at most `remaining` characters, flagging truncation when clipped.
    fn take(&mut self, value: &str) -> Option<String> {
        if self.remaining == 0 {
            self.truncated = true;
            return None;
        }
        let mut out = String::new();
        let mut taken = 0;
        for ch in value.chars() {
            if taken == self.remaining {
                self.truncated = true;
                break;
            }
            out.push(ch);
            taken += 1;
        }
        self.remaining -= taken;
        Some(out)
    }

    fn push_text(&mut self, value: &str) {
        self.html.push_str(&escape_html(value));
        self.text.push_str(value);
    }

    fn push_open(&mut self, open: &str, close: &'static str) {
        self.html.push_str(open);
        self.open.push(close);
    }

    fn close(&mut self) {
        if let Some(close) = self.open.pop() {
            self.html.push_str(close);
        }
    }

    fn text_break(&mut self) {
        if !self.text.is_empty() && !self.text.ends_with('\n') {
            self.text.push('\n');
        }
    }
}

/// Only http(s) is allowed, matching what email clients will actually open.
/// `javascript:`, `data:`, `file:` and relative URLs are dropped.
fn is_safe_url(url: &str) -> bool {
    let url = url.trim().to_ascii_lowercase();
    url.starts_with("https://") || url.starts_with("http://")
}

fn heading_style(level: HeadingLevel) -> (&'static str, usize) {
    match level {
        HeadingLevel::H1 => ("h1", 20),
        HeadingLevel::H2 => ("h2", 18),
        HeadingLevel::H3 => ("h3", 17),
        _ => ("h4", 16),
    }
}

fn heading_close(level: HeadingLevel) -> &'static str {
    match level {
        HeadingLevel::H1 => "</h1>",
        HeadingLevel::H2 => "</h2>",
        HeadingLevel::H3 => "</h3>",
        _ => "</h4>",
    }
}

fn cell_close(name: &'static str) -> &'static str {
    match name {
        "th" => "</th>",
        _ => "</td>",
    }
}

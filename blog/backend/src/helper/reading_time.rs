/// Estimated reading time for a post's markdown body.
///
/// Strips markdown structure (code fences, inline code, headings, links,
/// emphasis) and the blog's inline media/app tokens before counting words, so
/// syntax like `@[img:name]` or `:::app glb-demo` does not inflate the count.
use once_cell::sync::Lazy;
use regex::Regex;

pub const WORDS_PER_MINUTE: f64 = 150.0;

static FENCED_CODE: Lazy<Regex> = Lazy::new(|| Regex::new(r"(?s)```.*?```").unwrap());
static INLINE_CODE: Lazy<Regex> = Lazy::new(|| Regex::new(r"`([^`]+)`").unwrap());
static HEADING_MARKERS: Lazy<Regex> = Lazy::new(|| Regex::new(r"(?m)^#{1,6}\s+").unwrap());
static LINKS: Lazy<Regex> = Lazy::new(|| Regex::new(r"\[([^\]]+)\]\([^)]*\)").unwrap());
static IMAGES: Lazy<Regex> = Lazy::new(|| Regex::new(r"!\[([^\]]*)\]\([^)]*\)").unwrap());
static MEDIA_TOKENS: Lazy<Regex> = Lazy::new(|| Regex::new(r"@(?:\([^)]*\))?\[[^\]]*\]").unwrap());
static APP_DIRECTIVES: Lazy<Regex> = Lazy::new(|| Regex::new(r"(?m)^:::\w+\s+(.*)$").unwrap());

/// Strip markdown + blog-inline tokens, leaving roughly the readable words.
pub fn count_words(markdown: &str) -> usize {
    let mut text = markdown.to_string();

    // Fenced code blocks (``` ... ```) — drop entirely.
    text = FENCED_CODE.replace_all(&text, "").into_owned();
    // Inline code `...` — keep the inner text but drop the backticks.
    text = INLINE_CODE.replace_all(&text, "$1").into_owned();
    // Headings markers.
    text = HEADING_MARKERS.replace_all(&text, "").into_owned();
    // Links [text](url) -> text.
    text = LINKS.replace_all(&text, "$1").into_owned();
    // Images ![alt](url) -> alt.
    text = IMAGES.replace_all(&text, "$1").into_owned();
    // Blog inline media tokens: @[img:name], @(_300)[img:2].
    text = MEDIA_TOKENS.replace_all(&text, "").into_owned();
    // App directive blocks used by the blog (:::app ... fences) — keep text
    // after the directive keyword, drop the fence and the token.
    text = APP_DIRECTIVES.replace_all(&text, "").into_owned();

    text.split_whitespace()
        .filter(|word| word.chars().any(|ch| ch.is_alphanumeric()))
        .count()
}

/// Estimated reading time in minutes (rounded up, minimum 1).
pub fn estimate_reading_time_minutes(markdown: &str) -> i64 {
    let words = count_words(markdown);
    if words == 0 {
        return 0;
    }
    let minutes = (words as f64 / WORDS_PER_MINUTE).ceil() as i64;
    minutes.max(1)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn counts_plain_text() {
        assert_eq!(count_words("hello world foo bar"), 4);
    }

    #[test]
    fn ignores_media_tokens() {
        assert_eq!(
            count_words("some words @[img:hero] more @(_300)[img:2] text"),
            4
        );
    }

    #[test]
    fn ignores_code_blocks() {
        assert_eq!(
            count_words("intro\n```\nconst x = 1; @[img:nope]\n```\noutro"),
            2
        );
    }

    #[test]
    fn reads_headings_as_words() {
        assert_eq!(count_words("# Hello World"), 2);
    }

    #[test]
    fn strips_link_syntax() {
        assert_eq!(count_words("visit [my blog](https://x.dev) today"), 4);
    }

    #[test]
    fn empty_is_zero() {
        assert_eq!(estimate_reading_time_minutes(""), 0);
        assert_eq!(estimate_reading_time_minutes("   \n ## "), 0);
    }

    #[test]
    fn returns_at_least_one_for_text() {
        assert_eq!(estimate_reading_time_minutes("hello"), 1);
    }
}
/// Estimated reading time for a post's markdown body.
///
/// Strips markdown structure (code fences, inline code, headings, links,
/// emphasis) and the blog's inline media/app tokens before counting words, so
/// syntax like `@[img:name]` or `:::app glb-demo` does not inflate the count.
pub const WORDS_PER_MINUTE: f64 = 150.0;

/// Strip markdown + blog-inline tokens, leaving roughly the readable words.
pub fn count_words(markdown: &str) -> usize {
    let mut text = markdown.to_string();

    // Fenced code blocks (``` ... ```) — drop entirely.
    remove_pattern(&mut text, r"(?s)```.*?```");
    // Inline code `...` — keep the inner text but drop the backticks.
    replace_pattern(&mut text, r"`([^`]+)`", "$1");
    // Headings markers.
    remove_pattern(&mut text, r"(?m)^#{1,6}\s+");
    // Links [text](url) -> text.
    replace_pattern(&mut text, r"\[([^\]]+)\]\([^)]*\)", "$1");
    // Images ![alt](url) -> alt.
    replace_pattern(&mut text, r"!\[([^\]]*)\]\([^)]*\)", "$1");
    // Blog inline media tokens: @[img:name], @(_300)[img:2].
    remove_pattern(&mut text, r"@(?:\([^)]*\))?\[[^\]]*\]");
    // App directive blocks used by the blog (:::app ... fences) — keep text
    // after the directive keyword, drop the fence and the token.
    remove_pattern(&mut text, r"(?m)^:::\w+\s+(.*)$");

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

fn remove_pattern(text: &mut String, pattern: &str) {
    let re = regex::Regex::new(pattern).expect("valid regex");
    *text = re.replace_all(text, "").as_ref().to_string();
}

fn replace_pattern(text: &mut String, pattern: &str, replacement: &str) {
    let re = regex::Regex::new(pattern).expect("valid regex");
    *text = re.replace_all(text, replacement).as_ref().to_string();
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
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

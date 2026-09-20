// Media short-name extraction shared by the post, project, and game
// handlers: inline `@[..]` and `:::app lottie` references are collected so
// media usage can be tracked, then replaced with numeric indexes.
use std::collections::HashMap;

use once_cell::sync::Lazy;
use regex::Regex;

use crate::helper::string::replace_range_unicode;

#[derive(Debug)]
pub struct ShortNameExtraction {
    pub short_name: String,
    pub start: usize,
}

// These run on every post/project/game create, update, and publish.
static MEDIA_NAME_REGEXES: Lazy<[Regex; 2]> = Lazy::new(|| {
    [
        Regex::new(r"@(?:\([\d_]+\))?\[[\w-]+:([^\]]+)\]").unwrap(),
        Regex::new(r":::app\s+lottie\s+([^\s]+)").unwrap(),
    ]
});

pub fn extract_media_short_names(content: &str) -> Vec<ShortNameExtraction> {
    let mut extraction = Vec::<ShortNameExtraction>::new();
    for reg in MEDIA_NAME_REGEXES.iter() {
        for cap in reg.captures_iter(content) {
            if let Some(matched) = cap.get(1) {
                extraction.push(ShortNameExtraction {
                    short_name: matched.as_str().to_string(),
                    start: matched.start(),
                });
            }
        }
    }
    extraction.sort_by_key(|k| std::cmp::Reverse(k.start));
    extraction
}

pub fn replace_media_short_names(content: &mut String, usage: &mut HashMap<String, i64>) {
    for data in extract_media_short_names(content) {
        let len = usage.len();
        let index = usage
            .entry(data.short_name.clone())
            .or_insert_with(|| len as i64)
            .to_string();
        replace_range_unicode(content, data.start, data.short_name.len(), index);
    }
}

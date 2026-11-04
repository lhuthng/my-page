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

// Unlike inline tokens and lottie, a glb embed keeps the short name in the
// body: `App.svelte` resolves it through the short-name half of the render
// dictionary. It still has to be registered as a usage, or the medium is not
// linked to the post and a replacement can delete it underneath the embed.
static GLB_APP_REGEX: Lazy<Regex> =
    Lazy::new(|| Regex::new(r":::app\s+glb-demo\s+([^\s]+)").unwrap());

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

/// Register glb embeds in the usage map while leaving their short names in
/// place. Call after `replace_media_short_names`; the map deduplicates by short
/// name, so media referenced by both an inline token and a glb embed keeps one
/// code.
pub fn collect_unencoded_media_short_names(content: &str, usage: &mut HashMap<String, i64>) {
    for cap in GLB_APP_REGEX.captures_iter(content) {
        if let Some(short_name) = cap.get(1) {
            let len = usage.len();
            usage
                .entry(short_name.as_str().to_string())
                .or_insert_with(|| len as i64);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn glb_embeds_are_registered_without_being_rewritten() {
        let mut content = ":::app glb-demo robot".to_string();
        let mut usage = HashMap::new();

        replace_media_short_names(&mut content, &mut usage);
        collect_unencoded_media_short_names(&content, &mut usage);

        assert_eq!(content, ":::app glb-demo robot");
        assert_eq!(usage.get("robot"), Some(&0));
    }

    #[test]
    fn a_medium_used_inline_and_as_a_glb_embed_keeps_one_code() {
        let mut content = "@[img:robot] then :::app glb-demo robot".to_string();
        let mut usage = HashMap::new();

        replace_media_short_names(&mut content, &mut usage);
        collect_unencoded_media_short_names(&content, &mut usage);

        assert_eq!(content, "@[img:0] then :::app glb-demo robot");
        assert_eq!(usage.len(), 1);
        assert_eq!(usage.get("robot"), Some(&0));
    }
}

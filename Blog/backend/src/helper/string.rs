pub fn replace_range_unicode(s: &mut String, start: usize, size: usize, insert: String) {
    let start_byte = s
        .char_indices()
        .nth(start)
        .map(|(i, _)| i)
        .unwrap_or_else(|| s.len());

    let end_char = start + size;

    let end_byte = s
        .char_indices()
        .nth(end_char)
        .map(|(i, _)| i)
        .unwrap_or_else(|| s.len());

    s.replace_range(start_byte..end_byte, insert.as_str());
}

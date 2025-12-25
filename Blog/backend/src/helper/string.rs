pub fn replace_range_unicode(s: &mut String, start: usize, size: usize, insert: String) {
    s.replace_range(start..(start + size), insert.as_str());
}

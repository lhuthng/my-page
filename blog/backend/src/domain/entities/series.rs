#[derive(Debug, Clone)]
pub struct Series {
    pub id: i64,
    pub title: String,
    pub slug: String,
    pub description: String,
    pub url: Option<String>,
}

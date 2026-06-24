#[derive(sqlx::FromRow)]
pub struct UserRow {
    pub id: i64,
    pub username: String,
    pub email: String,
    pub role: String,
    pub created_at: String,
    pub display_name: String,
    pub bio: String,
    pub avatar_url: Option<String>,
}

#[derive(sqlx::FromRow)]
pub struct GqlPostRow {
    pub id: i64,
    pub title: String,
    pub slug: String,
    pub status: String,
    pub view_count: i64,
    pub is_featured: i64,
    pub published_at: Option<String>,
    pub created_at: String,
    pub updated_at: String,
    pub excerpt: Option<String>,
    pub author_slug: Option<String>,
    pub author_name: Option<String>,
    pub series_title: Option<String>,
}

#[derive(sqlx::FromRow)]
pub struct CommentRow {
    pub id: i64,
    pub content: String,
    pub parent_id: Option<i64>,
    pub is_deleted: i64,
    pub created_at: String,
    pub post_title: String,
    pub post_slug: String,
    pub author_username: Option<String>,
    pub author_name: Option<String>,
}

#[derive(sqlx::FromRow)]
pub struct MediaRow {
    pub id: i64,
    pub short_name: String,
    pub file_name: String,
    pub file_type: String,
    pub url: String,
    pub size: i64,
    pub description: Option<String>,
    pub use_count: i64,
    pub created_at: String,
    pub uploader_name: Option<String>,
}

#[derive(sqlx::FromRow)]
pub struct SeriesRow {
    pub id: i64,
    pub title: String,
    pub slug: String,
    pub description: Option<String>,
    pub cover_url: Option<String>,
    pub owner_username: Option<String>,
    pub created_at: String,
    pub post_count: i64,
}

#[derive(sqlx::FromRow)]
pub struct TagRow {
    pub id: i64,
    pub name: String,
    pub slug: String,
    pub post_count: i64,
}

#[derive(sqlx::FromRow)]
pub struct CategoryRow {
    pub id: i64,
    pub name: String,
    pub slug: String,
    pub description: Option<String>,
    pub post_count: i64,
}

#[derive(sqlx::FromRow)]
pub struct DashboardPostRow {
    pub post_id: i64,
    pub title: String,
    pub slug: String,
    pub excerpt: String,
    pub author_slug: String,
    pub author_name: String,
    pub url: Option<String>,
    pub cover_media_type: Option<String>,
    pub status: String,
    pub views: i64,
    pub likes: i64,
    pub comments_count: i64,
}

#[derive(sqlx::FromRow)]
pub struct DashboardProjectRow {
    pub project_id: i64,
    pub post_id: i64,
    pub title: String,
    pub slug: String,
    pub excerpt: String,
    pub author_slug: String,
    pub author_name: String,
    pub status: String,
    pub url: Option<String>,
    pub cover_media_type: Option<String>,
    pub demo_type: String,
    pub views: i64,
    pub likes: i64,
    pub comments_count: i64,
}

#[derive(sqlx::FromRow)]
pub struct TagJoinRow {
    pub post_id: i64,
    pub tag_name: String,
    pub tag_slug: String,
}

#[derive(sqlx::FromRow)]
pub struct SeriesPostRow {
    pub post_id: i64,
    pub title: String,
    pub slug: String,
    pub status: String,
    pub number: i64,
    pub url: Option<String>,
}

#[derive(sqlx::FromRow)]
pub struct UserInfoRow {
    pub username: String,
    pub display_name: String,
    pub role: String,
    pub avatar_url: Option<String>,
    pub created_at: String,
}

#[derive(sqlx::FromRow)]
pub struct RoleCountRow {
    pub role: String,
    pub count: i64,
}

#[derive(sqlx::FromRow)]
pub struct GrowthDayRow {
    pub date: String,
    pub count: i64,
}

#[derive(sqlx::FromRow)]
pub struct PostDetailRow {
    pub id: i64,
    pub title: String,
    pub slug: String,
    pub excerpt: String,
    pub content: String,
    pub draft: String,
    pub status: String,
    pub is_featured: i64,
    pub view_count: i64,
    pub published_at: Option<String>,
    pub created_at: String,
    pub updated_at: String,
    pub author_slug: String,
    pub author_name: String,
    pub cover_url: Option<String>,
    pub cover_media_type: Option<String>,
    pub series_title: Option<String>,
    pub series_slug: Option<String>,
    pub og_image_seconds: i64,
}

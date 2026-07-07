use async_graphql::SimpleObject;

#[derive(SimpleObject)]
pub struct GqlUser {
    pub id: i64,
    pub username: String,
    pub email: String,
    pub role: String,
    pub display_name: String,
    pub bio: String,
    pub avatar_url: Option<String>,
    pub created_at: String,
}

#[derive(SimpleObject)]
pub struct UserConnection {
    pub items: Vec<GqlUser>,
    pub total: i64,
}

#[derive(SimpleObject)]
pub struct GqlPost {
    pub id: i64,
    pub title: String,
    pub slug: String,
    pub status: String,
    pub author_name: Option<String>,
    pub author_slug: Option<String>,
    pub series_title: Option<String>,
    pub view_count: i64,
    pub is_featured: bool,
    pub published_at: Option<String>,
    pub created_at: String,
    pub updated_at: String,
    pub excerpt: Option<String>,
}

#[derive(SimpleObject)]
pub struct PostConnection {
    pub items: Vec<GqlPost>,
    pub total: i64,
}

#[derive(SimpleObject)]
pub struct GqlComment {
    pub id: i64,
    pub content: String,
    pub post_title: String,
    pub post_slug: String,
    pub author_name: Option<String>,
    pub author_username: Option<String>,
    pub parent_id: Option<i64>,
    pub is_deleted: bool,
    pub created_at: String,
}

#[derive(SimpleObject)]
pub struct CommentConnection {
    pub items: Vec<GqlComment>,
    pub total: i64,
}

#[derive(SimpleObject)]
pub struct GqlMedia {
    pub id: i64,
    pub short_name: String,
    pub file_name: String,
    pub file_type: String,
    pub url: String,
    pub size: i64,
    pub description: Option<String>,
    pub uploader_name: Option<String>,
    pub use_count: i64,
    pub created_at: String,
}

#[derive(SimpleObject)]
pub struct MediaConnection {
    pub items: Vec<GqlMedia>,
    pub total: i64,
}

#[derive(SimpleObject)]
pub struct GqlSeries {
    pub id: i64,
    pub title: String,
    pub slug: String,
    pub description: Option<String>,
    pub cover_url: Option<String>,
    pub owner_username: Option<String>,
    pub post_count: i64,
    pub created_at: String,
}

#[derive(SimpleObject)]
pub struct SeriesConnection {
    pub items: Vec<GqlSeries>,
    pub total: i64,
}

#[derive(SimpleObject)]
pub struct GqlTag {
    pub id: i64,
    pub name: String,
    pub slug: String,
    pub description: Option<String>,
    pub post_count: i64,
}

#[derive(SimpleObject)]
pub struct TagConnection {
    pub items: Vec<GqlTag>,
    pub total: i64,
}

#[derive(SimpleObject)]
pub struct GqlCategory {
    pub id: i64,
    pub name: String,
    pub slug: String,
    pub description: Option<String>,
    pub post_count: i64,
}

#[derive(SimpleObject)]
pub struct CategoryConnection {
    pub items: Vec<GqlCategory>,
    pub total: i64,
}

#[derive(SimpleObject)]
pub struct DbStats {
    pub total_users: i64,
    pub total_posts: i64,
    pub total_comments: i64,
    pub total_media: i64,
    pub total_series: i64,
    pub total_tags: i64,
    pub total_categories: i64,
}

#[derive(SimpleObject)]
pub struct GqlDashboardPost {
    pub id: i64,
    pub title: String,
    pub slug: String,
    pub excerpt: String,
    pub author_name: String,
    pub author_slug: String,
    pub tag_names: Vec<String>,
    pub tag_slugs: Vec<String>,
    pub status: String,
    pub cover_url: Option<String>,
    pub cover_media_type: Option<String>,
    pub views: i64,
    pub likes: i64,
    pub comments_count: i64,
}

#[derive(SimpleObject)]
pub struct DashboardPostConnection {
    pub items: Vec<GqlDashboardPost>,
    pub total: i64,
}

#[derive(SimpleObject)]
pub struct GqlDashboardProject {
    pub id: i64,
    pub post_id: i64,
    pub title: String,
    pub slug: String,
    pub excerpt: String,
    pub author_name: String,
    pub author_slug: String,
    pub tag_names: Vec<String>,
    pub tag_slugs: Vec<String>,
    pub status: String,
    pub cover_url: Option<String>,
    pub cover_media_type: Option<String>,
    pub demo_type: String,
    pub views: i64,
    pub likes: i64,
    pub comments_count: i64,
}

#[derive(SimpleObject)]
pub struct ProjectConnection {
    pub items: Vec<GqlDashboardProject>,
    pub total: i64,
}

#[derive(SimpleObject)]
pub struct GqlDashboardUser {
    pub username: String,
    pub display_name: String,
    pub role: String,
    pub avatar_url: Option<String>,
    pub created_at: String,
}

#[derive(SimpleObject)]
pub struct GqlRoleCounts {
    pub admin: i64,
    pub moderator: i64,
    pub user: i64,
}

#[derive(SimpleObject)]
pub struct GqlGrowthPoint {
    pub date: String,
    pub new_posts: i64,
    pub new_users: i64,
}

#[derive(SimpleObject)]
pub struct GqlDashboardOverview {
    pub total_published: i64,
    pub total_drafts: i64,
    pub total_users: i64,
    pub total_comments: i64,
    pub top_posts_by_views: Vec<GqlDashboardPost>,
    pub top_posts_by_likes: Vec<GqlDashboardPost>,
    pub top_posts_by_comments: Vec<GqlDashboardPost>,
    pub recent_posts: Vec<GqlDashboardPost>,
    pub recent_users: Vec<GqlDashboardUser>,
    pub role_counts: GqlRoleCounts,
    pub growth: Vec<GqlGrowthPoint>,
}

#[derive(SimpleObject)]
pub struct GqlSeriesPost {
    pub post_id: i64,
    pub title: String,
    pub slug: String,
    pub cover_url: Option<String>,
    pub status: String,
    pub number: i64,
}

#[derive(SimpleObject)]
pub struct GqlPostDetail {
    pub id: i64,
    pub title: String,
    pub slug: String,
    pub excerpt: String,
    pub content: String,
    pub draft: String,
    pub status: String,
    pub is_featured: bool,
    pub author_name: String,
    pub author_slug: String,
    pub tag_names: Vec<String>,
    pub tag_slugs: Vec<String>,
    pub cover_url: Option<String>,
    pub cover_media_type: Option<String>,
    pub series_title: Option<String>,
    pub series_slug: Option<String>,
    pub views: i64,
    pub likes: i64,
    pub comments_count: i64,
    pub published_at: Option<String>,
    pub created_at: String,
    pub updated_at: String,
    pub medium_urls: Vec<String>,
    pub medium_short_names: Vec<String>,
    pub og_image_seconds: i64,
}

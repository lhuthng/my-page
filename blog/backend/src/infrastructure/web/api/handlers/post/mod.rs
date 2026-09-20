// Post feature handlers, split by responsibility. This module is the public
// surface: route handlers for the router plus the wire types their
// signatures use. Internal helpers stay inside their sub-modules.
mod comments;
mod dto;
mod media;
mod publish;
mod read;
mod response;
mod trash;
mod update;
mod write;

pub use dto::{
    CheckQuery, CheckResponse, CommentsQuery, DeletePostQuery, GetFeaturedPostsBody,
    GetFeaturedPostsQuery, GetTagPostsQuery, NewCommentBody, PostData, PostPatchData,
    SearchPostQuery, SearchTagsQuery, SetPostFeaturedBody, SetRelatedPostsBody,
};
pub use response::{
    CategoryResponse, Comment, CommentsResponse, GetCategoriesResponse, GetFeaturedPostsResponse,
    GetPostDetailsResponse, GetRelatedPostsResponse, NewCommentResponse, Post, PostResponse,
    PostSeriesResponse, SearchPostResponse, SearchPostResult, SearchTagResult, SearchTagsResponse,
    TagPostsResponse, UpdatePostResponse,
};
pub use read::{
    check_post, get_categories, get_featured_posts, get_latest_posts, get_post_by_slug,
    get_post_details, get_posts_by_tag, get_related_posts, search, search_tags,
};
pub use write::new_post;
pub use update::{change_cover, set_related_posts, update_post};
pub use publish::{publish, set_post_featured};
pub use trash::{delete_post, purge_post_now, restore_post};
pub use comments::{get_comments, new_comment};
pub use media::{push_like, push_view};

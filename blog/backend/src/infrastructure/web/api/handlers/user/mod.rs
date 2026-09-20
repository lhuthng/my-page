// User feature handlers, split by responsibility. This module is the
// public surface: route handlers for the router plus the wire types their
// signatures use.
mod avatar;
mod dto;
mod profile;

pub use avatar::change_avatar;
pub use dto::{
    ChangeDetailsBody, CheckModResponse, GetLatestCommentsQuery, GetLatestCommentsResponse,
    GetPostsQuery, GetPostsResponse, GetUserResponse, MeResponse, SearchUserQuery,
    SearchUserResponse,
};
pub use profile::{change_details, check_mod, get_latest_comments, get_posts, get_user, me, search};

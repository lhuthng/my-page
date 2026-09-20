// Media feature handlers, split by responsibility. This module is the
// public surface: route handlers for the router plus the wire types their
// signatures use.
mod aliases;
mod dto;
mod manage;
mod serving;
mod upload;

pub use aliases::{add_alias, change_alias, delete_alias, get_aliases};
pub use dto::{
    AddAliasPayload, ChangeAliasPayload, ChangeDetailsPayload, GetAliasesResponse,
    GetLinkResponse, GetMediaDetailsResponse, MediaQuery, SearchResponse,
};
pub use manage::{change_details, get_details};
pub use serving::{get_link, get_media, search};
pub use upload::upload;

// Audiobook feature handlers, split by responsibility. This module is the
// public surface: route handlers for the router plus the wire types their
// signatures use.
mod dto;
mod read;
mod response;
mod shared;
mod tracks;
mod write;

pub use dto::{
    ChangeStatusPayload, ListQuery, ReorderTracksPayload, SlugQuery, UpdateAudiobookPayload,
    UpdateTrackPayload,
};
pub use read::{
    check_slug, get_audiobook_details, get_audiobooks, get_public_audiobook,
    get_public_audiobooks, list_tags,
};
pub use response::{
    AudiobookCreatedResponse, AudiobookDetailsResponse, AudiobookListResponse,
    AudiobookSummaryResponse, AudiobookTagsResponse, SlugAvailabilityResponse,
    TrackCreatedResponse,
};
pub use tracks::{add_track, remove_track, reorder_tracks, update_track};
pub use write::{change_cover, change_status, delete_audiobook, new_audiobook, update_audiobook};

// Engagement counters: view and like beacons.
use std::sync::Arc;

use axum::{
    extract::{Path, State},
    response::IntoResponse,
};

use crate::{
    application::{
        commands::post::{PushNewLikeCommand, PushNewViewCommand},
        services::post::PostService,
    },
    domain::errors::post::PostError,
    infrastructure::web::server::AppState,
};

pub async fn push_view(
    State(state): State<Arc<AppState>>,
    Path(post_id_str): Path<String>,
) -> Result<impl IntoResponse, PostError> {
    let post_id = post_id_str.parse::<i64>().unwrap();

    state
        .post_service
        .push_new_view(PushNewViewCommand { post_id })
        .await?;
    Ok(())
}

pub async fn push_like(
    State(state): State<Arc<AppState>>,
    Path(post_id_str): Path<String>,
) -> Result<impl IntoResponse, PostError> {
    let post_id = post_id_str.parse::<i64>().unwrap();

    state
        .post_service
        .push_new_like(PushNewLikeCommand { post_id })
        .await?;
    Ok(())
}

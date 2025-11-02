use std::sync::Arc;

use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::IntoResponse,
};

use crate::infrastructure::web::server::AppState;

pub async fn to_uppercase(
    State(state): State<Arc<AppState>>,
    Path(text): Path<String>,
) -> impl IntoResponse {
    let processed_text = text.to_uppercase();

    let response_message = format!("API response for: {}", processed_text);

    (StatusCode::OK, response_message)
}

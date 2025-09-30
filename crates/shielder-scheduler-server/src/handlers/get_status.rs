use std::sync::Arc;

use axum::{
    extract::{Path, State},
    response::IntoResponse,
    Json,
};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use tracing::{error, info, instrument};

use crate::{
    storage::{RequestStatus, StorageInterface},
    AppState,
};

#[derive(Debug, Serialize, Deserialize)]
pub struct GetStatusResponse {
    pub last_note_index: String,
    pub status: RequestStatus,
    pub created_at: DateTime<Utc>,
    pub processed_at: Option<DateTime<Utc>>,
    pub relay_after: DateTime<Utc>,
}

#[instrument(level = "info", skip_all)]
pub async fn get_status<Storage: StorageInterface>(
    State(state): State<Arc<AppState<Storage>>>,
    Path(last_note_index): Path<String>,
) -> impl IntoResponse {
    info!(
        "Received get status request for last_note_index: {}",
        last_note_index
    );

    match state
        .storage
        .get_request_by_last_note_index(&last_note_index)
        .await
    {
        Ok(Some(res)) => {
            info!(
                "Found request with status: {:?} for last_note_index: {}",
                res.status, last_note_index
            );
            (
                axum::http::StatusCode::OK,
                Json(GetStatusResponse {
                    last_note_index: res.last_note_index.to_string(),
                    status: res.status.clone(),
                    created_at: res.created_at,
                    processed_at: res.processed_at,
                    relay_after: res.relay_after,
                }),
            )
                .into_response()
        }
        Ok(None) => {
            info!("No request found for last_note_index: {}", last_note_index);
            (
                axum::http::StatusCode::NOT_FOUND,
                Json(serde_json::json!({
                    "error": "Request not found",
                    "last_note_index": last_note_index
                })),
            )
                .into_response()
        }
        Err(e) => {
            error!(
                "Failed to get request status for last_note_index {}: {:?}",
                last_note_index, e
            );
            (
                axum::http::StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({
                    "error": "Failed to retrieve request status"
                })),
            )
                .into_response()
        }
    }
}

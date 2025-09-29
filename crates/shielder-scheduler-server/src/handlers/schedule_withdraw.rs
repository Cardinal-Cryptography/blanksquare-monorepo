use std::sync::Arc;

use alloy_primitives::{Address, U256};
use axum::{extract::State, response::IntoResponse, Json};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use shielder_scheduler_common::protocol::EncryptionEnvelope;
use tracing::{error, info, instrument};

use crate::{
    app_state::AppState,
    credentials_provider::CredentialsProvider,
    storage::{ScheduledRequest, StorageProvider},
};

/// When requesting a withdraw schedule, user sends this struct as a JSON
#[derive(Debug, Deserialize, Serialize)]
pub struct ScheduleWithdrawRequest {
    pub encryption_envelope: EncryptionEnvelope,
    /// Index of the last leaf in the Merkle tree containing the account's note.
    /// Necessary to get the merkle path from this leaf to the current root.
    pub last_note_index: U256,
    /// Timestamp after which the relay is allowed (Unix timestamp in seconds).
    pub relay_after: i64,
    /// Pocket money amount for the withdrawal.
    pub pocket_money: U256,
    /// Token address for the withdrawal.
    pub token_address: Address,
}

#[derive(Debug, Serialize)]
pub struct ScheduleWithdrawResponse {
    pub request_id: String,
    pub message: String,
}

#[instrument(level = "info", skip_all)]
pub async fn schedule_withdraw<Storage: StorageProvider, Credentials: CredentialsProvider>(
    State(state): State<Arc<AppState<Storage, Credentials>>>,
    Json(schedule_withdraw_request): Json<ScheduleWithdrawRequest>,
) -> impl IntoResponse {
    info!(
        "Received schedule withdraw request - last_note_index: {}, pocket_money: {}, token_address: {}, relay_after: {}",
        schedule_withdraw_request.last_note_index,
        schedule_withdraw_request.pocket_money,
        schedule_withdraw_request.token_address,
        schedule_withdraw_request.relay_after
    );

    let relay_after = match DateTime::from_timestamp(schedule_withdraw_request.relay_after, 0) {
        Some(dt) => dt,
        None => {
            error!(
                "Invalid relay_after timestamp: {}",
                schedule_withdraw_request.relay_after
            );
            return (
                axum::http::StatusCode::BAD_REQUEST,
                Json(serde_json::json!({
                    "error": "Invalid relay_after timestamp"
                })),
            )
                .into_response();
        }
    };

    if relay_after <= Utc::now() {
        return (
            axum::http::StatusCode::BAD_REQUEST,
            Json(serde_json::json!({
                "error": "relay_after must be in the future"
            })),
        )
            .into_response();
    }

    let request = ScheduledRequest::new(
        schedule_withdraw_request.encryption_envelope,
        schedule_withdraw_request.last_note_index,
        schedule_withdraw_request.pocket_money,
        schedule_withdraw_request.token_address,
        relay_after,
    );

    let request_last_note_index = request.last_note_index;

    match state.storage.insert_scheduled_request(request).await {
        Ok(_) => {
            info!(
                "Successfully scheduled withdraw request with last_note_index: {}",
                request_last_note_index
            );
            (
                axum::http::StatusCode::CREATED,
                Json(ScheduleWithdrawResponse {
                    request_id: request_last_note_index.to_string(),
                    message: format!(
                        "Withdraw request scheduled successfully. Last note index: {}",
                        request_last_note_index
                    ),
                }),
            )
                .into_response()
        }
        Err(e) => {
            error!("Failed to insert scheduled request: {:?}", e);
            (
                axum::http::StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({
                    "error": "Failed to schedule withdraw request"
                })),
            )
                .into_response()
        }
    }
}

use std::sync::Arc;

use alloy_primitives::U256;
use axum::{extract::State, response::IntoResponse, Json};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use shielder_scheduler_common::base64_serialization;
use tracing::{error, info, instrument};

use crate::{db::insert_scheduled_request, AppState};

/// When requesting a withdraw schedule, user sends this struct as a JSON
#[derive(Debug, Deserialize, Serialize)]
pub struct ScheduleWithdrawRequest {
    /// For `payload` schema, see [`shielder_scheduler_common::protocol::Payload`].
    #[serde(with = "base64_serialization")]
    payload: Vec<u8>,

    // Unecrypted data useful for basic checks.
    // It should be consistent with the data in `payload`.
    
    /// Index of the last leaf in the Merkle tree containing the account's note.
    /// Necessary to get the merkle path from this leaf to the current root.
    last_note_index: U256,
    /// Maximum fee that the relayer can charge for this transaction.
    max_relayer_fee: U256,
    /// Timestamp after which the relay is allowed (Unix timestamp in seconds).
    relay_after: i64,
}

#[derive(Debug, Serialize)]
pub struct ScheduleWithdrawResponse {
    pub request_id: i64,
    pub message: String,
}

#[instrument(level = "info", skip_all)]
pub async fn schedule_withdraw(
    State(state): State<Arc<AppState>>,
    Json(schedule_withdraw_request): Json<ScheduleWithdrawRequest>,
) -> impl IntoResponse {
    info!(
        "Received schedule withdraw request - last_note_index: {}, max_relayer_fee: {}, relay_after: {}",
        schedule_withdraw_request.last_note_index,
        schedule_withdraw_request.max_relayer_fee,
        schedule_withdraw_request.relay_after
    );

    // Convert Unix timestamp to DateTime<Utc>
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

    // Basic validation
    // TBD: more comprehensive validation
    if schedule_withdraw_request.payload.is_empty() {
        return (
            axum::http::StatusCode::BAD_REQUEST,
            Json(serde_json::json!({
                "error": "Payload cannot be empty"
            })),
        )
            .into_response();
    }

    if relay_after <= Utc::now() {
        return (
            axum::http::StatusCode::BAD_REQUEST,
            Json(serde_json::json!({
                "error": "relay_after must be in the future"
            })),
        )
            .into_response();
    }

    // Insert the request into the database
    match insert_scheduled_request(
        &state.db_pool,
        &schedule_withdraw_request.payload,
        schedule_withdraw_request.last_note_index,
        schedule_withdraw_request.max_relayer_fee,
        relay_after,
    )
    .await
    {
        Ok(request_id) => {
            info!(
                "Successfully scheduled withdraw request with ID: {}",
                request_id
            );
            (
                axum::http::StatusCode::CREATED,
                Json(ScheduleWithdrawResponse {
                    request_id,
                    message: format!(
                        "Withdraw request scheduled successfully. Request ID: {}",
                        request_id
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

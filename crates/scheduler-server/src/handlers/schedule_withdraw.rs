use std::sync::Arc;

use alloy_primitives::U256;
use axum::{extract::State, response::IntoResponse, Json};
use scheduler_common::base64_serialization;
use serde::{Deserialize, Serialize};

use crate::AppState;

/// When requesting a withdraw schedule, user sends this struct as a JSON
#[derive(Debug, Deserialize, Serialize)]
pub struct ScheduleWithdrawRequest {
    /// For `payload` schema, see [`scheduler_common::protocol::Payload`].
    #[serde(with = "base64_serialization")]
    payload: Vec<u8>,

    // Unecrypted data useful for basic checks.
    // It should be consistent with the data in `payload`.
    // TBD what exactly is needed
    /// Index of the last leaf in the Merkle tree containing the account's note.
    /// Necessary to get the merkle path from this leaf to the current root.
    last_note_index: U256,
    /// Maximum fee that the relayer can charge for this transaction.
    max_relayer_fee: U256,
    /// Timestamp after which the relay is allowed.
    relay_after: U256,
}

pub async fn schedule_withdraw(
    State(_state): State<Arc<AppState>>,
    Json(_schedule_withdraw_request): Json<ScheduleWithdrawRequest>,
) -> impl IntoResponse {
    (
        axum::http::StatusCode::NOT_IMPLEMENTED,
        "Not implemented yet",
    )
}

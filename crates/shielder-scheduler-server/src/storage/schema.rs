use alloy_primitives::{Address, U256};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use shielder_scheduler_common::protocol::EncryptionEnvelope;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScheduledRequest {
    pub id: String,
    pub encryption_envelope: EncryptionEnvelope,
    pub last_note_index: U256,
    pub pocket_money: U256,
    pub token_address: Address,
    pub relay_after: DateTime<Utc>,
    pub status: RequestStatus,
    pub created_at: DateTime<Utc>,
    pub retry_count: u8,
    pub processed_at: Option<DateTime<Utc>>,
    pub error_message: Option<String>,
    // (Optional) Time after which the request can be cleared from storage.
    pub prune_after: Option<DateTime<Utc>>,
}

impl ScheduledRequest {
    pub fn new(
        id: String,
        encryption_envelope: EncryptionEnvelope,
        last_note_index: U256,
        pocket_money: U256,
        token_address: Address,
        relay_after: DateTime<Utc>,
    ) -> Self {
        Self {
            id,
            encryption_envelope,
            last_note_index,
            pocket_money,
            token_address,
            relay_after,
            status: RequestStatus::Pending,
            created_at: Utc::now(),
            retry_count: 0,
            processed_at: None,
            error_message: None,
            prune_after: None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum RequestStatus {
    Pending,
    Completed,
    Failed,
}

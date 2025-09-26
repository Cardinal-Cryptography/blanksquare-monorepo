use alloy_primitives::{Address, U256};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use shielder_scheduler_common::protocol::EncryptionEnvelope;
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScheduledRequest {
    pub id: u128,
    pub encryption_envelope: EncryptionEnvelope,
    pub last_note_index: U256,
    pub pocket_money: U256,
    pub token_address: Address,
    pub relay_after: DateTime<Utc>,
    pub status: RequestStatus,
    pub created_at: DateTime<Utc>,
    pub retry_count: i32,
    pub processed_at: Option<DateTime<Utc>>,
    pub error_message: Option<String>,
}

impl ScheduledRequest {
    pub fn new(
        encryption_envelope: EncryptionEnvelope,
        last_note_index: U256,
        pocket_money: U256,
        token_address: Address,
        relay_after: DateTime<Utc>,
    ) -> Self {
        let id = Uuid::new_v4().as_u128();
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
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum RequestStatus {
    Pending,
    Processing,
    Completed,
    Failed,
}

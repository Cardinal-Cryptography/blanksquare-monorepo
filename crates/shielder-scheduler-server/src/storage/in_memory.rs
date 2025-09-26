use std::collections::HashMap;

use alloy_primitives::U256;
use chrono::{DateTime, Utc};

use crate::storage::{RequestStatus, ScheduledRequest, StorageError, StorageInterface};

pub struct InMemoryStorage {
    requests: std::sync::Mutex<HashMap<String, ScheduledRequest>>,
}

impl InMemoryStorage {
    pub fn new() -> Self {
        Self {
            requests: std::sync::Mutex::new(HashMap::new()),
        }
    }
}

impl StorageInterface for InMemoryStorage {
    async fn get_pending_requests(
        &self,
        limit: usize,
    ) -> Result<Vec<ScheduledRequest>, StorageError> {
        let requests = self.requests.lock().unwrap();
        let mut pending_requests: Vec<ScheduledRequest> = requests
            .iter()
            .filter(|(_, req)| {
                (req.status == RequestStatus::Pending) && req.relay_after <= Utc::now()
            })
            .map(|(_, req)| req.clone())
            .collect();
        pending_requests.sort_by(|a, b| a.relay_after.cmp(&b.relay_after));
        Ok(pending_requests.into_iter().take(limit).collect())
    }

    async fn insert_scheduled_request(
        &self,
        request: ScheduledRequest,
    ) -> Result<(), StorageError> {
        let mut requests = self.requests.lock().unwrap();
        let key = request.last_note_index.to_string();
        if requests.contains_key(&key) {
            return Err(StorageError::DuplicateEntry(key));
        }
        requests.insert(key, request);
        Ok(())
    }

    async fn update_request_status(
        &self,
        last_note_index: &str,
        status: RequestStatus,
        processed_at: Option<DateTime<Utc>>,
        error_message: Option<&str>,
    ) -> Result<(), StorageError> {
        let mut requests = self.requests.lock().unwrap();
        if let Some(request) = requests.get_mut(last_note_index) {
            request.status = status;
            request.processed_at = processed_at;
            request.error_message = error_message.map(|s| s.to_string());
            Ok(())
        } else {
            Err(StorageError::NotFound(last_note_index.to_string()))
        }
    }

    async fn update_retry_attempt(
        &self,
        last_note_index: &str,
        new_relay_after: DateTime<Utc>,
        new_retry_count: i32,
        processed_at: Option<DateTime<Utc>>,
        new_error_message: Option<&str>,
    ) -> Result<(), StorageError> {
        let mut requests = self.requests.lock().unwrap();
        if let Some(request) = requests.get_mut(last_note_index) {
            request.relay_after = new_relay_after;
            request.retry_count = new_retry_count;
            request.processed_at = processed_at;
            request.error_message = new_error_message.map(|s| s.to_string());
            Ok(())
        } else {
            Err(StorageError::NotFound(last_note_index.to_string()))
        }
    }

    async fn get_request_by_last_note_index(
        &self,
        last_note_index: &str,
    ) -> Result<Option<ScheduledRequest>, StorageError> {
        let requests = self.requests.lock().unwrap();
        Ok(requests.get(last_note_index).cloned())
    }
}

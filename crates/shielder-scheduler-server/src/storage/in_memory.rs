use std::collections::HashMap;

use chrono::{DateTime, Utc};

use crate::storage::{RequestStatus, ScheduledRequest, StorageError, StorageInterface};

pub struct InMemoryStorage {
    requests: std::sync::Mutex<HashMap<u128, ScheduledRequest>>,
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
                (req.status == RequestStatus::Pending || req.status == RequestStatus::Processing)
                    && req.relay_after <= Utc::now()
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
        if requests.contains_key(&request.id) {
            return Err(StorageError::DuplicateEntry(request.id));
        }
        requests.insert(request.id, request);
        Ok(())
    }

    async fn update_request_status(
        &self,
        id: u128,
        status: RequestStatus,
        error_message: Option<&str>,
    ) -> Result<(), StorageError> {
        let mut requests = self.requests.lock().unwrap();
        if let Some(request) = requests.get_mut(&id) {
            request.status = status;
            request.error_message = error_message.map(|s| s.to_string());
            Ok(())
        } else {
            Err(StorageError::NotFound(id.to_string()))
        }
    }

    async fn update_retry_attempt(
        &self,
        id: u128,
        new_relay_after: DateTime<Utc>,
        new_error_message: Option<&str>,
    ) -> Result<(), StorageError> {
        let mut requests = self.requests.lock().unwrap();
        if let Some(request) = requests.get_mut(&id) {
            request.relay_after = new_relay_after;
            request.error_message = new_error_message.map(|s| s.to_string());
            Ok(())
        } else {
            Err(StorageError::NotFound(id.to_string()))
        }
    }

    async fn get_request_by_last_note_index(
        &self,
        last_note_index: &str,
    ) -> Result<Option<ScheduledRequest>, StorageError> {
        let requests = self.requests.lock().unwrap();
        let matching_requests: Vec<&ScheduledRequest> = requests
            .iter()
            .filter(|(_, req)| req.last_note_index.to_string() == last_note_index)
            .map(|(_, req)| req)
            .collect();

        if let Some(earliest_request) = matching_requests
            .into_iter()
            .min_by(|a, b| a.created_at.cmp(&b.created_at))
        {
            Ok(Some(earliest_request.clone()))
        } else {
            Ok(None)
        }
    }
}

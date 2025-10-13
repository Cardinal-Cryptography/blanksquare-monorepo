pub mod schema;
use chrono::{DateTime, Utc};
pub use schema::{RequestStatus, ScheduledRequest};
pub mod dynamo_db;
pub mod in_memory;

#[derive(thiserror::Error, Debug)]
pub enum StorageError {
    #[error("Not found: {0}")]
    NotFound(String),
    #[error("Duplicate entry. Last note index: {0}")]
    DuplicateEntry(String),
    #[error("Internal error: {0}")]
    Internal(String),
}

pub trait StorageProvider: Send + Sync {
    /// Insert a new scheduled request into storage.
    /// Returns an error if a request with the same ID already exists.
    fn insert_scheduled_request(
        &self,
        request: ScheduledRequest,
    ) -> impl std::future::Future<Output = Result<(), StorageError>> + Send;

    /// Get pending requests that are due for processing.
    /// Limits the number of returned requests to `limit`.
    /// Pending requests are those with status `Pending`
    /// and `relay_after` time less than or equal to the current time.
    fn get_pending_requests(
        &self,
        limit: usize,
    ) -> impl std::future::Future<Output = Result<Vec<ScheduledRequest>, StorageError>> + Send;

    /// Update the status of a scheduled request.
    /// If the request does not exist, returns a `NotFound` error.
    /// Optionally, an error message can be provided.
    fn update_request_status(
        &self,
        id: &str,
        status: RequestStatus,
        processed_at: Option<DateTime<Utc>>,
        error_message: Option<&str>,
    ) -> impl std::future::Future<Output = Result<(), StorageError>> + Send;

    /// Update the retry attempt of a scheduled request.
    /// Sets a new `relay_after` time and optionally updates the error message.
    /// If the request does not exist, returns a `NotFound` error.
    fn update_retry_attempt(
        &self,
        id: &str,
        new_relay_after: DateTime<Utc>,
        new_retry_count: u8,
        processed_at: Option<DateTime<Utc>>,
        new_error_message: Option<&str>,
    ) -> impl std::future::Future<Output = Result<(), StorageError>> + Send;

    /// Retrieve a scheduled request by its ID.
    /// Returns `Ok(None)` if no such request exists.
    fn get_request_by_id(
        &self,
        id: &str,
    ) -> impl std::future::Future<Output = Result<Option<ScheduledRequest>, StorageError>> + Send;
}

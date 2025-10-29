use axum::{
    http::StatusCode,
    response::{IntoResponse, Response as AxumResponse},
};
use shielder_contract::ShielderContractError;
use shielder_scheduler_common::{protocol::TeeError, vsock::VsockError};
use tokio::task::JoinError;
use tracing::error;

use crate::storage::StorageError;

#[derive(thiserror::Error, Debug)]
pub enum SchedulerServerError {
    #[error("Internal I/O error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Task pool error: {0}")]
    TaskPool(#[from] tokio_task_pool::Error),

    #[error("Join handle error: {0}")]
    JoinHandleError(#[from] JoinError),

    #[error("Unexpected response")]
    UnexpectedTeeResponse,

    #[error("TEE Error: {0}")]
    TeeError(#[from] TeeError),

    #[error("Vsock Error: {0}")]
    VsockError(#[from] VsockError),

    #[error("Failed to initialize metrics: {0}")]
    MetricsError(#[from] metrics_exporter_prometheus::BuildError),

    #[error("Failed to parse commandline arguments: {0}")]
    ParseError(String),

    #[error("Storage error: {0}")]
    StorageError(#[from] StorageError),

    #[error("Contract error: {0}")]
    ContractError(#[from] ShielderContractError),

    #[error("Relayer error: {0}")]
    RelayerError(String),

    #[error("Credentials error: {0}")]
    CredentialsError(String),
}

impl IntoResponse for SchedulerServerError {
    fn into_response(self) -> AxumResponse {
        let (status, error_message) = match &self {
            SchedulerServerError::TaskPool(_) => {
                (StatusCode::SERVICE_UNAVAILABLE, "Try again later")
            }
            _ => (StatusCode::INTERNAL_SERVER_ERROR, "Internal server error"),
        };

        error!("Error encountered: {:?}", self);

        (status, error_message).into_response()
    }
}

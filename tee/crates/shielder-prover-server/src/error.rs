use axum::{
    http::StatusCode,
    response::{IntoResponse, Response as AxumResponse},
};
use shielder_prover_common::vsock::VsockError;
use tokio::task::JoinError;
use tracing::error;

#[derive(thiserror::Error, Debug)]
pub enum ShielderProverServerError {
    #[error("Internal I/O error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Task pool error: {0}")]
    TaskPool(#[from] tokio_task_pool::Error),

    #[error("Join handle error: {0}")]
    JoinHandleError(#[from] JoinError),

    #[error("Proving Server error: {0}")]
    ProvingServerError(#[from] VsockError),

    #[error("Failed to initialize metrics: {0}")]
    MetricsError(#[from] metrics_exporter_prometheus::BuildError),
}

impl IntoResponse for ShielderProverServerError {
    fn into_response(self) -> AxumResponse {
        let (status, error_message) = match &self {
            ShielderProverServerError::TaskPool(_) => {
                (StatusCode::SERVICE_UNAVAILABLE, "Try again later")
            }
            _ => (StatusCode::INTERNAL_SERVER_ERROR, "Internal server error"),
        };

        error!("Error encountered: {:?}", self);

        (status, error_message).into_response()
    }
}

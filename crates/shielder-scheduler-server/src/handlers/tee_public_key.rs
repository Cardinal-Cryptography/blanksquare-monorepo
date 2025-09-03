use std::sync::Arc;

use axum::{extract::State, Json};
use shielder_scheduler_common::protocol::{Request, Response};
use tracing::instrument;
use base64::{engine::general_purpose, Engine as _};

use crate::{error::SchedulerServerError, handlers::tee_request, AppState};

#[instrument(level = "info", skip_all)]
pub async fn tee_public_key(
    State(state): State<Arc<AppState>>,
) -> Result<Json<Response>, SchedulerServerError> {
    let tee_task_pool = state.tee_task_pool.clone();
    let state_cloned = state.clone();
    let public_key = general_purpose::STANDARD.decode(state_cloned.options.tee_public_key.clone().as_bytes())
        .map_err(|e| SchedulerServerError::ParseError(format!("Failed to decode public key: {}", e)))?;
    tee_task_pool
        .spawn(async move {
            tee_request(
                state_cloned.clone(),
                Request::TeePublicKey {
                    public_key,
                },
            )
            .await
        })
        .await
        .map_err(SchedulerServerError::TaskPool)?
        .await
        .map_err(SchedulerServerError::JoinHandleError)??
        .map_err(SchedulerServerError::ProvingServerError)
}

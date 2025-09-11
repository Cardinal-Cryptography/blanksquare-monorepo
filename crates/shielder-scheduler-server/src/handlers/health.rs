use std::sync::Arc;

use axum::{extract::State, Json};
use shielder_scheduler_common::protocol::{Request, Response};
use tracing::instrument;

use crate::{error::SchedulerServerError, handlers::tee_request, AppState};

#[instrument(level = "info", skip_all)]
pub async fn health(
    State(state): State<Arc<AppState>>,
) -> Result<Json<Response>, SchedulerServerError> {
    let tee_task_pool = state.tee_task_pool.clone();
    let relayer_rpc_controller = state.relayer_rpc_controller.clone();

    let tee_result = tee_task_pool
        .spawn(async move { tee_request(state, Request::Ping).await })
        .await
        .map_err(SchedulerServerError::TaskPool)?
        .await
        .map_err(SchedulerServerError::JoinHandleError)??
        .map_err(SchedulerServerError::ProvingServerError);

    relayer_rpc_controller.health_check().await?;

    tee_result
}

use std::sync::Arc;

use axum::{extract::State, Json};
use shielder_scheduler_common::protocol::{Request, Response};
use tracing::instrument;

use crate::{error::SchedulerServerError, handlers::request, AppState};

#[instrument(level = "info")]
pub async fn health(
    State(state): State<Arc<AppState>>,
) -> Result<Json<Response>, SchedulerServerError> {
    let task_pool = state.task_pool.clone();

    task_pool
        .spawn(async move { request(state, Request::Ping).await })
        .await
        .map_err(SchedulerServerError::TaskPool)?
        .await
        .map_err(SchedulerServerError::JoinHandleError)??
        .map_err(SchedulerServerError::ProvingServerError)
}

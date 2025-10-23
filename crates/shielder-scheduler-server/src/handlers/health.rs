use std::sync::Arc;

use axum::{extract::State, Json};
use shielder_scheduler_common::protocol::{Request, Response};
use tracing::instrument;

use crate::{
    app_state::AppState, credentials_provider::CredentialsProvider, error::SchedulerServerError,
    metrics::Metrics, storage::StorageProvider,
};

#[instrument(level = "info", skip_all)]
pub async fn health<
    Storage: StorageProvider + 'static,
    Credentials: CredentialsProvider + 'static,
>(
    State(state): State<Arc<AppState<Storage, Credentials>>>,
) -> Result<Json<Response>, SchedulerServerError> {
    Metrics::record_health_request();
    let ((), tee_response) = tokio::try_join!(
        state.relayer_controller.health_check(),
        state.tee_controller.tee_request(Request::Ping),
    )?;
    Ok(Json(tee_response))
}

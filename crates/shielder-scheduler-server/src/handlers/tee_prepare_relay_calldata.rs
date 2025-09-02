use std::sync::Arc;

use alloy_primitives::{Address, U256};
use axum::{extract::State, Json};
use shielder_scheduler_common::protocol::{EncryptionEnvelope, MerklePath, Request, Response};
use tracing::instrument;

use crate::{error::SchedulerServerError, handlers::tee_request, AppState};

#[instrument(level = "info", skip_all)]
pub async fn prepare_relay_calldata(
    State(state): State<Arc<AppState>>,
    encryption_envelope: EncryptionEnvelope,
    relayer_fee: U256,
    relayer_address: Address,
    merkle_path: MerklePath,
) -> Result<Json<Response>, SchedulerServerError> {
    let tee_task_pool = state.tee_task_pool.clone();
    tee_task_pool
        .spawn(async move { tee_request(state, Request::PrepareRelayCalldata {
            encryption_envelope,
            relayer_fee,
            relayer_address,
            merkle_path,
        }).await })
        .await
        .map_err(SchedulerServerError::TaskPool)?
        .await
        .map_err(SchedulerServerError::JoinHandleError)??
        .map_err(SchedulerServerError::ProvingServerError)
}

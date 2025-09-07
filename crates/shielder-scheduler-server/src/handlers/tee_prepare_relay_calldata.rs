use std::sync::Arc;

use alloy_primitives::{Address, U256};
use axum::{extract::State, Json};
use base64::{engine::general_purpose::STANDARD as BASE64, Engine};
use shielder_scheduler_common::protocol::{AwsConfig, EncryptionEnvelope, MerklePath, Request, Response};
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
    let public_key = BASE64
        .decode(state.options.kms_public_key.clone())
        .map_err(|e| {
            SchedulerServerError::ParseError(format!(
                "Failed to decode KMS_PUBLIC_KEY from base64: {e:?}"
            ))
        })?;
    tee_task_pool
        .spawn(async move { tee_request(state.clone(), Request::PrepareRelayCalldata {
            aws_config: AwsConfig {
                public_key,
                kms_key_id: state.options.kms_key_id.clone(),
                aws_region: state.options.aws_region.clone(),
                aws_access_key_id: state.options.aws_access_key_id.clone(),
                aws_secret_access_key: state.options.aws_secret_access_key.clone(),
                aws_session_token: state.options.aws_session_token.clone(),
                kms_encryption_algorithm: state.options.kms_encryption_algorithm.clone(),
            },
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

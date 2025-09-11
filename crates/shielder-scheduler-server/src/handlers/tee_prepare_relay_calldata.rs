use std::sync::Arc;

use alloy_primitives::{Address, U256};
use axum::{extract::State, response::Json};
use base64::prelude::*;
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
    let public_key_bytes = BASE64_STANDARD.decode(&state.options.kms_public_key)
        .map_err(|e| {
            SchedulerServerError::ParseError(format!(
                "Failed to decode base64 KMS_PUBLIC_KEY: {e:?}"
            ))
        })?;

    // Get current AWS credentials
    let aws_credentials = state.aws_credentials.lock().await.clone();
    
    tee_task_pool
        .spawn(async move { tee_request(state.clone(), Request::PrepareRelayCalldata {
            aws_config: AwsConfig {
                public_key: public_key_bytes,
                kms_key_id: state.options.kms_key_id.clone(),
                aws_region: state.options.aws_region.clone(),
                aws_access_key_id: aws_credentials.access_key_id,
                aws_secret_access_key: aws_credentials.secret_access_key,
                aws_session_token: aws_credentials.session_token.unwrap_or_default(),
                kms_encryption_algorithm: "RSAES_OAEP_SHA_256".to_string(),
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

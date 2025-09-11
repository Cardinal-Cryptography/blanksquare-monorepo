use std::sync::Arc;

use axum::{extract::State, Json};
use base64::prelude::*;
use shielder_scheduler_common::protocol::{Request, Response};
use tracing::instrument;

use crate::{error::SchedulerServerError, handlers::tee_request, AppState};

#[instrument(level = "info", skip_all)]
pub async fn tee_public_key(
    State(state): State<Arc<AppState>>,
) -> Result<Json<Response>, SchedulerServerError> {
    let tee_task_pool = state.tee_task_pool.clone();
    let state_cloned = state.clone();
    let public_key_bytes = BASE64_STANDARD.decode(&state_cloned.options.kms_public_key)
        .map_err(|e| {
            SchedulerServerError::ParseError(format!(
                "Failed to decode base64 KMS_PUBLIC_KEY: {e:?}"
            ))
        })?;
    
    // Get current AWS credentials
    let aws_credentials = state_cloned.aws_credentials.lock().await.clone();
    
    tee_task_pool
        .spawn(async move {
            tee_request(
                state_cloned.clone(),
                Request::TeePublicKey {
                    aws_config: shielder_scheduler_common::protocol::AwsConfig {
                        public_key: public_key_bytes,
                        kms_key_id: state_cloned.options.kms_key_id.clone(),
                        aws_region: state_cloned.options.aws_region.clone(),
                        aws_access_key_id: aws_credentials.access_key_id,
                        aws_secret_access_key: aws_credentials.secret_access_key,
                        aws_session_token: aws_credentials.session_token.unwrap_or_default(),
                        kms_encryption_algorithm: "RSAES_OAEP_SHA_256".to_string(),
                    },
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
